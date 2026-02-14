use std::path::Path;
use super::frame::StereoFrame;

#[derive(Clone, Debug)]
pub struct SampleBuffer {
    pub data: Vec<StereoFrame>, // rhe audio data array
}

impl SampleBuffer {
    // Load a WAV file from disk into the sample buffer
    pub fn load_wav(path: &Path, target_rate: u32, target_channels: u16) -> anyhow::Result<Self> {
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        let file_rate = spec.sample_rate;
        let file_channels = spec.channels;

        // Read the samples from the WAV file
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader // float, just pass it through
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()?, 
            hound::SampleFormat::Int => { // int, convert to float
                let max = (1i32 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|x| x as f32 / max)) // cap the int at the max value
                    .collect::<Result<Vec<_>, _>>()?
            },
            _ => anyhow::bail!("Unsupported sample format: {:?}", spec.sample_format),
        };

        let mut frames: Vec<StereoFrame> = if file_channels == 1 {
            samples
                .into_iter()
                .map(|x| StereoFrame { // mono, duplicate
                    left: x, 
                    right: x 
                })
                .collect()
        } else {
            samples
                .chunks_exact(2)
                .map(|c| StereoFrame {
                    left: c[0],
                    right: c[1],
                })
                .collect()
        };

        if file_rate != target_rate {
            frames = resample_linear(&frames, file_rate, target_rate);
        }

        if target_channels != 2 {
            anyhow::bail!("Pocketty only supports stereo output right now");
        }

        Ok(Self { data: frames })
    }
}

fn resample_linear(frames: &[StereoFrame], source_rate: u32, target_rate: u32) -> Vec<StereoFrame> {
    // This is a simple linear resampler, we might want to use a better one past the treehacks context
    if source_rate == target_rate {
        return frames.to_vec();
    }
    let ratio = target_rate as f64 / source_rate as f64;
    let out_len = (frames.len() as f64 * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        // fractional position in the source buffer
        let src_pos = i as f64 / ratio; // ex. 3.7
        let idx = src_pos.floor() as usize; // ex. 3
        let frac = (src_pos - idx as f64) as f32; // ex. 0.7
        if idx >= frames.len().saturating_sub(1) { // edge case
            out.push(*frames.last().unwrap_or(&StereoFrame::zero()));
        } else {
            let a = frames[idx]; // ex. frame 3
            let b = frames[idx + 1]; // ex. frame 4
            out.push(StereoFrame { // blend via frac and linear interpolation
                left: a.left * (1.0 - frac) + b.left * frac,
                right: a.right * (1.0 - frac) + b.right * frac,
            });
        }
    }
    out
}
