use crate::shared::ParamPage;

// state local to tui, mirrors keybinds
// and resolves them into semantic inputevents
// playing, write_mode, and param_page are synced from DisplayState per loop
#[derive(Clone, Debug)]
pub struct TuiState {
    // modifier toggles: press once = on, press again = off
    pub sound_held: bool,
    pub pattern_held: bool,
    pub record_held: bool,
    pub fx_held: bool,
    pub bpm_held: bool,
    // synced from DisplayState each frame
    pub write_mode: bool,
    pub playing: bool,
    pub param_page: ParamPage,
    // grid pad held in write mode (stopped) for per-step knob editing
    pub held_step: Option<u8>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            sound_held: false,
            pattern_held: false,
            record_held: false,
            fx_held: false,
            bpm_held: false,
            write_mode: false,
            playing: false,
            param_page: ParamPage::Tone,
            held_step: None,
        }
    }
}
