// defines a ton of structs for middle.rs to finangle

use serde::{Deserialize, Serialize}; // serde does json
use crate::audio::SampleId;
use crate::shared::{NUM_PATTERNS, NUM_SOUNDS, STEPS_PER_PATTERN};

// -- DEFINITIONS --
// I hate all of this terminology. 
// The problem is that our internal explanations were misaligned with the teenage engineering terminology.
// I'll go ahead and define it here even though it's an inappropriate spot to do so:
//
// "pattern": what we were previously calling a "song", essentially a collection of tracks.
// "track": what we were previously calling a "pattern", essentially a collection of steps that we toggle to make sequences.
// "step": a single note of a particular sample, all that stuff stores in a SoundSlot.


// One of 16 instrument slots, is what TriggerParams is constructed from
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundSlot {
    pub sample_path: String,

    // We don't want to restore these on startup; they're load garbage values from the previous saved json.
    #[serde(skip)]
    pub sample_id: Option<SampleId>,
    #[serde(skip)]
    pub buffer_len: usize,

    pub trim_start: usize,
    pub length: usize,
    pub gain: f32,
    pub pitch: f32,

    // I'm thinking of doing the full PO-33 stuff here isntead of the OP-1 auto adsr stuff manit was talking about.
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
}

impl Default for SoundSlot {
    fn default() -> Self {
        Self {
            sample_path: String::new(),
            sample_id: None,
            buffer_len: 0,
            trim_start: 0,
            length: 44100,
            gain: 0.8,
            pitch: 1.0,
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
        }
    }
}

impl SoundSlot {
    pub fn is_loaded(&self) -> bool { self.sample_id.is_some() }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub tracks: [Track; NUM_SOUNDS],
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            tracks: std::array::from_fn(|_| Track::default()), // inherits defaults
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub steps: [Step; STEPS_PER_PATTERN],
}

impl Default for Track {
    fn default() -> Self {
        Self {
            steps: [Step::default(); STEPS_PER_PATTERN],
        }
    }
}


#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Step {
    pub active: bool,                        // has it been toggled in the UI?
    pub pitch_lock: Option<f32>,             // updates default pitch in trigger call (multiplied)
    pub gain_lock: Option<f32>,              // updates gain similarly
    pub filter_cutoff_lock: Option<f32>,     // updates filter cutoff similarly
    pub filter_resonance_lock: Option<f32>,  // updates filter resonance similarly

    // Upon review of the manual, we're only ever going to have one effect on a step at a time.
    // Also now that I think about it, the PO-33 doesn't even have sound-level effects, only global effects.
    // I guess we'll add our own effects later that can edit on the sound level, but for now I suppose all we
    // need to worry about is global effects. And because we're procrastinating that part anyways, I guess effects
    // won't matter for a while anyways.
    pub effect: Option<u8>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectState {
    pub sounds: [SoundSlot; NUM_SOUNDS],
    pub selected_sound: u8, // what sound/channel (previously called "pattern") are we on?
    pub patterns: [Pattern; NUM_PATTERNS],
    pub selected_pattern: u8, // what pattern (previously called "song") are we on?
    pub bpm: f32,
    pub current_step: u8, // what step are we on?

    // Fancy stuff
    pub swing: f32, // Not entirely sure how this is handled, probably an offset in the sequencer loop
    pub master_volume: u8, // It'd be fun to implement the PO BPM+1-16 volume control
    pub pattern_chain: Vec<u8>, // Also like a very, very end-game feature, definitely not needed for the demo.
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            sounds: std::array::from_fn(|_| SoundSlot::default()),
            selected_sound: 0,
            patterns: std::array::from_fn(|_| Pattern::default()),
            selected_pattern: 0,
            bpm: 120.0,
            swing: 0.0,
            master_volume: 8,
            pattern_chain: Vec::new(),
        }
    }
}

// We'll have to store held buttons here so the UI doesn't have to interpret any of the button combinations itself.
#[derive(Clone, Debug, Default)]
pub struct HeldButtons {
    pub sound: bool,
    pub pattern: bool,
    pub record: bool,
    pub fx: bool,
    pub bpm: bool,
    pub write_held: bool, // while certain other command-buttons are just toggled, write being held signifies live record
    pub grid: [bool; NUM_SOUNDS], // chords/passing in pitch as an array in trigger eventually?
}
