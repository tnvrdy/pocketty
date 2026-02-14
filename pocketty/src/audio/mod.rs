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

use engine::Engine;

pub struct AudioHandle {
    tx: Sender<AudioCommand>,
    _stream: cpal::Stream,
}

impl AudioHandle {
    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.tx.try_send(cmd);
    }
}

pub fn start_audio() -> anyhow::Result<AudioHandle> {
    let (tx, rx) = crossbeam_channel::bounded::<AudioCommand>(1024);

    let host = cpal::default_host();
    let device = host.default_output_device().context("no default output device")?;
    let config = device.default_output_config().context("no default output config")?;

    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let stream = build_stream_f32(&device, &config.into(), rx, channels)?;
            stream.play().context("failed to play stream")?;
            Ok(AudioHandle { tx, _stream: stream })
        }
        _ => anyhow::bail!("unsupported sample format (only f32 supported for now)"),
    }
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<AudioCommand>,
    channels: usize,
) -> anyhow::Result<cpal::Stream> {
    let mut engine = Engine::new();

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _info| {
            while let Ok(cmd) = rx.try_recv() { // set up command handling
                engine.handle_cmd(cmd);
            }

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
