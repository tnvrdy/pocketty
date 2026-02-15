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
    pub reverse: bool,
    trim_start: usize,
    length: usize,
    stutter_period: Option<u32>,
}

impl Voice {
    pub fn new(
        trim_start: usize,
        length: usize,
        pitch: f32,
        gain: f32,
        reverse: bool,
        stutter_period: Option<u32>,
    ) -> Self {
        let pos = if reverse && length > 0 {
            (length - 1) as f32
        } else {
            0.0
        };
        Self {
            pos,
            pitch,
            gain,
            active: true,
            reverse,
            trim_start,
            length,
            stutter_period,
        }
    }

    pub fn set_pos(&mut self, pos: f32) {
        if self.length > 0 {
            self.pos = pos.clamp(0.0, (self.length as f32) - 1.0);
        }
    }

    pub fn render_into(&mut self, buffer: &SampleBuffer, out: &mut [StereoFrame]) {
        // we're at a certain playback position, it's our job to render this voice into the output buffer
        if !self.active {
            return;
        }
        let available = buffer.data.len().saturating_sub(self.trim_start);
        if available == 0 {
            self.active = false;
            return;
        }
        self.length = self.length.min(available);

        if self.length == 0 {
            self.active = false;
            return;
        }

        let data = &buffer.data;

        for frame in out.iter_mut() { // for each frame in the output buffer
            if !self.active {
                break;
            }

            // for stuttering
            if self.stutter_period.is_none() {
                if self.reverse && self.pos < 0.0 {
                    self.active = false;
                    break;
                }
                if !self.reverse && self.pos >= self.length as f32 {
                    self.active = false;
                    break;
                }
            }

            // read sample at current position
            let read_pos = self.pos.clamp(0.0, (self.length as f32) - 1.0);
            let i = read_pos as usize;
            if i >= self.length {
                self.active = false;
                break;
            }
            let frac = read_pos - i as f32;
            let idx = self.trim_start + i;
            let s0 = data[idx];
            let s1 = data.get(idx + 1).copied().unwrap_or(s0);
            let sample = StereoFrame {
                left: lerp(s0.left, s1.left, frac),
                right: lerp(s0.right, s1.right, frac),
            };

            // gain
            frame.left += sample.left * self.gain;
            frame.right += sample.right * self.gain;

            // advance position
            if self.reverse {
                self.pos -= self.pitch;
            } else {
                self.pos += self.pitch;
            }

            // stutter wrap
            if let Some(period) = self.stutter_period {
                let p = (period as f32).min(self.length as f32);
                if p > 0.0 {
                    if self.reverse {
                        while self.pos < 0.0 {
                            self.pos += p;
                        }
                    } else {
                        while self.pos >= p {
                            self.pos -= p;
                        }
                    }
                }
            }
        }
    }
}
