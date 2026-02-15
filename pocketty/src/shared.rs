// The current input plan, up to yall to implement:
//
// Grid buttons (the 16 pads):
//   1 2 3 4       //  GridDown(0 or ... or 3) / GridUp(0 or ... or 3)
//   q w e r       //  GridDown(4 or ... or 7) / GridUp(4 or ... or 7)
//   a s d f       //  GridDown(8 or ... or 11) / GridUp(8 or ... or 11)
//   z x c v       //  GridDown(12 or ... or 15) / GridUp(12 or ... or 15)
//
// Modifier buttons (keybinds will probably change at some point):
//   g             //  SoundDown / SoundUp
//   h             //  PatternDown / PatternUp
//   t             //  WriteDown / WriteUp
//   Space         //  PlayPress
//   b             //  RecordDown / RecordUp
//   y             //  FxDown / FxUp
//   n             //  BpmDown / BpmUp
//
// Knobs:
//   [ / ]         //  KnobTurnA(-0.05 or 0.05, or whatever other offset we decide on)
//   - / =         //  KnobTurnB(-0.05 or 0.05, or whatever other offset we decide on)
//
// Quit:
//   Esc           //  Quit
//
// The idea of the rendering process:
//   - The goal is for only the middle layer to have the sequencer and parameter 
//     states, and the TUI just renders the display state object on every frame.
//      - Each frame, call `middle.display_state()` to get a `DisplayState`, then...
//      - Draw `leds[0..16]` as OnMedium, OnHigh, or Off (stupid names, I know, 
//        but the PO has 2 light intensities. We might want a blink mode too eventually)
//      - Draw mode indicators like `write_mode` and `playing` icons
//      - If applicable, draw `selected_sound` and `selected_pattern` indication (like 
//        when holding the pattern button, it'll show a high intensity on the current 
//        pattern's button)
//      - Draw `bpm` and a context-dependent `display_text` in the screen segment
//      - Draw `param_page` text (Tone, Filter, Trim), and the current `knob_a_label/value` 
//        and `knob_b_label/value` in the screen segment
//      - Probably other things too eventually...
//   - But yeah, this middle layer is where all of the complexity lies; the TUI just reads
//     what text, icons, LEDs, and Knob values to display, and does that.

pub const NUM_PADS: usize = 16;
pub const NUM_PATTERNS: usize = 16;
pub const NUM_SOUNDS: usize = 16;
pub const STEPS_PER_PATTERN: usize = 16;

// ye olde types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PadId(pub u8);

#[derive(Clone, Debug)]
pub enum UiAction {
    PadDown(PadId),
    Quit,
}


#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    // grid buttons
    GridDown(u8), // index 0-15
    GridUp(u8),

    // "sound" button
    SoundDown,
    SoundUp,

    // "pattern" button
    PatternDown,
    PatternUp,

    // "write" button
    WriteDown,
    WriteUp,

    // "play/stop" button (space)
    PlayPress,

    // "record" button (b)
    RecordDown,
    RecordUp,

    // "fx" button (y)
    FxDown,
    FxUp,

    // "bpm" button (n)
    BpmDown,
    BpmUp,

    // knobs
    KnobTurnA(f32),
    KnobTurnB(f32),

    // quit button (esc)
    Quit,

    // semantic grid events!! now resolving by tui and not sending keyevents to backend lol
    SelectSound(u8), // held sound + grid press
    SelectPattern(u8), // held pattern + grid press (stopped)
    ChainPattern(u8), // held pattern + grid press (playing)
    SetVolume(u8), // held bpm + grid press
    ToggleStep(u8), // write_mode + grid press (stopped)
    LiveRecordStep(u8), // held write + grid press (playing)
    SetRealtimeEffect(u8), // held fx + grid press (playing)
    ClearRealtimeEffect, // held fx + grid 16 (playing)
    DeleteSound, // held record + held sound
    TriggerPad(u8), // default: play pad melodically

    // semantic knob events, again resolving by tui
    AdjustSwing(f32), // held bpm + knob a
    AdjustBpm(f32), // held bpm + knob b
    PitchLockStep(f32), // held write + playing + knob a
    GainLockStep(f32), // held write + playing + knob b
    AdjustPitch(f32), // default knob a (tone page)
    AdjustGain(f32), // default knob b (tone page)
    AdjustFilterCutoff(f32), // default knob a (filter page)
    AdjustFilterResonance(f32), // default knob b (filter page)
    AdjustTrimStart(f32), // default knob a (trim page)
    AdjustTrimLength(f32), // default knob b (trim page)
}

#[derive(Clone, Debug)]
pub struct DisplayState {
    pub leds: [LedState; STEPS_PER_PATTERN],
    pub playing_step: Option<u8>, // if in sequence mode, which step is playing
    pub write_mode: bool,
    pub playing: bool, // whether we're in sequence mode and playing
    pub param_page: ParamPage, // knob text
    pub selected_sound: u8, // current sound slot
    pub selected_pattern: u8, // current pattern slot
    pub bpm: f32,
    pub display_text: String, // 4-6 chars of text to be displayed, not entirely sure what these will definitively be yet.
    pub knob_a_label: &'static str, // "PITCH", "CUTOFF", "START"
    pub knob_b_label: &'static str, // "GAIN", "RESO", "LENGTH"
    pub knob_a_value: f32,
    pub knob_b_value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedState {
    Off,
    OnMedium,
    OnHigh,

    // Will require a little bit of fanciness from the TUI to implement, because 
    // the blinking likely won't happen on every frame.
    Blink 
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamPage {
    Tone,
    Filter,
    Trim,
}

impl ParamPage {
    pub fn next(self) -> Self {
        match self {
            ParamPage::Tone => ParamPage::Filter,
            ParamPage::Filter => ParamPage::Trim,
            ParamPage::Trim => ParamPage::Tone,
        }
    }

    pub fn knob_labels(self) -> (&'static str, &'static str) {
        match self {
            ParamPage::Tone => ("PITCH", "GAIN"),
            ParamPage::Filter => ("CUTOFF", "RESO"),
            ParamPage::Trim => ("START", "LENGTH"),
        }
    }
}
