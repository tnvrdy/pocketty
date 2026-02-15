use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::shared::InputEvent;

pub fn poll_input(timeout: Duration) -> anyhow::Result<Option<InputEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(None);
        }
        let ev = match key.code {
            KeyCode::Esc => Some(InputEvent::Quit),
            KeyCode::Char(' ') => Some(InputEvent::PlayPress),
            KeyCode::Char('0') => Some(InputEvent::ClearTrack),

            // grid of a track (lowercase = down):
            // 1 2 3 4
            //  q w e r
            //   a s d f
            //    z x c v
            KeyCode::Char('1') => Some(InputEvent::GridDown(0)),
            KeyCode::Char('2') => Some(InputEvent::GridDown(1)),
            KeyCode::Char('3') => Some(InputEvent::GridDown(2)),
            KeyCode::Char('4') => Some(InputEvent::GridDown(3)),
            KeyCode::Char('q') => Some(InputEvent::GridDown(4)),
            KeyCode::Char('w') => Some(InputEvent::GridDown(5)),
            KeyCode::Char('e') => Some(InputEvent::GridDown(6)),
            KeyCode::Char('r') => Some(InputEvent::GridDown(7)),
            KeyCode::Char('a') => Some(InputEvent::GridDown(8)),
            KeyCode::Char('s') => Some(InputEvent::GridDown(9)),
            KeyCode::Char('d') => Some(InputEvent::GridDown(10)),
            KeyCode::Char('f') => Some(InputEvent::GridDown(11)),
            KeyCode::Char('z') => Some(InputEvent::GridDown(12)),
            KeyCode::Char('x') => Some(InputEvent::GridDown(13)),
            KeyCode::Char('c') => Some(InputEvent::GridDown(14)),
            KeyCode::Char('v') => Some(InputEvent::GridDown(15)),

            // modifier buttons (lowercase = down, shifted = up)
            KeyCode::Char('g') => Some(InputEvent::SoundDown),
            KeyCode::Char('G') => Some(InputEvent::SoundUp),
            KeyCode::Char('h') => Some(InputEvent::PatternDown),
            KeyCode::Char('H') => Some(InputEvent::PatternUp),
            KeyCode::Char('t') => Some(InputEvent::WriteDown),
            KeyCode::Char('T') => Some(InputEvent::WriteUp),
            KeyCode::Char('b') => Some(InputEvent::RecordDown),
            KeyCode::Char('B') => Some(InputEvent::RecordUp),
            KeyCode::Char('y') => Some(InputEvent::FxDown),
            KeyCode::Char('Y') => Some(InputEvent::FxUp),
            KeyCode::Char('n') => Some(InputEvent::BpmDown),
            KeyCode::Char('N') => Some(InputEvent::BpmUp),

            // knobs
            KeyCode::Char('[') => Some(InputEvent::KnobTurnA(-0.05)),
            KeyCode::Char(']') => Some(InputEvent::KnobTurnA(0.05)),
            KeyCode::Char('-') => Some(InputEvent::KnobTurnB(-0.05)),
            KeyCode::Char('=') => Some(InputEvent::KnobTurnB(0.05)),

            _ => None,
        };
        return Ok(ev);
    }

    Ok(None)
}
