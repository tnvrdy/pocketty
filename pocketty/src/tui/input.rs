use crate::shared::{PadId, UiAction};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind}; // keyevent should handle all keypresses, 
                                                                // just importing event for future: terminal 
                                                                // resize, trackpad, etc.

pub fn read_action() -> anyhow::Result<UiAction> {
    loop { 
        match crossterm::event::read()? {
            KeyEvent(k) => {
                match k.code {
                    KeyCode::Esc => return Ok(UiAction::Quit),
                    KeyCode::Char(c) => {
                        let c = c.to_ascii_lowercase();
                        if let Some(pad) = char_to_pad(c) {
                            return Ok(UiAction::PadDown(pad)); // return paddown uiaction
                        }
                    }
                    _ => {}
                }
            }
            _ => {} 
        }
    }
}

fn char_to_pad(c: char) -> Option<PadId> {
    // keypad:
    // 1234
    //  qwer
    //   asdf
    //    zxcv
    let idx = match c { // match keypresses to track's array indices
        '1' => 0,
        '2' => 1,
        '3' => 2,
        '4' => 3,
        'q' => 4,
        'w' => 5,
        'e' => 6,
        'r' => 7,
        'a' => 8,
        's' => 9,
        'd' => 10,
        'f' => 11,
        'z' => 12,
        'x' => 13,
        'c' => 14,
        'v' => 15,
        _ => return None,
    };
    Some(PadId(idx as u8)) // cast to u8 to satisfy PadId type
}