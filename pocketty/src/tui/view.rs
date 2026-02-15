use crate::shared::DisplayState;
use ratatui::style::Color;
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