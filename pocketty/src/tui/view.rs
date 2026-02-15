use crate::shared::{DisplayState, LedState, RecordingDisplay};
use ratatui::layout::{Alignment, Layout, Direction, Constraint, Rect};
use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, BorderType, Paragraph};
use ratatui::Frame;

const DIM: Color = Color::Rgb(200, 175, 188);   // muted baby pink
const MID: Color = Color::Rgb(220, 190, 200);   // muted baby pink
const TEXT: Color = Color::Rgb(255, 220, 230);   // baby pink
const ACCENT: Color = Color::Rgb(255, 230, 238); // baby pink (brighter, active)
const LCD_FG: Color = Color::Rgb(230, 200, 210);
const LCD_BRIGHT: Color = Color::Rgb(255, 225, 235);
const LED_MED: Color = Color::Rgb(220, 55, 50);
const LED_HI: Color = Color::Rgb(240, 50, 50);
const LED_RED: Color = Color::Rgb(255, 50, 50); // bright red when button is active

const PAD_LABELS: [&str; 16] = [
    "1", "2", "3", "4",
    "Q", "W", "E", "R",
    "A", "S", "D", "F",
    "Z", "X", "C", "V",
];

// terminal chars are ~2:1 so 40×41 chars ≈ 40×82 visual
const DEVICE_W: u16 = 40;
const DEVICE_H: u16 = 41;

pub fn render(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
    // Center device in terminal
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(DEVICE_W),
            Constraint::Min(0),
        ])
        .split(area);

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(DEVICE_H),
            Constraint::Min(0),
        ])
        .split(h[1]);

    let device_area = v[1];

    let border = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MID));

    let inner = border.inner(device_area);
    frame.render_widget(border, device_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // gap between top border and LCD
            Constraint::Length(13), // LCD screen (art)
            Constraint::Length(5),  // controls: buttons + knobs
            Constraint::Length(1),  // spacer (gap before grid)
            Constraint::Length(16), // pad grid: 4 rows × 4 lines each
            Constraint::Length(1),  // footer
        ])
        .split(inner);

    draw_title_gap(frame, rows[0]);
    draw_screen(frame, rows[1], state);
    draw_controls_row(frame, rows[2], state);
    // rows[3] = spacer — intentionally blank
    draw_pad_area(frame, rows[4], state, blink_on);
    draw_footer(frame, rows[5]);
}

fn draw_title_gap(frame: &mut Frame, area: Rect) {
    let line = Line::from(Span::styled(
        "pocketty ─ PO-33",
        Style::default().fg(TEXT),
    ));
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        area,
    );
}

const LCD_ART: &str = r#"
⠀⠀⣰⡿⠿⠿⢿⠆⠀⣴⡄⢀⣀⠀⠀⢀⣄⣀⠀⠀⠀⠀⠀⣀⡀⠀⠀⡊⠲⠉⢱⠀⠀⠀⢠⣤⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⢀⣿⣇⠀⠂⣌⠆⠀⠹⣿⠿⠛⠁⢹⠋⢉⡉⢳⣦⠀⠀⠀⢿⡿⠀⠀⠈⢀⣖⠁⠀⠀⠀⠈⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠈⠛⡻⢠⡴⠋⠢⡀⠀⠀⠀⠐⠯⣛⠀⠂⠀⢀⠄⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⡐⣹⣦⡟⡔⡑⠁⠀⠶⢠⠂⡄⠹⢞⠀⡔⠋⠀⠀⠀⠀⠀⠀⣀⣴⣿⣿⣿⣿⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⢱⣦⣛⠴⠂⠀⠠⠘⠒⠋⢰⢷⣲⠉⡉⠉⡆⠀⠀⠀⢴⡶⠀⠉⠉⢁⣼⣿⣯⠉⠉⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⢨⣿⣿⣄⠀⠀⠀⠉⣍⣈⣉⠒⡚⠒⠰⠁⡇⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣿⣿⣤⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⣿⡿⣿⠿⠦⠀⠀⠀⠀⠀⠀⠀⠘⣁⣁⡤⢷⠀⠀⣶⠄⠀⠀⠀⣉⣽⣿⣿⣿⣍⠙⠉⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠈⠁⠂⢱⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⠀⢀⣆⣞⣲⠒⡄⠀⠐⠶⢿⣿⣿⣿⣿⣿⣿⣷⣤⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⢧⣾⡶⠀⠀⠀⠀⠀⠀⠀⠀⠸⠤⢀⡀⣀⢸⣉⠇⠀⠀⠀⠀⠀⠈⡍⠈⠈⡇⠁⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠀⠀⠀⠀⠀⠀⠀⠀⠉⠉⠉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"#;

fn draw_screen(frame: &mut Frame, area: Rect, _state: &DisplayState) {
    let h = area.height as usize;
    let w = area.width as usize;
    let style = Style::default().fg(LCD_FG);
    let border_style = Style::default().fg(MID);

    let content_h = h.saturating_sub(2);
    let content_w = w.saturating_sub(2);
    if content_h == 0 || content_w == 0 {
        return;
    }

    let art_lines: Vec<&str> = LCD_ART.trim().lines().collect();
    let start = art_lines
        .iter()
        .position(|s| !s.chars().all(|c| c.is_whitespace()))
        .unwrap_or(0);
    let art_used: Vec<&str> = art_lines.get(start..).unwrap_or(&[]).to_vec();

    let top_border = format!("╔{}╗", "═".repeat(content_w));
    let bottom_border = format!("╚{}╝", "═".repeat(content_w));

    let mut lines = vec![Line::from(Span::styled(top_border.clone(), border_style))];

    for s in art_used.iter().take(content_h) {
        let raw: String = s.chars().take(content_w).collect();
        let pad_len = content_w.saturating_sub(raw.chars().count());
        let row = format!("{}{}", raw, " ".repeat(pad_len));
        lines.push(Line::from(vec![
            Span::styled("║", border_style),
            Span::styled(row, style),
            Span::styled("║", border_style),
        ]));
    }

    for _ in art_used.len().min(content_h)..content_h {
        lines.push(Line::from(vec![
            Span::styled("║", border_style),
            Span::styled(" ".repeat(content_w), style),
            Span::styled("║", border_style),
        ]));
    }

    lines.push(Line::from(Span::styled(bottom_border, border_style)));

    let para = Paragraph::new(lines).scroll((0, 0));
    frame.render_widget(para, area);
}

fn draw_controls_row(frame: &mut Frame, area: Rect, state: &DisplayState) {
    let inner_w = area.width;
    let block_w = 35u16;
    let side = inner_w.saturating_sub(block_w) / 2;
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(side),
            Constraint::Length(block_w),
            Constraint::Min(0),
        ])
        .split(area);
    let centered = h[1];

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(7); 5])
        .split(centered);


    let sh = state.display_text.starts_with("SND");
    let ph = state.display_text.starts_with("PAT");
    let bh = state.display_text.starts_with("VOL");

    draw_btn(frame, cols[0], "●", "sound", "g", sh);
    draw_btn(frame, cols[1], "●", "pattn", "h", ph);
    draw_btn(frame, cols[2], "●", "bpm", "n", bh);
    draw_knob(frame, cols[3], state.knob_a_label, "[/]", state.knob_a_value);
    draw_knob(frame, cols[4], state.knob_b_label, "-/=", state.knob_b_value);
}

/// dot lights up red when the button is active.
fn draw_btn(frame: &mut Frame, area: Rect, sym: &str, label: &str, key: &str, active: bool) {
    let c = if active { ACCENT } else { DIM };
    let sc = if active { LED_RED } else { MID }; // dot red when lit
    let lc = if active { ACCENT } else { TEXT };

    let lines = vec![
        Line::from(Span::styled(".:::.", Style::default().fg(c))),
        Line::from(vec![
            Span::styled(": ", Style::default().fg(c)),
            Span::styled(sym, Style::default().fg(sc)),
            Span::styled(" :", Style::default().fg(c)),
        ]),
        Line::from(Span::styled("':::'", Style::default().fg(c))),
        Line::from(Span::styled(label, Style::default().fg(lc))),
        Line::from(Span::styled(format!("({})", key), Style::default().fg(DIM))),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn draw_knob(frame: &mut Frame, area: Rect, label: &str, keys: &str, value: f32) {
    let (top, mid, bot) = knob_art(value);

    let lines = vec![
        Line::from(Span::styled(top, Style::default().fg(TEXT))),
        Line::from(Span::styled(mid, Style::default().fg(ACCENT))),
        Line::from(Span::styled(bot, Style::default().fg(TEXT))),
        Line::from(Span::styled(label, Style::default().fg(TEXT))),
        Line::from(Span::styled(format!("({})", keys), Style::default().fg(DIM))),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn knob_art(value: f32) -> (&'static str, &'static str, &'static str) {
    let pos = ((value.clamp(0.0, 1.0) * 8.0) as usize) % 8;
    match pos {
        0 => ("╭─·─╮", "│   │", "╰─·─╯"), // 12/6 o'clock
        1 => ("╭──·╮", "│ ╲ │", "╰·──╯"), //  1:30 / 7:30
        2 => ("╭───╮", "·───·", "╰───╯"), //  3/9 o'clock
        3 => ("╭·──╮", "│ ╱ │", "╰──·╯"), //  4:30 / 10:30
        4 => ("╭─·─╮", "│   │", "╰─·─╯"), //  6/12 o'clock
        5 => ("╭──·╮", "│ ╲ │", "╰·──╯"), //  7:30 / 1:30
        6 => ("╭───╮", "·───·", "╰───╯"), //  9/3 o'clock
        7 => ("╭·──╮", "│ ╱ │", "╰──·╯"), // 10:30 / 4:30
        _ => ("╭───╮", "│ ● │", "╰───╯"), // fallback
    }
}

fn draw_pad_area(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
    let inner_w = area.width;
    let block_w = 35u16;
    let side = inner_w.saturating_sub(block_w) / 2;
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(side),
            Constraint::Length(block_w),
            Constraint::Min(0),
        ])
        .split(area);
    let centered = h[1];

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(7); 5])
        .split(centered);

    for c in 0..4 {
        draw_pad_col(frame, cols[c], c, state, blink_on);
    }
    draw_side_col(frame, cols[4], state, blink_on);
}

fn draw_pad_col(frame: &mut Frame, area: Rect, col: usize, state: &DisplayState, blink_on: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4); 4])
        .split(area);

    for row in 0..4 {
        draw_pad(frame, rows[row], row * 4 + col, state, blink_on);
    }
}

fn draw_pad(frame: &mut Frame, area: Rect, idx: usize, state: &DisplayState, blink_on: bool) {
    if idx >= 16 { return; }

    let led = state.leds[idx];
    let label = PAD_LABELS[idx];
    let (led_sym, led_c) = led_symbol(led, blink_on);
    let pad_c = pad_color(led, blink_on);
    let lbl_c = if led == LedState::Off { TEXT } else { ACCENT };

    let lines = vec![
        Line::from(Span::styled(led_sym, Style::default().fg(led_c))),
        Line::from(Span::styled(".:::.", Style::default().fg(pad_c))),
        Line::from(vec![
            Span::styled(": ", Style::default().fg(pad_c)),
            Span::styled(label, Style::default().fg(lbl_c).add_modifier(Modifier::BOLD)),
            Span::styled(" :", Style::default().fg(pad_c)),
        ]),
        Line::from(Span::styled("':::'", Style::default().fg(pad_c))),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn led_symbol(led: LedState, blink_on: bool) -> (&'static str, Color) {
    match led {
        LedState::Off => ("○", DIM),
        LedState::OnMedium => ("●", LED_RED),
        LedState::OnHigh => ("◉", LED_RED),
        LedState::Blink => if blink_on { ("●", LED_RED) } else { ("○", DIM) },
    }
}

fn pad_color(led: LedState, blink_on: bool) -> Color {
    match led {
        LedState::Off => DIM,
        LedState::OnMedium => LED_MED,
        LedState::OnHigh => LED_HI,
        LedState::Blink => if blink_on { LED_HI } else { DIM },
    }
}

fn draw_side_col(frame: &mut Frame, area: Rect, state: &DisplayState, blink_on: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4); 4])
        .split(area);

    // Record button: steady when armed, blink when capturing
    let (rec_active, rec_sym) = match state.recording {
        RecordingDisplay::Idle => (false, "○"),
        RecordingDisplay::Armed => (true, "●"),
        RecordingDisplay::Capturing => (blink_on, if blink_on { "●" } else { "○" }),
    };
    draw_side_btn(frame, rows[0], rec_sym, "rec", "b", rec_active);
    draw_side_btn(frame, rows[1], "●", "fx", "y", false);

    let ps = if state.playing { "▶" } else { "■" };
    draw_side_btn(frame, rows[2], ps, "play", "_", state.playing);

    let ws = if state.write_mode { "●" } else { "○" };
    draw_side_btn(frame, rows[3], ws, "write", "t", state.write_mode);
}

fn draw_side_btn(frame: &mut Frame, area: Rect, sym: &str, label: &str, key: &str, active: bool) {
    let c = if active { ACCENT } else { DIM };
    let sc = if active { LED_RED } else { MID }; // dot red when lit
    let lc = if active { ACCENT } else { TEXT };

    let lines = vec![
        Line::from(Span::styled(".:::.", Style::default().fg(c))),
        Line::from(vec![
            Span::styled(": ", Style::default().fg(c)),
            Span::styled(sym, Style::default().fg(sc)),
            Span::styled(" :", Style::default().fg(c)),
        ]),
        Line::from(Span::styled("':::'", Style::default().fg(c))),
        Line::from(vec![
            Span::styled(format!("{} ", label), Style::default().fg(lc)),
            Span::styled(format!("({})", key), Style::default().fg(DIM)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("(esc)", Style::default().fg(DIM)),
        Span::styled("quit ", Style::default().fg(MID)),
        Span::styled("(0)", Style::default().fg(DIM)),
        Span::styled("clr ", Style::default().fg(MID)),
        Span::styled("(i)", Style::default().fg(DIM)),
        Span::styled("in ", Style::default().fg(MID)),
        Span::styled("(p)", Style::default().fg(DIM)),
        Span::styled("wav", Style::default().fg(MID)),
    ]);

    frame.render_widget(
        Paragraph::new(vec![line]).alignment(Alignment::Center),
        area,
    );
}
