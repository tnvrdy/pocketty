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
    frames_rendered: usize, // total output frames rendered (bounds stutter lifetime)
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
            frames_rendered: 0,
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

            // stutter blows up without this
            if self.frames_rendered >= self.length {
                self.active = false;
                break;
            }
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

            // Short fade-out near the end to avoid hard clicks (~6ms at 44.1kHz)
            const FADE_SAMPLES: f32 = 256.0;
            // Positional fade (end of sample region)
            let pos_dist = if self.reverse {
                self.pos
            } else {
                (self.length as f32 - self.pos).max(0.0)
            };
            let pos_fade = (pos_dist / FADE_SAMPLES).min(1.0);
            // Lifetime fade (end of stutter lifetime)
            let life_dist = self.length.saturating_sub(self.frames_rendered) as f32;
            let life_fade = (life_dist / FADE_SAMPLES).min(1.0);
            let fade = pos_fade.min(life_fade);

            // gain + fade
            let g = self.gain * fade;
            frame.left += sample.left * g;
            frame.right += sample.right * g;

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

            self.frames_rendered += 1;
        }
    }
}
