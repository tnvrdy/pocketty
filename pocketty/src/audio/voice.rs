use super::frame::StereoFrame;
use super::sample_buffer::SampleBuffer;

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[derive(Clone, Debug)]
pub struct Voice {
    pub pos: f32,
    pub pitch: f32,
    pub gain: f32,
    pub active: bool,
    trim_start: usize,
    length: usize,
}

impl Voice {
    pub fn new(trim_start: usize, length: usize, pitch: f32, gain: f32) -> Self {
        Self {
            pos: 0.0,
            pitch,
            gain,
            active: true,
            trim_start,
            length,
        }
    }

    pub fn render_into(&mut self, buffer: &SampleBuffer, out: &mut [StereoFrame]) {
        // we're at a certain playback position, it's our job to render this voice into the output buffer
        if !self.active {
            return;
        }
        if self.length == 0 {
            self.active = false;
            return;
        }

        let data = &buffer.data;

        for frame in out.iter_mut() { // for each frame in the output buffer
            if !self.active {
                break;
            }

            if self.pos >= self.length as f32 {
                self.active = false;
                break;
            }

            let sample = if self.pos < 0.0 {
                StereoFrame::default() // zeros
            } else { // lerp between the two frames, similar to the resampler
                let i = self.pos as usize;
                if i >= self.length { // keep bounded
                    self.active = false;
                    break;
                }
                let frac = self.pos - i as f32;
                let idx = self.trim_start + i;
                let s0 = data[idx];
                let s1 = data.get(idx + 1).copied().unwrap_or(s0);
                StereoFrame {
                    left: lerp(s0.left, s1.left, frac),
                    right: lerp(s0.right, s1.right, frac),
                }
            };

            // gain
            frame.left += sample.left * self.gain;
            frame.right += sample.right * self.gain;

            // pitch (changes how fast we move forward in the buffer)
            self.pos += self.pitch;
        }
    }
}
