use crate::shared::DisplayState;
use ratatui::layout::{Layout, Direction, Constraint, Rect};
use ratatui::Frame;

const PAD_LABELS: [&str; 16] = [
   "1", "2", "3", "4",
   "Q", "W", "E", "R",
   "A", "S", "D", "F",
   "Z", "X", "C", "V",
];

pub fn render(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
   let sections = Layout::default()
       .direction(Direction::Vertical)
       .constraints([
           Constraint::Length(6), // lcd screen
           Constraint::Length(3), // mode buttons + knobs row
           Constraint::Min(12), // pad grid + side buttons
       ])
       .split(area);

   draw_screen(frame, sections[0], state);
   draw_mode_row(frame, sections[1], state);
   draw_keypad(frame, sections[2], state, blink_on);
}

fn draw_screen(frame: &mut Frame, area: Rect, state: &DisplayState) {
}

fn draw_mode_row(frame: &mut Frame, area: Rect, state: &DisplayState) {
}

fn draw_keypad(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
}