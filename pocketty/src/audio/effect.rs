use super::frame::StereoFrame;

// At some point I'd like to split this effects bit into a folder structure; 
// the latter half of this event is going to be spent just making cool effects
// so it should be easy to add them.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    Bitcrusher { levels: u32 },
}

impl EffectSpec {
    pub fn to_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectSpec::Bitcrusher { levels } => Box::new(Bitcrusher::new(*levels)),
        }
    }

    pub fn label(&self) -> String {
        match self {
            EffectSpec::Bitcrusher { levels } => format!("Bitcrush({})", levels),
        }
    }
} 

pub trait Effect: Send {
    fn process(&mut self, buf: &mut [StereoFrame]);
}

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