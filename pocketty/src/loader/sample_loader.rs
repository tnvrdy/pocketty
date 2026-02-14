use std::path::Path;

use crate::audio::sample_buffer::SampleBuffer;
use crate::audio::{next_sample_id, SampleId};

// Load a WAV from disk, prepare for registration with the engine
pub fn load(path: &Path, target_rate: u32) -> anyhow::Result<(SampleId, SampleBuffer)> {
    let id = next_sample_id();
    let buffer = SampleBuffer::load_wav(path, target_rate, 2)?;
    Ok((id, buffer))
}
