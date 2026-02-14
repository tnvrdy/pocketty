use std::collections::HashMap;

use crate::audio_api::{AudioCommand, TriggerParams};
use super::effect::{Effect, EffectSpec};
use super::frame::StereoFrame;
use super::sample_buffer::SampleBuffer;
use super::voice::Voice;
use super::SampleId;

const TEMP_BUF_CAP: usize = 8192; // Sort of arbitrarily chosen, but chosen nonetheless

// An internal attachment between a voice and the params of our trigger call
struct ActiveVoice {
    voice: Voice,
    sample_id: SampleId,
    effect_chain: Vec<Box<dyn Effect>>,
}

pub struct Engine {
    samples: HashMap<SampleId, SampleBuffer>, // the sample buffers we've registered
    active: Vec<ActiveVoice>,
    temp_buf: Vec<StereoFrame>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            active: Vec::new(),
            temp_buf: vec![StereoFrame::default(); TEMP_BUF_CAP],
        }
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
                );
                self.active.push(ActiveVoice {
                    voice,
                    sample_id: params.sample_id,
                    effect_chain,
                });
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
