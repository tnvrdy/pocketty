use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};

use crate::audio_api::AudioCommand;
use super::effect::{Effect, EffectSpec};
use super::frame::StereoFrame;
use super::sample_buffer::SampleBuffer;
use super::voice::Voice;
use super::SampleId;

const TEMP_BUF_CAP: usize = 8192; // Sort of arbitrarily chosen, but chosen nonetheless
const RECORD_PEAK_THRESHOLD: f32 = 0.02;
const PRE_ROLL_FRAMES: usize = 6615;

enum RecordingState {
    Idle,
    Armed {
        sample_id: SampleId,
        pre_roll: PreRollRing,
    },
    Capturing {
        sample_id: SampleId,
        buffer: Vec<StereoFrame>,
    },
}

struct PreRollRing {
    data: Vec<StereoFrame>,
    write_pos: usize,
    len: usize,
}

impl PreRollRing {
    fn new(capacity: usize) -> Self {
        Self {
            data: vec![StereoFrame::default(); capacity],
            write_pos: 0,
            len: 0,
        }
    }

    fn push(&mut self, frame: StereoFrame) {
        let cap = self.data.len();
        if cap == 0 {
            return;
        }
        self.data[self.write_pos] = frame;
        self.write_pos = (self.write_pos + 1) % cap;
        if self.len < cap {
            self.len += 1;
        }
    }

    fn drain_ordered(&self) -> Vec<StereoFrame> {
        let cap = self.data.len();
        if self.len == 0 || cap == 0 {
            return Vec::new();
        }
        let start = if self.len < cap {
            0
        } else {
            self.write_pos
        };
        let mut out = Vec::with_capacity(self.len);
        for i in 0..self.len {
            out.push(self.data[(start + i) % cap]);
        }
        out
    }
}


// An internal attachment between a voice and the params of our trigger call
struct ActiveVoice {
    voice: Voice,
    sample_id: SampleId,
    effect_chain: Vec<Box<dyn Effect>>,
}

pub struct CompletedRecording {
    pub sample_id: SampleId,
    pub buffer: SampleBuffer,
}

pub struct Engine {
    samples: HashMap<SampleId, SampleBuffer>, // the sample buffers we've registered
    active: Vec<ActiveVoice>,
    temp_buf: Vec<StereoFrame>,

    // Recording
    recording: RecordingState,
    input_rx: Option<Receiver<Vec<StereoFrame>>>,
    completed_tx: Option<Sender<CompletedRecording>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            active: Vec::new(),
            temp_buf: vec![StereoFrame::default(); TEMP_BUF_CAP],
            recording: RecordingState::Idle,
            input_rx: None,
            completed_tx: None,
        }
    }

    pub fn set_input_rx(&mut self, rx: Receiver<Vec<StereoFrame>>) {
        self.input_rx = Some(rx);
    }

    pub fn set_completed_tx(&mut self, tx: Sender<CompletedRecording>) {
        self.completed_tx = Some(tx);
    }

    pub fn handle_cmd(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::RegisterSample { id, buffer } => {
                self.samples.insert(id, buffer);
            }
            AudioCommand::Trigger(params) => {
                if !self.samples.contains_key(&params.sample_id) {
                    return;
                }
                let effect_chain: Vec<Box<dyn Effect>> = params
                    .effect_chain
                    .iter()
                    .map(EffectSpec::to_effect)
                    .collect();
                let voice = Voice::new(
                    params.trim_start,
                    params.length,
                    params.pitch,
                    params.gain,
                    params.reverse,
                    params.stutter_period_samples,
                );
                self.active.push(ActiveVoice {
                    voice,
                    sample_id: params.sample_id,
                    effect_chain,
                });
            }
            AudioCommand::SetPlaybackPosition { sample_id, position } => { // scratch effect
                if let Some(active) = self.active.iter_mut().rev()
                    .find(|a| a.sample_id == sample_id && a.voice.active)
                {
                    active.voice.set_pos(position);
                }
            }
            AudioCommand::StopAllVoices => {
                for active in &mut self.active {
                    active.voice.active = false;
                }
            }
            AudioCommand::StartRecording { sample_id } => {
                self.recording = RecordingState::Armed {
                    sample_id,
                    pre_roll: PreRollRing::new(PRE_ROLL_FRAMES),
                };
            }
            AudioCommand::StopRecording => {
                // Finalise whatever we have and register the sample
                match std::mem::replace(&mut self.recording, RecordingState::Idle) {
                    RecordingState::Capturing { sample_id, buffer } => {
                        let buf = if buffer.is_empty() {
                            SampleBuffer::from_frames(vec![StereoFrame::default()])
                        } else {
                            SampleBuffer::from_frames(buffer)
                        };
                        // Send a copy to the main thread for saving to disk
                        if let Some(tx) = &self.completed_tx {
                            let _ = tx.try_send(CompletedRecording {
                                sample_id,
                                buffer: buf.clone(),
                            });
                        }
                        self.samples.insert(sample_id, buf);
                    }
                    RecordingState::Armed { sample_id, .. } => {
                        // Never reached the threshold — register silence
                        self.samples.insert(
                            sample_id,
                            SampleBuffer::from_frames(vec![StereoFrame::default()]),
                        );
                    }
                    RecordingState::Idle => {} // nothing to do
                }
            }
        }
    }

    pub fn drain_input(&mut self) {
        let rx = match &self.input_rx {
            Some(rx) => rx,
            None => return,
        };

        let mut chunks: Vec<Vec<StereoFrame>> = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            chunks.push(chunk);
        }

        if chunks.is_empty() {
            return;
        }

        match &mut self.recording {
            RecordingState::Idle => {}
            RecordingState::Armed { pre_roll, .. } => {
                let mut triggered = false;
                let mut trigger_offset: usize = 0; // frame index within all chunks where peak was hit
                let mut total_offset: usize = 0;

                'outer: for chunk in &chunks {
                    for (i, frame) in chunk.iter().enumerate() {
                        let level = frame.left.abs().max(frame.right.abs());
                        if level > RECORD_PEAK_THRESHOLD {
                            triggered = true;
                            trigger_offset = total_offset + i;
                            break 'outer;
                        }
                        pre_roll.push(*frame);
                    }
                    total_offset += chunk.len();
                }

                if triggered {
                    let mut buffer = pre_roll.drain_ordered();

                    let mut global_idx: usize = 0;
                    for chunk in &chunks {
                        for frame in chunk {
                            if global_idx >= trigger_offset {
                                buffer.push(*frame);
                            }
                            global_idx += 1;
                        }
                    }

                    let sample_id = match std::mem::replace(
                        &mut self.recording,
                        RecordingState::Idle,
                    ) {
                        RecordingState::Armed { sample_id, .. } => sample_id,
                        _ => unreachable!(),
                    };
                    self.recording = RecordingState::Capturing { sample_id, buffer };
                }
            }
            RecordingState::Capturing { buffer, .. } => {
                for chunk in &chunks {
                    buffer.extend_from_slice(chunk);
                }
            }
        }
    }

    /// Fill the output buffer. Call from the stream callback only.
    pub fn render_block(&mut self, out: &mut [StereoFrame]) {
        let n_frames = out.len();
        if n_frames == 0 {
            return;
        }
        let temp = if n_frames <= self.temp_buf.len() { // a small optimization
            &mut self.temp_buf[..n_frames]
        } else {
            self.temp_buf.resize(n_frames, StereoFrame::default());
            &mut self.temp_buf[..]
        };

        for f in out.iter_mut() { // clear to zeros
            *f = StereoFrame::default();
        }

        for active in &mut self.active { // for each active voice
            if !active.voice.active {
                continue;
            }
            let Some(buffer) = self.samples.get(&active.sample_id) else { // get the sample buffer for this voice
                continue;
            };
            for f in temp.iter_mut() { // clear the temp buffer to zeros
                *f = StereoFrame::default();
            }
            active.voice.render_into(buffer, temp); // render the voice into the temp buffer
            for effect in &mut active.effect_chain { // plug in the temp through the effect chain
                effect.process(temp);
            }
            for (i, f) in temp.iter().enumerate().take(n_frames) { // add the temp to the output
                out[i].left += f.left;
                out[i].right += f.right;
            }
        }

        self.active.retain(|a| a.voice.active); // remove voices that have finished playing
    }
}
