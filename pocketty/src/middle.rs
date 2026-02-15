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
    recording_armed: bool, // true between RecordDown and RecordUp
    is_capturing: bool,    // true when engine is actively capturing audio (set from main loop)
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
            recording_armed: false,
            is_capturing: false,
            display: Self::empty_display(),
        }
    }

    pub fn with_state(state: ProjectState) -> Self {
        let mut m = Self::new();
        m.state = state;
        m
    }

    /// Called from the main loop to update recording capture state from the engine.
    pub fn set_capturing(&mut self, capturing: bool) {
        self.is_capturing = capturing;
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
                // Toggle write mode (stopped or playing)
                self.write_mode = !self.write_mode;
                vec![]
            }
            InputEvent::WriteUp => {
                self.held.write_held = false;
                vec![]
            }

            InputEvent::PlayPress => {
                self.playing = !self.playing;
                if self.playing {
                    // Start one step behind so the first advance_step() lands on step 0
                    self.current_step = (STEPS_PER_PATTERN as u8).wrapping_sub(1);
                    self.step_accumulator = 0.0;
                    self.chain_position = 0;
                    vec![]
                } else {
                    // Stopping: kill all playing voices immediately
                    vec![AudioCommand::StopAllVoices]
                }
            }

            InputEvent::RecordDown => {
                self.held.record = true;
                // Record + Pattern = clear pattern
                if self.held.pattern {
                    let pi = self.state.selected_pattern as usize;
                    self.state.patterns[pi] = Default::default();
                    return vec![];
                }
                // Record alone = arm mic recording into selected sound slot
                if !self.held.sound {
                    self.recording_armed = true;
                    let sid = next_sample_id();
                    let slot = self.state.selected_sound as usize;
                    let sound = &mut self.state.sounds[slot];
                    sound.sample_id = Some(sid);
                    sound.sample_path = "(recording)".into();
                    sound.trim_start = 0;
                    sound.buffer_len = 0;
                    sound.length = usize::MAX; // voice clamps to actual buffer length
                    return vec![AudioCommand::StartRecording { sample_id: sid }];
                }
                vec![]
            }
            InputEvent::RecordUp => {
                self.held.record = false;
                self.recording_armed = false;
                self.is_capturing = false;
                vec![AudioCommand::StopRecording]
            }

            InputEvent::FxDown => {
                self.held.fx = true;
                self.fx_down_at = Some(Instant::now());
                vec![]
            }
            InputEvent::FxUp => {
                self.held.fx = false;
                let had_effect = self.active_rt_effect.is_some();
                self.active_rt_effect = None;
                // If it was a quick tap (< threshold), cycle param page
                if let Some(at) = self.fx_down_at.take() {
                    if at.elapsed().as_millis() < FX_TAP_THRESHOLD_MS {
                        self.param_page = self.param_page.next();
                    }
                }
                // Kill lingering voices (stutter/loop) when leaving fx mode
                if had_effect { vec![AudioCommand::StopAllVoices] } else { vec![] }
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

            // semantic grid events resolved and sent by tui

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
                let step = &mut self.state.patterns[pi].tracks[si].steps[n as usize];
                step.active = !step.active;
                if !step.active {
                    // Reset all per-step locks when untoggling
                    step.pitch_lock = None;
                    step.gain_lock = None;
                    step.filter_cutoff_lock = None;
                    step.filter_resonance_lock = None;
                    step.effect = None;
                }
                vec![]
            }
            InputEvent::LiveRecordStep(n) => {
                let quantized_step = self.quantize_to_nearest_step();
                let pi = self.state.selected_pattern as usize;
                let si = self.state.selected_sound as usize;
                let pitch_mult = Self::pad_to_major_scale_pitch(n);
                let step = &mut self.state.patterns[pi].tracks[si].steps[quantized_step];
                step.active = true;
                step.pitch_lock = Some(pitch_mult);
                // Also trigger immediately at the recorded pitch so you hear what you played
                self.trigger_sound_with_pitch(self.state.selected_sound, Some(pitch_mult))
            }
            InputEvent::SetRealtimeEffect(fx_num) => {
                // Kill old effect voices before switching to new effect
                let mut cmds = vec![AudioCommand::StopAllVoices];
                self.active_rt_effect = Some(fx_num);
                if self.write_mode {
                    let pi = self.state.selected_pattern as usize;
                    let sound_idx = self.state.selected_sound as usize;
                    let si = self.current_step as usize;
                    self.state.patterns[pi].tracks[sound_idx].steps[si].effect =
                        Some(fx_num);
                }
                cmds
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
                // Kill lingering stutter/loop voices immediately
                vec![AudioCommand::StopAllVoices]
            }
            InputEvent::DeleteSound => {
                self.state.sounds[self.state.selected_sound as usize] = SoundSlot::default();
                vec![]
            }
            InputEvent::ClearTrack => {
                let pattern_idx = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                self.state.patterns[pattern_idx].tracks[sound_idx] = Default::default();
                vec![]
            }
            InputEvent::TriggerPad(n) => {
                let pitch = Self::pad_to_major_scale_pitch(n);
                self.trigger_sound_with_pitch(self.state.selected_sound, Some(pitch))
            }

            // semantic knob events resolved and sent by tui

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
                // Multiplicative: ~0.24 semitones per click, round-trips cleanly
                let semitones = delta * 4.8;
                let factor = 2.0_f32.powf(semitones / 12.0);
                step.pitch_lock = Some((current * factor).clamp(0.5, 2.0));
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
                // Multiplicative: ~0.24 semitones per click, round-trips cleanly
                let semitones = delta * 4.8;
                let factor = 2.0_f32.powf(semitones / 12.0);
                sound.pitch = (sound.pitch * factor).clamp(0.5, 2.0);
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
                // Much finer: 0.2% of buffer per click (was 5%)
                let step_size = (max as f32 * delta.abs() * 0.04).max(1.0) as usize;
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
                // A little finer: 1% of buffer per click (was 5%)
                let step_size = (max as f32 * delta.abs() * 0.2).max(1.0) as usize;
                if delta > 0.0 {
                    sound.length = (sound.length + step_size).min(max);
                } else {
                    sound.length = sound.length.saturating_sub(step_size).max(1);
                }
                vec![]
            }

            // Per-step parameter locks (hold step in write mode, stopped, + knob)
            InputEvent::LockStepPitchAt { step, delta } => {
                let pi = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                let sound = &self.state.sounds[sound_idx];
                let s = &mut self.state.patterns[pi].tracks[sound_idx].steps[step as usize];
                let current = s.pitch_lock.unwrap_or(sound.pitch);
                // Semitone-based: delta=0.05 → ~0.24 semitones (all 12 chromatic notes reachable)
                let semitones = delta * 4.8;
                let factor = 2.0_f32.powf(semitones / 12.0);
                s.pitch_lock = Some((current * factor).clamp(0.25, 4.0));
                vec![]
            }
            InputEvent::LockStepGainAt { step, delta } => {
                let pi = self.state.selected_pattern as usize;
                let sound_idx = self.state.selected_sound as usize;
                let sound = &self.state.sounds[sound_idx];
                let s = &mut self.state.patterns[pi].tracks[sound_idx].steps[step as usize];
                let current = s.gain_lock.unwrap_or(sound.gain);
                s.gain_lock = Some((current + delta).clamp(0.0, 1.0));
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

        // Effect 13 (6/8 quantize): triplet swing timing
        let base = 60.0 / (self.state.bpm as f64 * 4.0);
        let secs_per_step = if self.active_rt_effect == Some(13) {
            // Alternate long/short steps to create a triplet feel (2:1 ratio)
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

            // Real-time effect (y + pad) takes priority over per-step saved effect
            let fx = self.active_rt_effect.or(step.effect);
            let effect_chain = self.build_effect_chain(sound, fx);

            // Derive voice-level modifiers from the active effect
            let (reverse, stutter_period_samples, pitch_mult, is_unison, unison_detune) =
                Self::derive_trigger_mods_from_fx(self.state.bpm, fx);
            pitch *= pitch_mult;

            commands.push(AudioCommand::Trigger(TriggerParams {
                sample_id,
                trim_start: sound.trim_start,
                length: sound.length,
                gain,
                pitch,
                effect_chain: effect_chain.clone(),
                reverse,
                stutter_period_samples,
            }));

            // Unison: trigger a second voice with slight detune
            if is_unison {
                let detune_factor = 2.0_f32.powf(unison_detune / 1200.0);
                commands.push(AudioCommand::Trigger(TriggerParams {
                    sample_id,
                    trim_start: sound.trim_start,
                    length: sound.length,
                    gain,
                    pitch: pitch * detune_factor,
                    effect_chain,
                    reverse,
                    stutter_period_samples,
                }));
            }
        }

        // Effect 14 (retrigger): reset pattern to step 0 on next advance
        let has_retrigger = self.active_rt_effect == Some(14) || {
            let pattern = &self.state.patterns[pi];
            pattern.tracks.iter().any(|t| {
                let step = &t.steps[si];
                step.active && step.effect == Some(14)
            })
        };
        if has_retrigger {
            // Next advance_step will increment this to 0
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
                // pitch: 0.5-2.0 mapped to 0.0-1.0 via log2
                // log2(0.5)=-1 → 0.0, log2(1.0)=0 → 0.5, log2(2.0)=1 → 1.0
                ((sound.pitch.log2() + 1.0) / 2.0).clamp(0.0, 1.0),
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

        let recording = if self.is_capturing {
            RecordingDisplay::Capturing
        } else if self.recording_armed {
            RecordingDisplay::Armed
        } else {
            RecordingDisplay::Idle
        };

        self.display = DisplayState {
            leds,
            playing_step,
            write_mode: self.write_mode,
            playing: self.playing,
            recording,
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
            recording: RecordingDisplay::Idle,
            param_page: ParamPage::Tone,
            selected_sound: 0,
            selected_pattern: 0,
            bpm: 120.0,
            display_text: String::from("120 BPM"),
            knob_a_label: "PITCH",
            knob_b_label: "GAIN",
            knob_a_value: 0.5,
            knob_b_value: 0.5,
        }
    }

    pub fn load_sample_into_slot(
        &mut self,
        slot: u8,
        path: &Path,
        target_rate: u32,
    ) -> anyhow::Result<AudioCommand> {
        let (sample_id, buffer) = sample_loader::load(path, target_rate)?;
        let buf_len = buffer.data.len();
        let sound = &mut self.state.sounds[slot as usize];
        let is_fresh = sound.sample_path.is_empty();

        sound.sample_path = path.to_string_lossy().into_owned();
        sound.sample_id = Some(sample_id);
        sound.buffer_len = buf_len;

        if is_fresh {
            // First time loading: use full buffer
            sound.trim_start = 0;
            sound.length = buf_len;
        } else {
            // Restoring from saved state: preserve trim/length, clamp to buffer
            if sound.trim_start >= buf_len {
                sound.trim_start = 0;
            }
            let remaining = buf_len.saturating_sub(sound.trim_start);
            sound.length = sound.length.min(remaining).max(1);
        }

        Ok(AudioCommand::RegisterSample { id: sample_id, buffer })
    }

    pub fn clear_slot(&mut self, slot: u8) { // deletes buffers after, say, deleting the wav and reloading pocketty
        if (slot as usize) < self.state.sounds.len() {
            self.state.sounds[slot as usize] = SoundSlot::default();
        }
    }

    /// Called when the engine finishes a recording. Finds the slot that owns
    /// `sample_id`, updates its metadata, and writes the WAV into
    /// `<project_dir>/.pocketty/recordings/`.
    pub fn on_recording_complete(
        &mut self,
        sample_id: SampleId,
        buffer: &SampleBuffer,
        project_dir: &Path,
    ) -> anyhow::Result<std::path::PathBuf> {
        let slot_idx = self.state.sounds.iter()
            .position(|s| s.sample_id == Some(sample_id))
            .ok_or_else(|| anyhow::anyhow!("no slot found for recorded sample_id"))?;

        let rec_dir = project_dir.join(".pocketty").join("recordings");
        std::fs::create_dir_all(&rec_dir)?;
        let filename = format!("rec_{:02}.wav", slot_idx);
        let wav_path = rec_dir.join(&filename);

        const SAMPLE_RATE: u32 = 44100;
        buffer.save_wav(&wav_path, SAMPLE_RATE)?;

        let sound = &mut self.state.sounds[slot_idx];
        sound.sample_path = wav_path.to_string_lossy().into_owned();
        sound.buffer_len = buffer.data.len();
        sound.trim_start = 0;
        sound.length = buffer.data.len();

        Ok(wav_path)
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
        let (reverse, stutter_period_samples, pitch_mult, is_unison, unison_detune) =
            Self::derive_trigger_mods_from_fx(self.state.bpm, fx);
        let pitch = match pitch_override_mult {
            Some(m) => sound.pitch * m * pitch_mult,
            None => sound.pitch * pitch_mult,
        };

        let mut cmds = vec![AudioCommand::Trigger(TriggerParams {
            sample_id,
            trim_start: sound.trim_start,
            length: sound.length,
            gain,
            pitch,
            effect_chain: effect_chain.clone(),
            reverse,
            stutter_period_samples,
        })];

        if is_unison {
            let detune_factor = 2.0_f32.powf(unison_detune / 1200.0);
            cmds.push(AudioCommand::Trigger(TriggerParams {
                sample_id,
                trim_start: sound.trim_start,
                length: sound.length,
                gain,
                pitch: pitch * detune_factor,
                effect_chain,
                reverse,
                stutter_period_samples,
            }));
        }

        cmds
    }

    fn trigger_sound(&self, slot: u8) -> Vec<AudioCommand> {
        self.trigger_sound_with_pitch(slot, None)
    }

    fn build_effect_chain(&self, _sound: &SoundSlot, _fx: Option<u8>) -> Vec<EffectSpec> {
        // PO-33 effects are all handled via voice params (stutter, pitch, reverse)
        // or sequencer logic (retrigger, 6/8 quantize). None use the sample-domain
        // effect chain. Keeping this for future custom effects.
        //
        // PO-33 effect map (all handled outside the chain):
        //   1: loop 16       → stutter (1 beat)
        //   2: loop 12       → stutter (triplet)
        //   3: loop short    → stutter (1/2 step)
        //   4: loop shorter  → stutter (1/4 step)
        //   5: unison        → double trigger with +7 cent detune
        //   6: unison low    → double trigger with -7 cent detune
        //   7: octave up     → pitch *= 2.0
        //   8: octave down   → pitch *= 0.5
        //   9: stutter 4     → stutter (1 step)
        //   10: stutter 3    → stutter (triplet step)
        //   11: scratch       → knob A sends SetPlaybackPosition
        //   12: scratch fast  → knob A sends SetPlaybackPosition (fine)
        //   13: 6/8 quantize  → tick() adjusts step timing
        //   14: retrigger     → advance_step resets current_step
        //   15: reverse       → reverse flag on voice
        vec![]
    }

    /// Derive voice-level modifiers (reverse, stutter, pitch) from an effect number.
    /// These are NOT in the effect chain — they change how the Voice reads the buffer.
    /// Returns (reverse, stutter_period_samples, pitch_mult, is_unison, unison_detune).
    fn derive_trigger_mods_from_fx(bpm: f32, fx: Option<u8>) -> (bool, Option<u32>, f32, bool, f32) {
        let reverse = fx == Some(15);

        let stutter_period_samples = match fx {
            Some(1) => {
                // loop 16: 1 whole beat
                let secs = 60.0 / bpm;
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(2) => {
                // loop 12: triplet beat (1/3 of a bar = 1 beat in 3/4)
                let secs = 60.0 / (bpm * 3.0 / 2.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(3) => {
                // loop short: 1/2 step
                let secs = 60.0 / (bpm * 8.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(4) => {
                // loop shorter: 1/4 step
                let secs = 60.0 / (bpm * 16.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(9) => {
                // stutter 4: 1 step (1/16 note)
                let secs = 60.0 / (bpm * 4.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            Some(10) => {
                // stutter 3: triplet step (1/12 note)
                let secs = 60.0 / (bpm * 12.0);
                Some((secs * SAMPLE_RATE) as u32)
            }
            _ => None,
        };

        let pitch_mult = match fx {
            Some(7) => 2.0,  // octave up
            Some(8) => 0.5,  // octave down
            _ => 1.0,
        };

        // Unison: trigger a second voice with slight detune
        let (is_unison, unison_detune) = match fx {
            Some(5) => (true, 7.0),   // unison: +7 cents
            Some(6) => (true, -7.0),  // unison low: -7 cents
            _ => (false, 0.0),
        };

        (reverse, stutter_period_samples, pitch_mult, is_unison, unison_detune)
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