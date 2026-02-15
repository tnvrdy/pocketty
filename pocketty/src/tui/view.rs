use crate::shared::{DisplayState, LedState};
use ratatui::layout::{Alignment, Layout, Direction, Constraint, Rect};
use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const DIM: Color = Color::Rgb(60, 60, 60);
const MID: Color = Color::Rgb(120, 110, 90);
const TEXT: Color = Color::Rgb(200, 200, 180);
const ACCENT: Color = Color::Rgb(255, 200, 80);
const LED_MED: Color = Color::Rgb(180, 140, 50);
const LED_HI: Color = Color::Rgb(255, 220, 80);
const LCD_BG: Color = Color::Rgb(30, 35, 25);
const LCD_FG: Color = Color::Rgb(140, 180, 100);
const LCD_BRIGHT: Color = Color::Rgb(190, 230, 130);

const PAD_LABELS: [&str; 16] = [
    "1", "2", "3", "4",
    "Q", "W", "E", "R",
    "A", "S", "D", "F",
    "Z", "X", "C", "V",
];

const DEVICE_W: u16 = 46;

pub fn render(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
    let h_center = Layout::default() // horizontal center
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(DEVICE_W),
            Constraint::Min(0),
        ])
        .split(area);
    let device = h_center[1];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // lcd screen
            Constraint::Length(3),  // mode buttons (sound, pattern, bpm)
            Constraint::Length(2),  // knob indicators
            Constraint::Min(14),   // pad grid + side buttons
            Constraint::Length(1),  // footer
        ])
        .split(device);

    draw_screen(frame, rows[0], state);
    draw_mode_row(frame, rows[1], state);
    draw_knob_row(frame, rows[2], state);
    draw_pad_grid(frame, rows[3], state, blink_on);
    draw_footer(frame, rows[4]);
}

fn draw_screen(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let w = area.width as usize;
    let inner = if w > 4 { w - 4 } else { w };

    let border = Style::default().fg(MID);
    let lcd = Style::default().fg(LCD_FG);
    let lcd_b = Style::default().fg(LCD_BRIGHT);

    let top = format!(" ╔{}╗", "═".repeat(inner));
    let bot = format!(" ╚{}╝", "═".repeat(inner));

    let play_icon = if state.playing { "▶" } else { "■" };
    let write_icon = if state.write_mode { "●W" } else { "○W" };
    let line1_content = format!(
        " {:<12} snd:{:<2} pat:{:<2}",
        state.display_text,
        state.selected_sound + 1,
        state.selected_pattern + 1,
    );

    let page_name = format!("{:?}", state.param_page);
    let line2_content = format!(
        " {:<6} {}:{:.2}  {}:{:.2}",
        page_name,
        state.knob_a_label, state.knob_a_value,
        state.knob_b_label, state.knob_b_value,
    );

    let line3_content = format!(
        " {} PLAY   {} WRITE   {:.0} BPM",
        play_icon, write_icon, state.bpm
    );

    let pad_line = |content: &str| -> String {
        let visible = content.chars().count();
        let padding = if inner > visible { inner - visible } else { 0 };
        format!(" ║{}{}║", content, " ".repeat(padding))
    };

    let lines = vec![
        Line::from(Span::styled(top, border)),
        Line::from(vec![
            Span::styled(pad_line(&line1_content), border),
        ]),
        Line::from(vec![
            Span::styled(pad_line(&line2_content), lcd),
        ]),
        Line::from(vec![
            Span::styled(pad_line(&line3_content), lcd_b),
        ]),
        Line::from(Span::styled(bot, border)),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_mode_row(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    // infer held state from display_text prefix
    let sound_held = state.display_text.starts_with("SND");
    let pattern_held = state.display_text.starts_with("PAT");
    let bpm_held = state.display_text.starts_with("VOL");

    draw_mode_button(frame, cols[0], "sound", "g", sound_held);
    draw_mode_button(frame, cols[1], "pattern", "h", pattern_held);
    draw_mode_button(frame, cols[2], "bpm", "n", bpm_held);
}

fn draw_mode_button(frame: &mut Frame, area: Rect, label: &str, key: &str, held: bool) {
    let sym = if held { "◉" } else { "●" };
    let sym_color = if held { ACCENT } else { MID };
    let lbl_color = if held { ACCENT } else { TEXT };

    let lines = vec![
        Line::from(vec![
            Span::styled("  (", Style::default().fg(DIM)),
            Span::styled(sym, Style::default().fg(sym_color)),
            Span::styled(") ", Style::default().fg(DIM)),
            Span::styled(label, Style::default().fg(lbl_color)),
        ]),
        Line::from(vec![
            Span::styled(format!("   ({})", key), Style::default().fg(DIM)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_knob_row(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ])
        .split(area);

    draw_knob_inline(frame, cols[0], "A", state.knob_a_label, "[ / ]");
    draw_knob_inline(frame, cols[1], "B", state.knob_b_label, "- / =");
}

fn draw_knob_inline(frame: &mut Frame, area: Rect, id: &str, label: &str, keys: &str) {
    let lines = vec![
        Line::from(vec![
            Span::styled("  [", Style::default().fg(DIM)),
            Span::styled("◉", Style::default().fg(TEXT)),
            Span::styled("] ", Style::default().fg(DIM)),
            Span::styled(id, Style::default().fg(ACCENT)),
            Span::styled(" ", Style::default()),
            Span::styled(label, Style::default().fg(TEXT)),
            Span::styled("  ", Style::default()),
            Span::styled(keys, Style::default().fg(DIM)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_pad_grid(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5), // pad col 0
            Constraint::Ratio(1, 5), // pad col 1
            Constraint::Ratio(1, 5), // pad col 2
            Constraint::Ratio(1, 5), // pad col 3
            Constraint::Ratio(1, 5), // side buttons
        ])
        .split(area);

    for col in 0..4 {
        draw_pad_column(frame, cols[col], col, state, blink_on);
    }

    draw_side_buttons(frame, cols[4], state);
}

fn draw_pad_column(frame: &mut Frame, area: Rect, col: usize, state: &DisplayState, blink_on: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
        ])
        .split(area);

    for row in 0..4 {
        let idx = row * 4 + col;
        draw_pad_cell(frame, rows[row], idx, state, blink_on);
    }
}

fn draw_pad_cell(frame: &mut Frame, area: Rect, idx: usize, state: &DisplayState, blink_on: bool) {
    if idx >= 16 { return; }

    let led = state.leds[idx];
    let label = PAD_LABELS[idx];

    let (led_sym, led_color) = led_symbol(led, blink_on);
    let pad_color = match led {
        LedState::Off => DIM,
        LedState::OnMedium => LED_MED,
        LedState::OnHigh => LED_HI,
        LedState::Blink => if blink_on { LED_HI } else { DIM },
    };

    let lines = vec![
        Line::from(Span::styled( // led indicator line
            format!("  {}  ", led_sym),
            Style::default().fg(led_color),
        )),
        Line::from(Span::styled(" .:::. ", Style::default().fg(pad_color))), // pad top
        Line::from(vec![
            Span::styled(" : ", Style::default().fg(pad_color)),
            Span::styled(label, Style::default().fg(if led == LedState::Off { TEXT } else { ACCENT })
                .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" : ", Style::default().fg(pad_color)),
        ]),
        Line::from(Span::styled(" ':::' ", Style::default().fg(pad_color))), // pad bottom
        Line::from(Span::styled(" ':::' ", Style::default().fg(pad_color))),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn led_symbol(led: LedState, blink_on: bool) -> (&'static str, Color) {
    match led {
        LedState::Off => ("○", DIM),
        LedState::OnMedium => ("●", LED_MED),
        LedState::OnHigh => ("◉", LED_HI),
        LedState::Blink => {
            if blink_on { ("●", LED_HI) } else { ("○", DIM) }
        }
    }
}

fn draw_side_buttons(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
        ])
        .split(area);

    draw_side_button(frame, rows[0], "●", "rec", "b", false); // record button

    draw_side_button(frame, rows[1], "●", "fx", "y", false); // fx button

    let play_sym = if state.playing { "▶" } else { "■" };
    draw_side_button(frame, rows[2], play_sym, "play", "spc", state.playing); // play/stop button

    let write_sym = if state.write_mode { "●" } else { "○" };
    draw_side_button(frame, rows[3], write_sym, "write", "t", state.write_mode); // write button
}

fn draw_side_button(frame: &mut Frame, area: Rect, symbol: &str, label: &str, key: &str, active: bool) {
    let sym_color = if active { ACCENT } else { MID };
    let lbl_color = if active { ACCENT } else { TEXT };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" (", Style::default().fg(DIM)),
            Span::styled(symbol, Style::default().fg(sym_color)),
            Span::styled(") ", Style::default().fg(DIM)),
            Span::styled(label, Style::default().fg(lbl_color)),
        ]),
        Line::from(vec![
            Span::styled(format!("  ({})", key), Style::default().fg(DIM)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("  (esc)", Style::default().fg(DIM)),
        Span::styled(" quit", Style::default().fg(MID)),
        Span::styled("   (0)", Style::default().fg(DIM)),
        Span::styled(" clear", Style::default().fg(MID)),
        Span::styled("   (y)", Style::default().fg(DIM)),
        Span::styled(" fx/page", Style::default().fg(MID)),
    ]);

    frame.render_widget(Paragraph::new(vec![line]).alignment(Alignment::Center), area);
}
