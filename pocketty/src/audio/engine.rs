use crate::audio_api::{AudioCommand, TriggerParams};
use crate::shared::PadId;

const MAX_VOICES: usize = 16; // hard cap so we wont malloc in audio callback

#[derive(Clone, Copy, Debug)]
struct Voice { // basic oscillator for now
    phase: f32,
    phase_inc: f32
    amp: f32,
    decay: f32,
    alive: bool,
}

pub struct Engine {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES], // fixed pool of voices
}

impl Engine {
    pub fn new(sample_rate: u32) -> Self {
        // initialize with empty voices
        let empty = Voice {
            phase: 0.0,
            phase_inc: 0.0,
            amp: 0.0,
            decay: 1.0,
            alive: false,
        };

        Self {
            sample_rate: sample_rate as f32,
            voices: [empty; MAX_VOICES],
        }
    }

    pub fn handle_cmd(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::Trigger(t) => self.trigger_voice(t),
        }
    }

    fn trigger_voice(&mut self, t: TriggerParams) {
        let freq = pad_to_freq(t.pad);

        // what slot do we write to?
        let slot = self.voices.iter().position(|v| !v.alive).unwrap_or(0);

        // radians per sample
        let phase_inc = (std::f32::consts::TAU * freq) / self.sample_rate;

        // some defaults just for this test
        self.voices[slot] = Voice {
            phase: 0.0,
            phase_inc,
            amp: 0.25 * t.velocity,
            decay: 0.9995,
            alive: true,
        };
    }

    pub fn next_sample(&mut self) -> f32 {
        // arbitrary sine stuff for this checkpoint
        let mut out = 0.0f32;
        for v in &mut self.voices {
            if !v.alive {
                continue;
            }
            out += v.amp * v.phase.sin();
            v.phase += v.phase_inc;
            if v.phase > std::f32::consts::TAU {
                v.phase -= std::f32::consts::TAU;
            }
            v.amp *= v.decay;
            if v.amp < 0.0005 {
                v.alive = false;
            }
        }

        out
    }
}

fn pad_to_freq(pad: PadId) -> f32 {
    let semis = pad.0 as f32;
    220.0 * 2.0_f32.powf(semis / 12.0)
}
