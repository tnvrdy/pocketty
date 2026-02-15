use crate::shared::{DisplayState, LedState, STEPS_PER_PATTERN};
use ratatui::style::Color;
use ratatui::layout::{Layout, Direction, Constraint, Rect};
use ratatui::Frame;

const BG: Color = Color::Black;
const DARK: Color = Color::Rgb(60, 60, 60);
const MED: Color = Color::Rgb(200, 140, 50);
const HIGH: Color = Color::Rgb(255, 200, 80);
const BLINK_ON: Color = Color::Rgb(255, 80, 40);
const TEXT: Color = Color::Rgb(200, 200, 180);
const BRIGHT: Color = Color::Rgb(255, 200, 80);

const PAD_LABELS: [&str; 16] = [
    "1", "2", "3", "4",
    "Q", "W", "E", "R",
    "A", "S", "D", "F",
    "Z", "X", "C", "V",
];

// main renderer
pub fn render(frame: &mut Frame, ds: &DisplayState, blink_on: bool) {
    // take in frame, display state, and blink on/off
    // chunk into grid
    // draw status screen
    // draw pad grid
    // draw knobs
}

fn draw_pad_grid(frame: &mut Frame, area: Rect, pads_lit: &[LedState; STEPS_PER_PATTERN], blink_on: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1,4); 4])
        .split(area);

    for row in 0..4 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1,4); 4])
            .split(rows[row]);

        for col in 0..4 {
            let pad_i = row * 4 + col;
            let lit = pads_lit[pad_i];

            let (fg, bg) = match lit {
                LedState::Off => (DARK, BG),
                LedState::OnMedium => (MED, BG),
                LedState::OnHigh => (HIGH, BG),
                LedState::Blink => {
                    if blink_on {
                        (BG, BLINK_ON)
                    } else {
                        (DARK, BG)
                    }
                },
            };
        }
    }
}