pub use crate::audio::{EffectSpec, SampleBuffer, SampleId};

#[derive(Clone, Debug)]
pub struct TriggerParams {
    pub sample_id: SampleId,
    pub trim_start: usize,
    pub length: usize,
    pub gain: f32,
    pub pitch: f32,
    pub effect_chain: Vec<EffectSpec>,
    pub reverse: bool,                         // reverse effect
    pub stutter_period_samples: Option<u32>,   // loop effects
}

#[derive(Clone, Debug)]
pub enum AudioCommand {
    // The engine can't load files (interrupts thread), so we you must first 
    // register a preloaded buffer (see sample_loader.rs), then send that to 
    // the engine
    RegisterSample { id: SampleId, buffer: SampleBuffer },  
    
    // The engine then uses the sample id to trigger the sound 
    Trigger(TriggerParams),

    StartRecording { sample_id: SampleId },
    StopRecording,

    // Scatch effects
    SetPlaybackPosition { sample_id: SampleId, position: f32 },

    // Kill all playing voices immediately (used when stopping playback)
    StopAllVoices,
}
