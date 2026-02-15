use crate::shared::ParamPage;

// state local to tui, mirrors keybinds
// and resolves them into semantic inputevents
// playing, write_mode, and param_page are synced from DisplayState per loop
#[derive(Clone, Debug)]
pub struct TuiState {
    pub sound_held: bool,
    pub pattern_held: bool,
    pub write_held: bool,
    pub record_held: bool,
    pub fx_held: bool,
    pub bpm_held: bool,
    pub write_mode: bool, // toggled, not held. synced from DisplayState
    pub playing: bool, // synced from DisplayState
    pub param_page: ParamPage, // synced from DisplayState
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            sound_held: false,
            pattern_held: false,
            write_held: false,
            record_held: false,
            fx_held: false,
            bpm_held: false,
            write_mode: false,
            playing: false,
            param_page: ParamPage::Tone,
        }
    }
}
