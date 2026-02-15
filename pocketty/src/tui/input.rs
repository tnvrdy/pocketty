use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::shared::{InputEvent, ParamPage};
use super::mode::TuiState;

// All modifier buttons are TOGGLES: press once = on, press again = off.
// Buttons do NOT repeat when held. Knobs DO repeat when held.
// Keyboard enhancement (if the terminal supports it) gives us Press vs Repeat
// distinction so we can filter repeats for buttons while allowing them for knobs.
pub fn poll_input(timeout: Duration, ts: &mut TuiState) -> anyhow::Result<Vec<InputEvent>> {
    if !event::poll(timeout)? {
        return Ok(vec![]);
    }

    if let Event::Key(key) = event::read()? {
        return Ok(match key.kind {
            KeyEventKind::Press => handle_press(key.code, ts),
            KeyEventKind::Repeat => handle_repeat(key.code, ts),
            KeyEventKind::Release => handle_release(key.code, ts),
        });
    }
    Ok(vec![])
}

// ── First press ──────────────────────────────────────────────────

fn handle_press(code: KeyCode, ts: &mut TuiState) -> Vec<InputEvent> {
    match code {
        KeyCode::Esc => vec![InputEvent::Quit],
        KeyCode::Char(' ') => vec![InputEvent::PlayPress],

        // 4×4 grid pads
        KeyCode::Char(c) if is_pad_char(c) => {
            if let Some(n) = char_to_pad(c) {
                resolve_grid(n, ts)
            } else {
                vec![]
            }
        }

        // modifier toggles: press once = on, press again = off
        KeyCode::Char('g') => {
            ts.sound_held = !ts.sound_held;
            if ts.sound_held { vec![InputEvent::SoundDown] } else { vec![InputEvent::SoundUp] }
        }
        KeyCode::Char('h') => {
            ts.pattern_held = !ts.pattern_held;
            if ts.pattern_held { vec![InputEvent::PatternDown] } else { vec![InputEvent::PatternUp] }
        }
        KeyCode::Char('t') => {
            // Always send WriteDown — backend toggles write_mode on every WriteDown
            vec![InputEvent::WriteDown]
        }
        KeyCode::Char('b') => {
            ts.record_held = !ts.record_held;
            if ts.record_held { vec![InputEvent::RecordDown] } else { vec![InputEvent::RecordUp] }
        }
        KeyCode::Char('y') => {
            ts.fx_held = !ts.fx_held;
            if ts.fx_held { vec![InputEvent::FxDown] } else { vec![InputEvent::FxUp] }
        }
        KeyCode::Char('n') => {
            ts.bpm_held = !ts.bpm_held;
            if ts.bpm_held { vec![InputEvent::BpmDown] } else { vec![InputEvent::BpmUp] }
        }

        KeyCode::Char('0') => vec![InputEvent::ClearTrack],

        // knobs (also handled in handle_repeat for auto-repeat)
        KeyCode::Char('[') => resolve_knob_a(-0.05, ts),
        KeyCode::Char(']') => resolve_knob_a(0.05, ts),
        KeyCode::Char('-') => resolve_knob_b(-0.05, ts),
        KeyCode::Char('=') => resolve_knob_b(0.05, ts),

        _ => vec![],
    }
}

// ── Auto-repeat (held key) — only knobs repeat ──────────────────

fn handle_repeat(code: KeyCode, ts: &mut TuiState) -> Vec<InputEvent> {
    match code {
        KeyCode::Char('[') => resolve_knob_a(-0.05, ts),
        KeyCode::Char(']') => resolve_knob_a(0.05, ts),
        KeyCode::Char('-') => resolve_knob_b(-0.05, ts),
        KeyCode::Char('=') => resolve_knob_b(0.05, ts),
        _ => vec![], // all other keys: ignore repeats
    }
}

// ── Key release — only used to clear held_step for per-step editing ─

fn handle_release(code: KeyCode, ts: &mut TuiState) -> Vec<InputEvent> {
    if let KeyCode::Char(c) = code {
        if is_pad_char(c) {
            ts.held_step = None;
        }
    }
    vec![]
}

// ── Grid resolution ──────────────────────────────────────────────

fn resolve_grid(n: u8, ts: &mut TuiState) -> Vec<InputEvent> {
    if ts.sound_held {
        return vec![InputEvent::SelectSound(n)];
    }
    if ts.pattern_held {
        if ts.playing {
            return vec![InputEvent::ChainPattern(n)];
        } else {
            return vec![InputEvent::SelectPattern(n)];
        }
    }
    if ts.bpm_held {
        return vec![InputEvent::SetVolume(n + 1)];
    }
    if ts.fx_held && ts.playing {
        if n == 15 {
            return vec![InputEvent::ClearRealtimeEffect];
        } else {
            return vec![InputEvent::SetRealtimeEffect(n + 1)];
        }
    }
    if ts.record_held && ts.sound_held {
        return vec![InputEvent::DeleteSound];
    }
    if ts.write_mode && !ts.playing {
        // Write mode (stopped): toggle step AND track it for per-step knob editing
        ts.held_step = Some(n);
        return vec![InputEvent::ToggleStep(n)];
    }
    if ts.write_mode && ts.playing {
        // Write mode (playing): live record — quantize the note into the pattern
        return vec![InputEvent::LiveRecordStep(n)];
    }
    // default: trigger pad melodically
    vec![InputEvent::TriggerPad(n)]
}

// ── Knob resolution ──────────────────────────────────────────────

fn resolve_knob_a(delta: f32, ts: &TuiState) -> Vec<InputEvent> {
    if ts.bpm_held {
        return vec![InputEvent::AdjustSwing(delta)];
    }
    // Per-step pitch lock: holding a step pad in write mode (stopped) + knob A
    if let Some(step) = ts.held_step {
        if ts.write_mode && !ts.playing {
            return vec![InputEvent::LockStepPitchAt { step, delta }];
        }
    }
    if ts.write_mode && ts.playing {
        return vec![InputEvent::PitchLockStep(delta)];
    }
    match ts.param_page {
        ParamPage::Tone => vec![InputEvent::AdjustPitch(delta)],
        ParamPage::Filter => vec![InputEvent::AdjustFilterCutoff(delta)],
        ParamPage::Trim => vec![InputEvent::AdjustTrimStart(delta)],
    }
}

fn resolve_knob_b(delta: f32, ts: &TuiState) -> Vec<InputEvent> {
    if ts.bpm_held {
        return vec![InputEvent::AdjustBpm(delta)];
    }
    // Per-step gain lock: holding a step pad in write mode (stopped) + knob B
    if let Some(step) = ts.held_step {
        if ts.write_mode && !ts.playing {
            return vec![InputEvent::LockStepGainAt { step, delta }];
        }
    }
    if ts.write_mode && ts.playing {
        return vec![InputEvent::GainLockStep(delta)];
    }
    match ts.param_page {
        ParamPage::Tone => vec![InputEvent::AdjustGain(delta)],
        ParamPage::Filter => vec![InputEvent::AdjustFilterResonance(delta)],
        ParamPage::Trim => vec![InputEvent::AdjustTrimLength(delta)],
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn is_pad_char(c: char) -> bool {
    matches!(c, '1'..='4' | 'q' | 'w' | 'e' | 'r'
                | 'a' | 's' | 'd' | 'f'
                | 'z' | 'x' | 'c' | 'v')
}

fn char_to_pad(c: char) -> Option<u8> {
    let idx = match c {
        '1' => 0, '2' => 1, '3' => 2, '4' => 3,
        'q' => 4, 'w' => 5, 'e' => 6, 'r' => 7,
        'a' => 8, 's' => 9, 'd' => 10, 'f' => 11,
        'z' => 12, 'x' => 13, 'c' => 14, 'v' => 15,
        _ => return None,
    };
    Some(idx)
}
