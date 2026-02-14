// The smallest unit of audio; one stereo frame
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

impl StereoFrame {
    pub fn zero() -> Self { // just giving `default` a better name for clarity
        Self::default()
    }
}
