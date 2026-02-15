// middle.rs is the brain of the PO

use std::path::Path;
use std::time::Instant;

use crate::audio_api::{AudioCommand, TriggerParams};
use crate::audio::{next_sample_id, EffectSpec, SampleBuffer, SampleId};
use crate::loader::sample_loader;
use crate::pipeline::project::{HeldButtons, ProjectState, SoundSlot};
use crate::shared::*;

const FX_TAP_THRESHOLD_MS: u128 = 200;
const SAMPLE_RATE: f32 = 44100.0;

pub struct Middle {
    pub state: ProjectState,
    held: HeldButtons,
    playing: bool,
    write_mode: bool,
    current_step: u8,
    step_accumulator: f64,
    chain_position: usize,
    param_page: ParamPage,
    fx_down_at: Option<Instant>, // tap/hold detection
    active_rt_effect: Option<u8>, // active real-time effect while fx held
    scratch_position: f32, // 0.0-1.0 normalized position for scratch effect
    display: DisplayState,
}

impl Middle {
    pub fn new() -> Self {
        Self {
            state: ProjectState::default(),
            held: HeldButtons::default(),
            playing: false,
            write_mode: false,
            current_step: 0,
            step_accumulator: 0.0,
            chain_position: 0,
            param_page: ParamPage::Tone,
            fx_down_at: None,
            active_rt_effect: None,
            scratch_position: 0.0,
            display: Self::empty_display(),
        }
    }

    pub fn with_state(state: ProjectState) -> Self {
        let mut m = Self::new();
        m.state = state;
        m
    }

    pub fn handle_input(&mut self, event: InputEvent) -> Vec<AudioCommand> {
        match event {
            InputEvent::SoundDown => { self.held.sound = true; vec![] }
            InputEvent::SoundUp => { self.held.sound = false; vec![] }

            InputEvent::PatternDown => { self.held.pattern = true; vec![] }
            InputEvent::PatternUp => {
                self.held.pattern = false;
                // Chain handling would go here, but we're not there yet.
                vec![]
            }

            InputEvent::WriteDown => {
                self.held.write_held = true;
                if !self.playing {
                    // When stopped, Write toggles write mode
                    self.write_mode = !self.write_mode;
                }
                vec![]
            }
            InputEvent::WriteUp => {
                self.held.write_held = false;
                vec![]
            }

            InputEvent::PlayPress => {
                self.playing = !self.playing;
                if self.playing {
                    self.current_step = 0;
                    self.step_accumulator = 0.0;
                    self.chain_position = 0;
                }
                vec![]
            }

            InputEvent::RecordDown => {
                self.held.record = true;
                // Record + Pattern = clear pattern
                if self.held.pattern {
                    let pi = self.state.selected_pattern as usize;
                    self.state.patterns[pi] = Default::default();
                    return vec![];
                }
                // Record alone = start mic recording
                if !self.held.sound {
                    let sid = next_sample_id();
                    let slot = self.state.selected_sound as usize;
                    let sound = &mut self.state.sounds[slot];
                    sound.sample_id = Some(sid);
                    sound.sample_path = "(recording)".into();
                    sound.trim_start = 0;
                    sound.buffer_len = 0;
                    sound.length = usize::MAX;
                    return vec![AudioCommand::StartRecording { sample_id: sid }];
                }
                vec![]
            }
            InputEvent::RecordUp => {
                self.held.record = false;
                vec![AudioCommand::StopRecording]
            }

            InputEvent::FxDown => {
                self.held.fx = true;
                self.fx_down_at = Some(Instant::now());
                vec![]
            }
            InputEvent::FxUp => {
                self.held.fx = false;
                self.active_rt_effect = None;
                // If it was a quick tap (< threshold), cycle param page
                if let Some(at) = self.fx_down_at.take() {
                    if at.elapsed().as_millis() < FX_TAP_THRESHOLD_MS {
                        self.param_page = self.param_page.next();
                    }
                }
                vec![]
            }

            InputEvent::BpmDown => {
                self.held.bpm = true;
                vec![]
            }
            InputEvent::BpmUp => {
                self.held.bpm = false;
                self.cycle_bpm_preset();
                vec![]
            }

            InputEvent::ClearTrack => {
                // Clear the currently selected sound's track in the current pattern
                let pi = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                self.state.patterns[pi].tracks[sound_idx] = Default::default();
                vec![]
            }

            InputEvent::KnobTurnA(delta) => self.on_knob_a(delta),
            InputEvent::KnobTurnB(delta) => self.on_knob_b(delta),

            // ── Semantic grid events (resolved by TUI) ──────────────

            InputEvent::SelectSound(n) => {
                self.state.selected_sound = n;
                vec![]
            }
            InputEvent::SelectPattern(n) => {
                self.state.selected_pattern = n;
                vec![]
            }
            InputEvent::ChainPattern(n) => {
                self.state.pattern_chain.push(n);
                vec![]
            }
            InputEvent::SetVolume(n) => {
                self.state.master_volume = n; // 1-16
                vec![]
            }
            InputEvent::ToggleStep(n) => {
                let pi = self.state.selected_pattern as usize;
                let si = self.state.selected_sound as usize;
                self.state.patterns[pi].tracks[si].steps[n as usize].active ^= true;
                vec![]
            }
            InputEvent::LiveRecordStep(n) => {
                let quantized_step = self.quantize_to_nearest_step();
                let pi = self.state.selected_pattern as usize;
                let si = self.state.selected_sound as usize;
                self.state.patterns[pi].tracks[si].steps[quantized_step].active = true;
                let _ = n; // pad number used by TUI for resolution; we quantize here
                self.trigger_sound(self.state.selected_sound)
            }
            InputEvent::SetRealtimeEffect(fx_num) => {
                self.active_rt_effect = Some(fx_num);
                if self.write_mode {
                    let pi = self.state.selected_pattern as usize;
                    let sound_idx = self.state.selected_sound as usize;
                    let si = self.current_step as usize;
                    self.state.patterns[pi].tracks[sound_idx].steps[si].effect =
                        Some(fx_num);
                }
                vec![]
            }
            InputEvent::ClearRealtimeEffect => {
                self.active_rt_effect = None;
                if self.write_mode {
                    let pi = self.state.selected_pattern as usize;
                    let si = self.current_step as usize;
                    for track in &mut self.state.patterns[pi].tracks {
                        track.steps[si].effect = None;
                    }
                }
                vec![]
            }
            InputEvent::DeleteSound => {
                self.state.sounds[self.state.selected_sound as usize] = SoundSlot::default();
                vec![]
            }
            InputEvent::TriggerPad(n) => {
                let pitch = Self::pad_to_major_scale_pitch(n);
                self.trigger_sound_with_pitch(self.state.selected_sound, Some(pitch))
            }

            // ── Semantic knob events (resolved by TUI) ──────────────

            InputEvent::AdjustSwing(delta) => {
                self.state.swing = (self.state.swing + delta).clamp(0.0, 1.0);
                vec![]
            }
            InputEvent::AdjustBpm(delta) => {
                self.state.bpm = (self.state.bpm + delta * 180.0).clamp(60.0, 240.0);
                vec![]
            }
            InputEvent::PitchLockStep(delta) => {
                let pi = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                let si = self.current_step as usize;
                let sound = &self.state.sounds[sound_idx];
                let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[si];
                let current = step.pitch_lock.unwrap_or(sound.pitch);
                step.pitch_lock = Some((current + delta * 1.5).clamp(0.5, 2.0));
                vec![]
            }
            InputEvent::GainLockStep(delta) => {
                let pi = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                let si = self.current_step as usize;
                let sound = &self.state.sounds[sound_idx];
                let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[si];
                let current = step.gain_lock.unwrap_or(sound.gain);
                step.gain_lock = Some((current + delta).clamp(0.0, 1.0));
                vec![]
            }
            InputEvent::AdjustPitch(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                sound.pitch = (sound.pitch + delta * 1.5).clamp(0.5, 2.0);
                vec![]
            }
            InputEvent::AdjustGain(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                sound.gain = (sound.gain + delta).clamp(0.0, 1.0);
                vec![]
            }
            InputEvent::AdjustFilterCutoff(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                let factor = if delta > 0.0 { 1.1 } else { 0.9 };
                sound.filter_cutoff = (sound.filter_cutoff * factor).clamp(20.0, 20000.0);
                vec![]
            }
            InputEvent::AdjustFilterResonance(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                sound.filter_resonance = (sound.filter_resonance + delta).clamp(0.0, 1.0);
                vec![]
            }
            InputEvent::AdjustTrimStart(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                let max = sound.buffer_len.saturating_sub(1);
                let step_size = (max as f32 * delta.abs()).max(1.0) as usize;
                if delta > 0.0 {
                    sound.trim_start = (sound.trim_start + step_size).min(max);
                } else {
                    sound.trim_start = sound.trim_start.saturating_sub(step_size);
                }
                let remaining = sound.buffer_len.saturating_sub(sound.trim_start);
                sound.length = sound.length.min(remaining);
                vec![]
            }
            InputEvent::AdjustTrimLength(delta) => {
                let sound = &mut self.state.sounds[self.state.selected_sound as usize];
                let max = sound.buffer_len.saturating_sub(sound.trim_start);
                let step_size = (max as f32 * delta.abs()).max(1.0) as usize;
                if delta > 0.0 {
                    sound.length = (sound.length + step_size).min(max);
                } else {
                    sound.length = sound.length.saturating_sub(step_size).max(1);
                }
                vec![]
            }

            InputEvent::Quit => vec![],
        }
    }

    pub fn tick(&mut self, elapsed: f64) -> Vec<AudioCommand> {
        if !self.playing {
            return vec![];
        }

        self.step_accumulator += elapsed;

        // 6/8 quantize
        let base = 60.0 / (self.state.bpm as f64 * 4.0);
        let secs_per_step = if self.active_rt_effect == Some(13) {
            if self.current_step % 2 == 0 { base * 4.0 / 3.0 } else { base * 2.0 / 3.0 }
        } else {
            base
        };

        let mut commands = Vec::new();

        while self.step_accumulator >= secs_per_step {
            self.step_accumulator -= secs_per_step;
            self.advance_step(&mut commands);
        }

        commands
    }

    /// Advance to the next step and trigger any active sounds.
    fn advance_step(&mut self, commands: &mut Vec<AudioCommand>) {
        self.current_step = (self.current_step + 1) % STEPS_PER_PATTERN as u8;

        // pattern chaining doesn't do anything now, but will
        if self.current_step == 0 && !self.state.pattern_chain.is_empty() {
            self.chain_position =
                (self.chain_position + 1) % self.state.pattern_chain.len();
            self.state.selected_pattern =
                self.state.pattern_chain[self.chain_position];
        }

        let pi = self.state.selected_pattern as usize;
        let si = self.current_step as usize;
        let pattern = &self.state.patterns[pi];

        for (sound_idx, track) in pattern.tracks.iter().enumerate() {
            let step = &track.steps[si];
            if !step.active {
                continue;
            }

            let sound = &self.state.sounds[sound_idx];
            let Some(sample_id) = sound.sample_id else {
                continue;
            };

            let gain = step.gain_lock.unwrap_or(sound.gain)
                * (self.state.master_volume as f32 / 16.0);
            let mut pitch = step.pitch_lock.unwrap_or(sound.pitch);

            // real-time effect takes priority over per-step saved effect
            let fx = self.active_rt_effect.or(step.effect);
            let effect_chain = self.build_effect_chain(sound, fx);

            let (reverse, stutter_period_samples, pitch_mult) =
                self.derive_trigger_mods_from_fx(fx);
            pitch *= pitch_mult;

            commands.push(AudioCommand::Trigger(TriggerParams {
                sample_id,
                trim_start: sound.trim_start,
                length: sound.length,
                gain,
                pitch,
                effect_chain,
                reverse,
                stutter_period_samples,
            }));
        }

        // retrigger effect
        let has_retrigger = self.active_rt_effect == Some(14) || {
            let pattern = &self.state.patterns[pi];
            pattern.tracks.iter().any(|t| {
                let step = &t.steps[si];
                step.active && step.effect == Some(14)
            })
        };
        if has_retrigger {
            // next advance_step will increment this to 0
            self.current_step = STEPS_PER_PATTERN as u8 - 1;
        }
    }

    pub fn display_state(&mut self) -> &DisplayState {
        self.rebuild_display();
        &self.display
    }

    fn rebuild_display(&mut self) {
        // basic display refreshing
        let (a_label, b_label) = self.param_page.knob_labels();

        let mut leds = [LedState::Off; STEPS_PER_PATTERN];

        if self.held.sound {
            leds[self.state.selected_sound as usize] = LedState::OnMedium;
        } else if self.held.pattern {
            leds[self.state.selected_pattern as usize] = LedState::OnMedium;
        } else if self.held.bpm {
            for i in 0..self.state.master_volume as usize {
                if i < STEPS_PER_PATTERN {
                    leds[i] = LedState::OnMedium;
                }
            }
        } else if self.held.fx {
            for led in &mut leds {
                *led = LedState::OnMedium;
            }
        } else {
            let pi = self.state.selected_pattern as usize;
            let si = self.state.selected_sound as usize;
            let track = &self.state.patterns[pi].tracks[si];
            for (i, step) in track.steps.iter().enumerate() {
                if step.active {
                    leds[i] = LedState::OnMedium;
                }
            }
        }

        let playing_step = if self.playing {
            Some(self.current_step)
        } else {
            None
        };
        if let Some(ps) = playing_step {
            leds[ps as usize] = LedState::Blink;
        }

        // Knob values (normalized 0.0-1.0 for display)
        let sound = &self.state.sounds[self.state.selected_sound as usize];
        let (knob_a, knob_b) = match self.param_page {
            ParamPage::Tone => (
                // pitch: 0.5-2.0 mapped to 0.0-1.0
                (sound.pitch - 0.5) / 1.5,
                sound.gain,
            ),
            ParamPage::Filter => (
                // cutoff: 20-20000 mapped to 0.0-1.0 (log scale approximation)
                ((sound.filter_cutoff / 20.0).ln() / (1000.0_f32).ln()).clamp(0.0, 1.0),
                sound.filter_resonance,
            ),
            ParamPage::Trim => (
                if sound.buffer_len > 0 {
                    sound.trim_start as f32 / sound.buffer_len as f32
                } else {
                    0.0
                },
                if sound.buffer_len > 0 {
                    sound.length as f32 / sound.buffer_len as f32
                } else {
                    1.0
                },
            ),
        };

        // Display text
        let display_text = if self.held.bpm {
            format!("VOL {}", self.state.master_volume)
        } else if self.held.sound {
            format!("SND {}", self.state.selected_sound + 1)
        } else if self.held.pattern {
            format!("PAT {}", self.state.selected_pattern + 1)
        } else {
            format!("{:.0} BPM", self.state.bpm)
        };

        self.display = DisplayState {
            leds,
            playing_step,
            write_mode: self.write_mode,
            playing: self.playing,
            param_page: self.param_page,
            selected_sound: self.state.selected_sound,
            selected_pattern: self.state.selected_pattern,
            bpm: self.state.bpm,
            display_text,
            knob_a_label: a_label,
            knob_b_label: b_label,
            knob_a_value: knob_a,
            knob_b_value: knob_b,
        };
    }

    fn empty_display() -> DisplayState {
        DisplayState {
            leds: [LedState::Off; STEPS_PER_PATTERN],
            playing_step: None,
            write_mode: false,
            playing: false,
            param_page: ParamPage::Tone,
            selected_sound: 0,
            selected_pattern: 0,
            bpm: 120.0,
            display_text: String::from("120 BPM"),
            knob_a_label: "PITCH",
            knob_b_label: "GAIN",
            knob_a_value: 0.5,
            knob_b_value: 0.8,
        }
    }

    pub fn load_sample_into_slot(
        &mut self,
        slot: u8,
        path: &Path,
        target_rate: u32,
    ) -> anyhow::Result<AudioCommand> {
        let (sample_id, buffer) = sample_loader::load(path, target_rate)?;
        let sound = &mut self.state.sounds[slot as usize];
        sound.sample_path = path.to_string_lossy().into_owned();
        sound.sample_id = Some(sample_id);
        sound.buffer_len = buffer.data.len();
        sound.trim_start = 0;
        sound.length = buffer.data.len();
        Ok(AudioCommand::RegisterSample { id: sample_id, buffer })
    }

    pub fn clear_slot(&mut self, slot: u8) { // deletes buffers after, say, deleting the wav and reloading pocketty
        if (slot as usize) < self.state.sounds.len() {
            self.state.sounds[slot as usize] = SoundSlot::default();
        }
    }

    pub fn on_recording_complete(
        &mut self,
        sample_id: SampleId,
        buffer: &SampleBuffer,
        project_dir: &Path,
    ) -> anyhow::Result<std::path::PathBuf> {
        // find the slot that was assigned this sample_id
        let slot_idx = self.state.sounds.iter()
            .position(|s| s.sample_id == Some(sample_id))
            .ok_or_else(|| anyhow::anyhow!("no slot found for recorded sample_id"))?;

        // save into .pocketty/recordings/ so we don't clash with user WAVs
        let rec_dir = project_dir.join(".pocketty").join("recordings");
        std::fs::create_dir_all(&rec_dir)?;
        let filename = format!("rec_{:02}.wav", slot_idx);
        let wav_path = rec_dir.join(&filename);

        const SAMPLE_RATE: u32 = 44100;
        buffer.save_wav(&wav_path, SAMPLE_RATE)?;

        // update the slot metadata so persistence and trim work correctly
        let sound = &mut self.state.sounds[slot_idx];
        sound.sample_path = wav_path.to_string_lossy().into_owned();
        sound.buffer_len = buffer.data.len();
        sound.trim_start = 0;
        sound.length = buffer.data.len();

        Ok(wav_path)
    }

    fn on_grid_down(&mut self, n: u8) -> Vec<AudioCommand> {
        let idx = n as usize;
        if idx >= NUM_PADS {
            return vec![];
        }
        self.held.grid[idx] = true;
        if self.held.sound {
            self.state.selected_sound = n;
            return vec![];
        }

        // pattern chaining doesn't do anything now, but will
        if self.held.pattern {
            if self.playing {
                self.state.pattern_chain.push(n);
            } else {
                self.state.selected_pattern = n;
            }
            return vec![];
        }

        if self.held.bpm { // bpm and volume macro
            self.state.master_volume = n + 1; // 1-16
            return vec![];
        }

        // fx, untested
        if self.held.fx && self.playing {
            if n == 15 {
                // fx 16 = clear effects
                self.active_rt_effect = None;
                if self.write_mode {
                    let pi = self.state.selected_pattern as usize;
                    let si = self.current_step as usize;
                    for track in &mut self.state.patterns[pi].tracks {
                        track.steps[si].effect = None;
                    }
                }
            } else {
                let fx_num = n + 1; // 1-15
                self.active_rt_effect = Some(fx_num);
                if self.write_mode {
                    // Save effect to current step for selected sound
                    let pi = self.state.selected_pattern as usize;
                    let sound_idx = self.state.selected_sound as usize;
                    let si = self.current_step as usize;
                    self.state.patterns[pi].tracks[sound_idx].steps[si].effect =
                        Some(fx_num);
                }
            }
            return vec![];
        }

        // Placeholder for eventual mic/screen recording
        if self.held.record {
            // Record + Sound = delete current sound
            if self.held.sound {
                self.state.sounds[self.state.selected_sound as usize] = SoundSlot::default();
                return vec![];
            }
            return vec![];
        }

        if self.write_mode && !self.playing {
            let pi = self.state.selected_pattern as usize;
            let sound_idx = self.state.selected_sound as usize;
            let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[idx];
            step.active = !step.active;
            return vec![];
        }

        // Live recording (rhythm + pitch)
        if self.held.write_held && self.playing {
            let quantized_step = self.quantize_to_nearest_step();
            let pi = self.state.selected_pattern as usize;
            let sound_idx = self.state.selected_sound as usize;

            // self.state.patterns[pi].tracks[sound_idx].steps[quantized_step].active = true;
            // // Also trigger the sound immediately
            // return self.trigger_sound(self.state.selected_sound);
            
            let pitch_mult = Self::pad_to_major_scale_pitch(n);
            let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[quantized_step];
            step.active = true;
            step.pitch_lock = Some(pitch_mult);
            // Also trigger the sound immediately at the recorded pitch
            return self.trigger_sound_with_pitch(self.state.selected_sound, Some(pitch_mult));
        }

        // melodic style is default for all sounds
        let pitch_mult = Self::pad_to_major_scale_pitch(n);
        self.trigger_sound_with_pitch(self.state.selected_sound, Some(pitch_mult))
    }

    fn on_knob_a(&mut self, delta: f32) -> Vec<AudioCommand> {
        // scratch effect
        if matches!(self.active_rt_effect, Some(11) | Some(12)) {
            let sound = &self.state.sounds[self.state.selected_sound as usize];
            if let Some(sample_id) = sound.sample_id {
                let scale = if self.active_rt_effect == Some(11) { 0.2 } else { 0.05 };
                self.scratch_position = (self.scratch_position + delta * scale).clamp(0.0, 1.0);
                let position = self.scratch_position * sound.length as f32;
                return vec![AudioCommand::SetPlaybackPosition { sample_id, position }];
            }
        }

        if self.held.bpm { // swing
            self.state.swing = (self.state.swing + delta).clamp(0.0, 1.0);
            return vec![];
        }

        if self.held.write_held && self.playing { // pitch locking
            let pi = self.state.selected_pattern as usize;
            let sound_idx = self.state.selected_sound as usize;
            let si = self.current_step as usize;
            let sound = &self.state.sounds[sound_idx];
            let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[si];
            let current = step.pitch_lock.unwrap_or(sound.pitch);
            step.pitch_lock = Some((current + delta * 1.5).clamp(0.5, 2.0));
            return vec![];
        }

        // adjust param page by default
        let sound = &mut self.state.sounds[self.state.selected_sound as usize];
        match self.param_page {
            ParamPage::Tone => {
                sound.pitch = (sound.pitch + delta * 1.5).clamp(0.5, 2.0);
            }
            ParamPage::Filter => {
                let factor = if delta > 0.0 { 1.1 } else { 0.9 };
                sound.filter_cutoff = (sound.filter_cutoff * factor).clamp(20.0, 20000.0);
            }
            ParamPage::Trim => {
                let max = sound.buffer_len.saturating_sub(1);
                let step_size = (max as f32 * delta.abs()).max(1.0) as usize;
                if delta > 0.0 {
                    sound.trim_start = (sound.trim_start + step_size).min(max);
                } else {
                    sound.trim_start = sound.trim_start.saturating_sub(step_size);
                }
                // clamping length so we don't exceed buffer length
                let remaining = sound.buffer_len.saturating_sub(sound.trim_start);
                sound.length = sound.length.min(remaining);
            }
        }
        vec![]
    }

    fn on_knob_b(&mut self, delta: f32) -> Vec<AudioCommand> {
        if self.held.bpm {// bpm
            self.state.bpm = (self.state.bpm + delta * 180.0).clamp(60.0, 240.0);
            return vec![];
        }

        if self.held.write_held && self.playing { // gain locking
            let pi = self.state.selected_pattern as usize;
            let sound_idx = self.state.selected_sound as usize;
            let si = self.current_step as usize;
            let sound = &self.state.sounds[sound_idx];
            let step = &mut self.state.patterns[pi].tracks[sound_idx].steps[si];
            let current = step.gain_lock.unwrap_or(sound.gain);
            step.gain_lock = Some((current + delta).clamp(0.0, 1.0));
            return vec![];
        }

        let sound = &mut self.state.sounds[self.state.selected_sound as usize];
        match self.param_page {
            ParamPage::Tone => {
                sound.gain = (sound.gain + delta).clamp(0.0, 1.0);
            }
            ParamPage::Filter => {
                sound.filter_resonance = (sound.filter_resonance + delta).clamp(0.0, 1.0);
            }
            ParamPage::Trim => {
                let max = sound.buffer_len.saturating_sub(sound.trim_start);
                let step_size = (max as f32 * delta.abs()).max(1.0) as usize;
                if delta > 0.0 {
                    sound.length = (sound.length + step_size).min(max);
                } else {
                    sound.length = sound.length.saturating_sub(step_size).max(1);
                }
            }
        }
        vec![]
    }

    fn pad_to_major_scale_pitch(pad_index: u8) -> f32 {
        const PAD_ORDER_LOW_TO_HIGH: [u8; 16] =
            [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];
        const MAJOR_SEMITONES: [i32; 16] =
            [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24, 26];
        let idx = (0..16).find(|&i| PAD_ORDER_LOW_TO_HIGH[i] == pad_index).unwrap_or(0);
        2.0_f32.powf(MAJOR_SEMITONES[idx] as f32 / 12.0)
    }

    // trigger for melodic style
    fn trigger_sound_with_pitch(&self, slot: u8, pitch_override_mult: Option<f32>) -> Vec<AudioCommand> {
        let sound = &self.state.sounds[slot as usize];
        let Some(sample_id) = sound.sample_id else {
            return vec![];
        };

        let gain = sound.gain * (self.state.master_volume as f32 / 16.0);
        let fx = self.active_rt_effect;
        let effect_chain = self.build_effect_chain(sound, fx);
        let (reverse, stutter_period_samples, pitch_mult) =
            self.derive_trigger_mods_from_fx(fx);
        let pitch = match pitch_override_mult {
            Some(m) => sound.pitch * m * pitch_mult,
            None => sound.pitch * pitch_mult,
        };

        vec![AudioCommand::Trigger(TriggerParams {
            sample_id,
            trim_start: sound.trim_start,
            length: sound.length,
            gain,
            pitch,
            effect_chain,
            reverse,
            stutter_period_samples,
        })]
    }

    fn trigger_sound(&self, slot: u8) -> Vec<AudioCommand> {
        self.trigger_sound_with_pitch(slot, None)
    }

    fn build_effect_chain(&self, _sound: &SoundSlot, fx: Option<u8>) -> Vec<EffectSpec> {
        // PO-33 effect map for reference (not all implemented yet):
        //   1-4: Loop variants (not implemented)
        //   5-6: Unison (not implemented)
        //   7: octave up (handled via pitch in advance_step, not effect chain)
        //   8: octave down (handled via pitch in advance_step, not effect chain)
        //   9-10: Stutter { period } (not implemented)
        //   11-12: Scratch (not implemented)
        //   13: 6/8 quantize (sequencer-level, not effect chain)
        //   14: retrigger pattern (sequencer-level)
        //   15: reverse (not implemented)
        //
        // Current mapping using available effects:
        //   1: Distortion (light)
        //   2: Distortion (medium)
        //   3: Distortion (heavy)
        //   4: Bitcrusher (light)
        //   5: Bitcrusher (medium)
        //   6: Bitcrusher (heavy)
        //   7-15: not yet wired to audio effects

        match fx {
            Some(1) => vec![EffectSpec::Distortion { drive: 0.3 }],
            Some(2) => vec![EffectSpec::Distortion { drive: 0.6 }],
            Some(3) => vec![EffectSpec::Distortion { drive: 1.0 }],
            Some(4) => vec![EffectSpec::Bitcrusher { levels: 256 }],
            Some(5) => vec![EffectSpec::Bitcrusher { levels: 32 }],
            Some(6) => vec![EffectSpec::Bitcrusher { levels: 8 }],
            _ => vec![],
        }
    }

    fn derive_trigger_mods_from_fx(&self, fx: Option<u8>) -> (bool, Option<u32>, f32) {
        let reverse = fx == Some(15);

        let stutter_period_samples = match fx {
            Some(9) => {
                // 1/16 note stutter
                let secs = 60.0 / (self.state.bpm * 4.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(10) => {
                // 1/32 note stutter
                let secs = 60.0 / (self.state.bpm * 8.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            _ => None,
        };

        let pitch_mult = match fx {
            Some(7) => 2.0,  // octave up
            Some(8) => 0.5,  // octave down
            _ => 1.0,
        };

        (reverse, stutter_period_samples, pitch_mult)
    }

    fn cycle_bpm_preset(&mut self) {
        self.state.bpm = match self.state.bpm as u32 {
            0..=99 => 120.0,
            100..=129 => 140.0,
            _ => 80.0,
        };
    }

    // live recording quantization attempt
    fn quantize_to_nearest_step(&self) -> usize {
        let secs_per_step = 60.0 / (self.state.bpm as f64 * 4.0);
        let fraction = self.step_accumulator / secs_per_step;

        if fraction >= 0.5 {
            ((self.current_step as usize + 1) % STEPS_PER_PATTERN)
        } else {
            self.current_step as usize
        }
    }
}
