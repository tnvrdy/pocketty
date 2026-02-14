use crate::shared::NUM_PADS;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::Frame;

const COLS: usize = 4;
const ROWS: usize = 4;

pub fn draw_pad_grid(frame: &mut Frame, area: Rect, pads_lit: &[bool; NUM_PADS]) {
    let row_constraints = [Constraint::Percentage(25); ROWS];
    let col_constraints = [Constraint::Percentage(25); COLS];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (row_idx, row_area) in rows.iter().enumerate() {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_area);

        for (col_idx, cell_area) in cols.iter().enumerate() {
            let pad_idx = row_idx * COLS + col_idx;
            let lit = pads_lit[pad_idx];
            let color = if lit {
                Style::default().fg(Color::LightMagenta).bg(Color::Magenta)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let block = Block::default()
                .border_style(color)
                .style(color);
            frame.render_widget(block, *cell_area);
        }
    }
}