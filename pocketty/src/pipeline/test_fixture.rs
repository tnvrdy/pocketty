// Purely for testing: load a sample, register it with the engine, and put it into song state on track 0 / step 0.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;

use crate::audio_api::AudioCommand;
use crate::loader::sample_loader;

use super::song_state::SongState;

pub fn load_test_sample_into_track0(
    state: &Arc<RwLock<SongState>>,
    audio_tx: Sender<AudioCommand>,
    path: &Path,
    sample_rate: u32,
) -> anyhow::Result<()> {
    let (sample_id, buffer) = sample_loader::load(path, sample_rate)?;
    let length = buffer.data.len().min(sample_rate as usize);
    let _ = audio_tx.try_send(AudioCommand::RegisterSample {
        id: sample_id,
        buffer,
    });
    {
        let mut g = state.write().unwrap();
        g.song.tracks[0].sound.sample_id = sample_id;
        g.song.tracks[0].sound.trim_start = 0;
        g.song.tracks[0].sound.length = length;
        g.song.tracks[0].sound.gain = 1.0;
        g.song.tracks[0].sound.pitch = 1.0;
        g.song.tracks[0].sound.effect_chain = vec![];
        g.song.tracks[0].pattern.steps[0].is_active = true;
    }
    Ok(())
}
