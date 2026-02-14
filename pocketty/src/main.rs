mod shared;
mod tui;
mod audio_api;
mod audio;
mod middle;
mod pipeline;

use crossterm::terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use shared::UiAction;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard; // auto drops when out of scope

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut tui = Terminal::new(backend)?;

    let audio = audio::start_audio()?;
    let mut pads_lit = [false; shared::NUM_PADS];
    loop {
        let action = tui::input::read_action()?;
        match action {
            UiAction::Quit => break,
            UiAction::PadDown(pad) => {
                pads_lit[pad.0 as usize] = true; // must index w usize not u8
                if let Some(cmd) = middle::action_to_audio(action) {
                    audio.send(cmd);
                }
            }
        }
        pads_lit.fill(false); // will remove for lit while held (would have to handle key-release)
    }
    drop(audio);
    Ok(())
}

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}