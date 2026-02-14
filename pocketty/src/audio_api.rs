use crate::audio::effect::EffectSpec;
use crate::audio::sample_buffer::SampleBuffer;
use crate::audio::SampleId;

pub use crate::audio::SampleId;

#[derive(Clone, Debug)]
pub struct TriggerParams {
    pub sample_id: SampleId,
    pub trim_start: usize,
    pub length: usize,
    pub gain: f32,
    pub pitch: f32,
    pub effect_chain: Vec<EffectSpec>,
}

#[derive(Clone, Debug)]
pub enum AudioCommand {
    // The engine can't load files (interrupts thread), so we you must first 
    // register a preloaded buffer (see sample_loader.rs), then send that to 
    // the engine
    RegisterSample { id: SampleId, buffer: SampleBuffer },  
    
    // The engine then uses the sample id to trigger the sound 
    Trigger(TriggerParams),
}
