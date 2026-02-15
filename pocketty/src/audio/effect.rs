use super::frame::StereoFrame;

// At some point I'd like to split this effects bit into a folder structure; 
// the latter half of this event is going to be spent just making cool effects
// so it should be easy to add them.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    Bitcrusher { levels: u32 },
    Distortion { drive: f32 },
}

impl EffectSpec {
    pub fn to_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectSpec::Bitcrusher { levels } => Box::new(Bitcrusher::new(*levels)),
            EffectSpec::Distortion { drive } => Box::new(Distortion::new(*drive)),
        }
    }

    pub fn label(&self) -> String {
        match self {
            EffectSpec::Bitcrusher { levels } => format!("Bitcrush({})", levels),
            EffectSpec::Distortion { drive } => format!("Distortion({})", drive),
        }
    }
} 

pub trait Effect: Send {
    fn process(&mut self, buf: &mut [StereoFrame]);
}

//bitcrusher
pub struct Bitcrusher {
    levels: f32,
}

impl Bitcrusher {
    pub fn new(levels: u32) -> Self {
        Self {
            levels: (levels.max(2).min(65536)) as f32,
        }
    }
}

impl Effect for Bitcrusher {
    fn process(&mut self, buf: &mut [StereoFrame]) {
        let scale = (self.levels - 1.0) * 0.5;
        let inv = 1.0 / scale;
        for f in buf.iter_mut() {
            f.left = (f.left.clamp(-1.0, 1.0) * scale).round() * inv;
            f.right = (f.right.clamp(-1.0, 1.0) * scale).round() * inv;
        }
    }
}

//distortion
pub struct Distortion {
    drive: f32,
}

impl Distortion {
    pub fn new(drive: f32) -> Self {
        Self { 
            drive: drive.clamp(0.0, 1.0) as f32,
        }
    }
}

impl Effect for Distortion {
    fn process(&mut self, buf: &mut [StereoFrame]) {
        let pre_gain = 1.0 + self.drive * 10.0; 
        for f in buf.iter_mut() {
            f.left = (pre_gain * f.left.clamp(-1.0, 1.0)).tanh();
            f.right = (pre_gain * f.right.clamp(-1.0, 1.0)).tanh();
        }
    }
}
