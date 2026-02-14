mod shared;
mod tui;
mod audio_api;
mod audio;
mod loader;
mod middle;
mod pipeline;

use std::path::Path;
use crossterm::terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use shared::UiAction;
use audio_api::{AudioCommand, TriggerParams};

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

    // example of how to play it
    let (sample_id, buffer) = loader::sample_loader::load(Path::new("src/audio_samples/808CHH01.wav"), 44100)?;
    let length = buffer.data.len().min(44100);
    audio.send(AudioCommand::RegisterSample { id: sample_id, buffer });
    audio.send(AudioCommand::Trigger(TriggerParams {
        sample_id,
        trim_start: 0,
        length,
        gain: 1.0,
        pitch: 1.0,
        effect_chain: vec![],
    }));


    let mut pads_lit = [false; shared::NUM_PADS];
    loop {
        tui.draw(|frame| {
            tui::grid::draw_pad_grid(frame, frame.area(), &pads_lit);
        });
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