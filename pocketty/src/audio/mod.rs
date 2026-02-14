use anyhow::Context;
use crossbeam_channel::{Receiver, Sender};
use crate::audio_api::AudioCommand;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod engine;
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

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), rx, sample_rate, channels)?,
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config.into(), rx, sample_rate, channels)?,
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config.into(), rx, sample_rate, channels)?,
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play().context("failed to play stream")?;

    Ok(AudioHandle { tx, _stream: stream })
}

fn build_stream<T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<AudioCommand>,
    sample_rate: u32,
    channels: usize,
) -> anyhow::Result<cpal::Stream> {
    let mut engine = Engine::new(sample_rate);

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _info| {
            // Read all commands
            while let Ok(cmd) = rx.try_recv() {
                engine.handle_cmd(cmd);
            }

            // Render audio
            for frame in data.chunks_mut(channels) {
                let s = engine.next_sample();
                let v: T = T::from_sample::<f32>(s);
                for ch in frame.iter_mut() {
                    *ch = v;
                }
            }
        },
        err_fn,
        None
    )?;

    Ok(stream)
}
