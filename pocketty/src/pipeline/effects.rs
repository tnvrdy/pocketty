// The Effect Data 

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectType {
    Distortion,
    Reverb,
    LowPass,
    HighPass,
    BitCrush,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub kind: EffectType,
    pub intensity: f32, // 0.0 to 1.0
}

impl Effect {
    pub fn new(kind: EffectType, intensity: f32) -> Self {
        Self { kind, intensity }
    }
}