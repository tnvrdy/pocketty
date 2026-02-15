use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::shared::{InputEvent, ParamPage};
use super::mode::TuiState;

// poll for input from tui, tracks state of key presses/holds in tuistate,
// resolves key combos to sequences of input events for the backend to handle
pub fn poll_input(timeout: Duration, ts: &mut TuiState) -> anyhow::Result<Vec<InputEvent>> {
    if !event::poll(timeout)? {
        return Ok(vec![]);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(vec![]);
        }
        return Ok(handle_key(key.code, ts));
    }
    Ok(vec![])
}

fn handle_key(code: KeyCode, ts: &mut TuiState) -> Vec<InputEvent> {
    match code {
        KeyCode::Esc => vec![InputEvent::Quit],
        KeyCode::Char(' ') => vec![InputEvent::PlayPress],

        // any keys on the 4x4 grid pad
        KeyCode::Char(c @ ('1' | '2' | '3' | '4'
            | 'q' | 'w' | 'e' | 'r'
            | 'a' | 's' | 'd' | 'f'
            | 'z' | 'x' | 'c' | 'v')) => {
            if let Some(n) = char_to_pad(c) {
                resolve_grid(n, ts)
            } else {
                vec![]
            }
        }

        // keys that modify params, lowercase = down and shifted = up
        KeyCode::Char('g') => { ts.sound_held = true; vec![InputEvent::SoundDown] }
        KeyCode::Char('G') => { ts.sound_held = false; vec![InputEvent::SoundUp] }
        KeyCode::Char('h') => { ts.pattern_held = true; vec![InputEvent::PatternDown] }
        KeyCode::Char('H') => { ts.pattern_held = false; vec![InputEvent::PatternUp] }
        KeyCode::Char('t') => { ts.write_held = true; vec![InputEvent::WriteDown] }
        KeyCode::Char('T') => { ts.write_held = false; vec![InputEvent::WriteUp] }
        KeyCode::Char('b') => { ts.record_held = true; vec![InputEvent::RecordDown] }
        KeyCode::Char('B') => { ts.record_held = false; vec![InputEvent::RecordUp] }
        KeyCode::Char('y') => { ts.fx_held = true; vec![InputEvent::FxDown] }
        KeyCode::Char('Y') => { ts.fx_held = false; vec![InputEvent::FxUp] }
        KeyCode::Char('n') => { ts.bpm_held = true; vec![InputEvent::BpmDown] }
        KeyCode::Char('N') => { ts.bpm_held = false; vec![InputEvent::BpmUp] }
        KeyCode::Char('0') => vec![InputEvent::ClearTrack],

        // knobs for more continuous control
        KeyCode::Char('[') => resolve_knob_a(-0.05, ts),
        KeyCode::Char(']') => resolve_knob_a(0.05, ts),
        KeyCode::Char('-') => resolve_knob_b(-0.05, ts),
        KeyCode::Char('=') => resolve_knob_b(0.05, ts),

        _ => vec![],
    }
}

// resolve grid keypresses into semantic inputevents based on held state
fn resolve_grid(n: u8, ts: &TuiState) -> Vec<InputEvent> {
    if ts.sound_held { // if in sound mode, select sound
        return vec![InputEvent::SelectSound(n)];
    }
    if ts.pattern_held { // if in pattern mode, select pattern
        if ts.playing {
            return vec![InputEvent::ChainPattern(n)];
        } else {
            return vec![InputEvent::SelectPattern(n)];
        }
    }
    if ts.bpm_held { // if holding bpm and pressing a pad, set volume
        return vec![InputEvent::SetVolume(n + 1)]; // volume 1-16
    }
    if ts.fx_held && ts.playing { // if holding fx and pressing a pad, set fx
        if n == 15 {
            return vec![InputEvent::ClearRealtimeEffect];
        } else {
            return vec![InputEvent::SetRealtimeEffect(n + 1)]; // fx 1-15
        }
    }
    if ts.record_held && ts.sound_held { // if holding record and sound and pressing a pad, delete sound
        return vec![InputEvent::DeleteSound];
    }
    if ts.write_mode && !ts.playing { // if in write mode, toggle step
        return vec![InputEvent::ToggleStep(n)];
    }
    if ts.write_held && ts.playing { // if in live record mode, record step
        return vec![InputEvent::LiveRecordStep(n)];
    }
    // by default, trigger pad melodically
    vec![InputEvent::TriggerPad(n)]
}

// resolve knob a turn into a semantic event based on held state + param page
fn resolve_knob_a(delta: f32, ts: &TuiState) -> Vec<InputEvent> {
    if ts.bpm_held { // if using knob to adjust swing
        return vec![InputEvent::AdjustSwing(delta) ];
    }
    if ts.write_held && ts.playing { // if using knob to adjust pitch locking
        return vec![InputEvent::PitchLockStep(delta)];
    }
    match ts.param_page { // if using knob to adjust {tone, filter, trim} params
        ParamPage::Tone => vec![InputEvent::AdjustPitch(delta)],
        ParamPage::Filter => vec![InputEvent::AdjustFilterCutoff(delta)],
        ParamPage::Trim => vec![InputEvent::AdjustTrimStart(delta)],
    }
}

// resolve knob b turn into a semantic event based on held state + param page
fn resolve_knob_b(delta: f32, ts: &TuiState) -> Vec<InputEvent> {
    if ts.bpm_held { // if using knob to adjust bpm
        return vec![InputEvent::AdjustBpm(delta)];
    }
    if ts.write_held && ts.playing { // if using knob to adjust gain locking
        return vec![InputEvent::GainLockStep(delta)];
    }
    match ts.param_page { // if using knob to adjust {tone, filter, trim} params
        ParamPage::Tone => vec![InputEvent::AdjustGain(delta)],
        ParamPage::Filter => vec![InputEvent::AdjustFilterResonance(delta)],
        ParamPage::Trim => vec![InputEvent::AdjustTrimLength(delta)],
    }
}

// convert char to pad index
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
