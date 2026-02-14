// holds the song state in an arc and rwlock so the ui can send actions to update it
// even when the sequencer is playing and update in real time.

use std::sync::{Arc, RwLock};
use super::song::Song;

#[derive(Clone)]
pub struct SongState {
    pub song: Song,
    pub is_playing: bool,
}

impl SongState {
    pub fn new_shared() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(SongState { song: Song::new(), is_playing: false }))
    }
}
