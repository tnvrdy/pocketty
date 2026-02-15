use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
    capturing_flag: Arc<AtomicBool>,
    _output_stream: cpal::Stream,

    // Input device switching
    input_stream: Option<cpal::Stream>,
    input_tx: Sender<Vec<StereoFrame>>,
    sample_rate: cpal::SampleRate,
    input_device_index: usize,

    // Sample registry clone (for offline bounce)
    sample_registry: HashMap<SampleId, SampleBuffer>,
}

impl AudioHandle {
    /// Send a command to the engine. Also keeps a clone of registered samples
    /// so we can do offline bounce on the main thread.
    pub fn send(&mut self, cmd: AudioCommand) {
        if let AudioCommand::RegisterSample { id, ref buffer } = cmd {
            self.sample_registry.insert(id, buffer.clone());
        }
        let _ = self.tx.try_send(cmd);
    }

    /// Access the sample registry (for offline bounce).
    pub fn samples(&self) -> &HashMap<SampleId, SampleBuffer> {
        &self.sample_registry
    }

    /// The output device's actual sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn poll_completed_recording(&self) -> Option<CompletedRecording> {
        self.completed_rx.try_recv().ok()
    }

    /// True when the engine has crossed the peak threshold and is actively capturing audio.
    pub fn is_capturing(&self) -> bool {
        self.capturing_flag.load(Ordering::Relaxed)
    }

    /// List names of all available input devices.
    pub fn list_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devs| {
                devs.filter_map(|d| d.name().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Name of the currently active input device.
    pub fn current_input_name(&self) -> String {
        let devices = Self::list_input_devices();
        if devices.is_empty() {
            return "none".into();
        }
        devices.get(self.input_device_index)
            .cloned()
            .unwrap_or_else(|| "none".into())
    }

    /// Cycle to the next input device and rebuild the input stream.
    /// Returns the name of the newly selected device.
    pub fn cycle_input_device(&mut self) -> String {
        let host = cpal::default_host();
        let devices: Vec<cpal::Device> = host.input_devices()
            .map(|d| d.collect())
            .unwrap_or_default();

        if devices.is_empty() {
            return "none".into();
        }

        // Advance to next device (wrapping)
        self.input_device_index = (self.input_device_index + 1) % devices.len();
        let device = &devices[self.input_device_index];
        let name = device.name().unwrap_or_else(|_| "???".into());

        // Drop old stream (stops it)
        self.input_stream = None;

        // Build new stream on the selected device
        self.input_stream = build_input_stream_on_device(
            device,
            self.sample_rate,
            self.input_tx.clone(),
        );

        if self.input_stream.is_none() {
            // silently failed — UI shows the device name regardless
        }

        name
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
    let capturing_flag = Arc::new(AtomicBool::new(false));

    match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let output_stream = build_output_stream_f32(
                &device, &config.into(), rx, input_rx, completed_tx,
                channels, Arc::clone(&capturing_flag),
            )?;
            output_stream.play().context("failed to play output stream")?;

            // Find the index of the default input device
            let default_input_name = host.default_input_device()
                .and_then(|d| d.name().ok())
                .unwrap_or_default();
            let all_inputs: Vec<String> = host.input_devices()
                .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
                .unwrap_or_default();
            let input_device_index = all_inputs.iter()
                .position(|n| n == &default_input_name)
                .unwrap_or(0);

            let input_stream = try_build_input_stream(&host, sample_rate, input_tx.clone());

            Ok(AudioHandle {
                tx,
                completed_rx,
                capturing_flag,
                _output_stream: output_stream,
                input_stream,
                input_tx,
                sample_rate,
                input_device_index,
                sample_registry: HashMap::new(),
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
    capturing_flag: Arc<AtomicBool>,
) -> anyhow::Result<cpal::Stream> {
    let mut engine = Engine::new(capturing_flag);
    engine.set_input_rx(input_rx);
    engine.set_completed_tx(completed_tx);

    let err_fn = |err: cpal::StreamError| { let _ = err; };

    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _info| {
            while let Ok(cmd) = rx.try_recv() {
                engine.handle_cmd(cmd);
            }

            engine.drain_input();

            let n_frames = data.len() / channels;
            let frames: &mut [StereoFrame] = unsafe {
                std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut StereoFrame, n_frames)
            };
            engine.render_block(frames);
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

// ── Input stream (default device) ────────────────────────────────

fn try_build_input_stream(
    host: &cpal::Host,
    target_sample_rate: cpal::SampleRate,
    tx: Sender<Vec<StereoFrame>>,
) -> Option<cpal::Stream> {
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            return None;
        }
    };

    build_input_stream_on_device(&device, target_sample_rate, tx)
}

// ── Input stream (specific device) ──────────────────────────────

fn build_input_stream_on_device(
    device: &cpal::Device,
    target_sample_rate: cpal::SampleRate,
    tx: Sender<Vec<StereoFrame>>,
) -> Option<cpal::Stream> {
    let supported = device.default_input_config().ok()?;
    let stream_config: cpal::StreamConfig = supported.into();

    // Use the device's native sample rate — forcing a different rate causes
    // many devices (AirPods, BlackHole) to silently produce no audio.
    let device_rate_hz = stream_config.sample_rate;
    let target_rate_hz = target_sample_rate;
    let resample_ratio = (target_rate_hz as f64) / (device_rate_hz as f64);
    let needs_resample = (resample_ratio - 1.0).abs() > 0.001;

    let in_channels = stream_config.channels as usize;

    let err_fn = |err: cpal::StreamError| { let _ = err; };

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

                // Resample to target rate if the device runs at a different rate
                let output = if needs_resample {
                    resample_linear_frames(&frames, resample_ratio)
                } else {
                    frames
                };

                let _ = tx.try_send(output);
            },
            err_fn,
            None,
        )
        .ok()?;

    if stream.play().is_err() {
        return None;
    }

    Some(stream)
}

/// Simple linear interpolation resampler for input frames.
fn resample_linear_frames(input: &[StereoFrame], ratio: f64) -> Vec<StereoFrame> {
    if input.is_empty() {
        return Vec::new();
    }
    let out_len = (input.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;
        let s0 = input[idx.min(input.len() - 1)];
        let s1 = input[(idx + 1).min(input.len() - 1)];
        output.push(StereoFrame {
            left: s0.left * (1.0 - frac) + s1.left * frac,
            right: s0.right * (1.0 - frac) + s1.right * frac,
        });
    }
    output
}

// ── Offline bounce ──────────────────────────────────────────────

/// Render a pattern offline into a SampleBuffer.
/// `step_commands[i]` = the AudioCommands to fire at step i (0..15).
/// Output is exactly `n_steps * frames_per_step` frames — hard cutoff at the pattern boundary.
pub fn bounce_offline(
    samples: &HashMap<SampleId, SampleBuffer>,
    step_commands: &[Vec<AudioCommand>],
    frames_per_step: usize,
) -> SampleBuffer {
    let capturing_flag = Arc::new(AtomicBool::new(false));
    let mut engine = Engine::new(capturing_flag);

    // Register all samples
    for (&id, buffer) in samples {
        engine.handle_cmd(AudioCommand::RegisterSample { id, buffer: buffer.clone() });
    }

    let n_steps = step_commands.len();
    let total = n_steps * frames_per_step;
    let mut output = vec![StereoFrame::default(); total];

    for (step_idx, cmds) in step_commands.iter().enumerate() {
        for cmd in cmds {
            engine.handle_cmd(cmd.clone());
        }
        let start = step_idx * frames_per_step;
        let end = start + frames_per_step;
        engine.render_block(&mut output[start..end]);
    }

    SampleBuffer::from_frames(output)
}
