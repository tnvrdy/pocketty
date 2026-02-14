use std::path::{Path, PathBuf};
use crate::audio::{next_sample_id, SampleId, SampleBuffer};

// Load a WAV from disk, prepare for registration with the engine
pub fn load(path: &Path, target_rate: u32) -> anyhow::Result<(SampleId, SampleBuffer)> {
    let id = next_sample_id();
    let buffer = SampleBuffer::load_wav(path, target_rate, 2)?;
    Ok((id, buffer))
}

// Auto-assigning samples to slots at startup, will be expanded later.
pub fn index_wav_in_dir(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map_or(false, |e| e.eq_ignore_ascii_case("wav")))
        .collect();

    paths.sort_by_cached_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });

    Ok(paths) // returns the sorted paths
}
