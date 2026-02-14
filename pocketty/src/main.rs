mod shared;
mod tui;
mod audio_api;
mod audio;
mod middle;

use crossterm::terminal;
use shared::UiAction;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?; // should disable according to docs

    let audio = audio::start_audio()?;
    loop {
        let action = tui::input::read_action()?;
        match action {
            UiAction::Quit => break,
            _ => {
                if let Some(cmd) = middle::action_to_audio(action) {
                    audio.send(cmd);
                }
            }
        }
    }
    drop(audio);
    Ok(())
}