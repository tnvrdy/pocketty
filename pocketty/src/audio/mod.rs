use anyhow::Context;
use crossbeam_channel::{Receiver, Sender};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio_api::AudioCommand;

mod effect;
mod engine;
mod frame;
mod sample_buffer;
mod sample_id;
mod voice;

pub use effect::{Effect, EffectSpec};
pub use frame::StereoFrame;
pub use sample_buffer::SampleBuffer;
pub use sample_id::{next_sample_id, SampleId};

use engine::{CompletedRecording, Engine};

pub struct AudioHandle {
    tx: Sender<AudioCommand>,
    completed_rx: Receiver<CompletedRecording>,
    _output_stream: cpal::Stream,
    _input_stream: Option<cpal::Stream>, // None when no mic available
}

impl AudioHandle {
    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.tx.try_send(cmd);
    }

    pub fn poll_completed_recording(&self) -> Option<CompletedRecording> {
        self.completed_rx.try_recv().ok()
    }
}

pub fn start_audio() -> anyhow::Result<AudioHandle> {
    let (tx, rx) = crossbeam_channel::bounded::<AudioCommand>(1024);

    let host = cpal::default_host();
    let device = host.default_output_device().context("no default output device")?;
    let config = device.default_output_config().context("no default output config")?;

    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    let (input_tx, input_rx) = crossbeam_channel::bounded::<Vec<StereoFrame>>(2048);
    let (completed_tx, completed_rx) = crossbeam_channel::bounded::<CompletedRecording>(16);

    match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let output_stream = build_output_stream_f32(
                &device, &config.into(), rx, input_rx, completed_tx, channels,
            )?;
            output_stream.play().context("failed to play output stream")?;

            let input_stream = try_build_input_stream(&host, sample_rate, input_tx);

            Ok(AudioHandle {
                tx,
                completed_rx,
                _output_stream: output_stream,
                _input_stream: input_stream,
            })
        }
        _ => anyhow::bail!("unsupported sample format (only f32 supported for now)"),
    }
}

// ── Output stream ─────────────────────────────────────────────────

fn build_output_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<AudioCommand>,
    input_rx: Receiver<Vec<StereoFrame>>,
    completed_tx: crossbeam_channel::Sender<CompletedRecording>,
    channels: usize,
) -> anyhow::Result<cpal::Stream> {
    let mut engine = Engine::new();
    engine.set_input_rx(input_rx);
    engine.set_completed_tx(completed_tx);

    let err_fn = |err| eprintln!("audio output stream error: {err}");

    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _info| {
            while let Ok(cmd) = rx.try_recv() { // set up command handling
                engine.handle_cmd(cmd);
            }

            // Drain mic input into the recording state machine
            engine.drain_input();

            let n_frames = data.len() / channels;
            let frames: &mut [StereoFrame] = unsafe { // casting raw floats to StereoFrames
                std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut StereoFrame, n_frames)
            };
            engine.render_block(frames); // filling that temp buffer with the audio data
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn try_build_input_stream(
    host: &cpal::Host,
    target_sample_rate: cpal::SampleRate,
    tx: Sender<Vec<StereoFrame>>,
) -> Option<cpal::Stream> {
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            eprintln!("pocketty: no default input device — mic recording disabled");
            return None;
        }
    };

    let supported = device.default_input_config().ok()?;
    let mut stream_config: cpal::StreamConfig = supported.into();
    stream_config.sample_rate = target_sample_rate;

    let in_channels = stream_config.channels as usize;

    let err_fn = |err| eprintln!("audio input stream error: {err}");

    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let frames: Vec<StereoFrame> = if in_channels == 1 {
                    data.iter()
                        .map(|&s| StereoFrame { left: s, right: s })
                        .collect()
                } else {
                    data.chunks_exact(in_channels)
                        .map(|c| StereoFrame {
                            left: c[0],
                            right: if c.len() > 1 { c[1] } else { c[0] },
                        })
                        .collect()
                };

                let _ = tx.try_send(frames);
            },
            err_fn,
            None,
        )
        .ok()?;

    if let Err(e) = stream.play() {
        eprintln!("pocketty: could not start input stream: {e}");
        return None;
    }

    Some(stream)
}
