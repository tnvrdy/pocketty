/// The Global Project State
#[derive(Clone, Debug)]
pub struct Song {
    pub bpm: u32,             
    pub tracks: [Track; 16],  // Fixed size: 16 tracks (pads)
}

impl Song {
    pub fn new() -> Self {
        Self {
            bpm: 120,
            tracks: std::array::from_fn(|i| Track::new(i)),
        }
    }
}

/// A Single Track (Corresponds to one pad on the PO-33)
#[derive(Clone, Debug)]
pub struct Track {
    pub id: usize,            // 0..15
    pub sound: Sound,         // The audio parameters
    pub pattern: Pattern,     // The sequencer data
}

impl Track {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            sound: Sound::default(),
            pattern: Pattern::default(),
        }
    }
}

/// The Sound Engine Parameters (What the Audio Thread needs)
#[derive(Clone, Debug)]
pub struct Sound {
    pub sample_path: String,  // e.g., "assets/kick.wav"
    pub trim_start: usize,    // Sample index (0..len)
    pub length: usize,        // Number of samples to play
    pub gain: f32,            // 0.0 to 1.0 (Volume)
    pub pitch: f32,           // 0.5 to 2.0 (Playback speed)
    pub effect_chain: Vec<Effect>, // Ordered list of effects
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            sample_path: String::from("../audio_samples/808CHH01.wav"), // Placeholder
            trim_start: 0,
            length: 44100,    // Default 1 second (at 44.1kHz)
            gain: 1.0,
            pitch: 1.0,
            effect_chain: Vec::new(),
        }
    }
}

// The Pattern Data (16 steps)
#[derive(Clone, Debug)]
pub struct Pattern {
    pub swing: f32,        // 0.0 - 1.0
    pub steps: [Step; 16], // The Grid
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            swing: 0.0,
            steps: [Step { is_active: false, pitch: 1.0 }; 16],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub is_active: bool,
    pub pitch: f32, // 1.0 = normal, 2.0 = octave up
}




