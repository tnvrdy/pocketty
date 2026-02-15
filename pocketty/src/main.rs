mod shared;
mod tui;
mod audio_api;
mod audio;
mod loader;
mod middle;
mod pipeline;

use std::path::PathBuf;
use std::time::Instant;
use crossterm::terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use middle::Middle;
use pipeline::persistence;
use shared::InputEvent;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard; // auto drops when out of scope
    let audio = audio::start_audio()?;
    let project_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let state = persistence::load_project(&project_dir)
        .unwrap_or_default();
    let mut middle = Middle::with_state(state);

    const SAMPLE_RATE: u32 = 44100;
    let wav_paths = loader::sample_loader::index_wav_in_dir(&project_dir)
        .unwrap_or_default();
    let num_loaded = wav_paths.len().min(shared::NUM_SOUNDS); // always refresh from disk
    for (slot, path) in wav_paths.into_iter().take(shared::NUM_SOUNDS).enumerate() {
        match middle.load_sample_into_slot(slot as u8, &path, SAMPLE_RATE) {
            Ok(cmd) => audio.send(cmd),
            Err(e) => eprintln!("Warning: could not load slot {} ({}): {}", slot, path.display(), e),
        }
    }
    for slot in num_loaded..shared::NUM_SOUNDS { // clear any samples removed from disk
        middle.clear_slot(slot as u8);
    }

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut term = Terminal::new(backend)?;
    term.clear()?;

    let tick_rate = std::time::Duration::from_millis(16); // ~60fps
    let mut last_tick = Instant::now();
    let blink_start = Instant::now();

    loop {
        let blink_on = (blink_start.elapsed().as_millis() / 250) % 2 == 0;
        let ds = middle.display_state().clone();
        term.draw(|frame| {
            tui::view::render(frame, frame.area(), &ds, blink_on);
        })?;
        
        if let Some(event) = tui::input::poll_input(tick_rate)? {
            if event == InputEvent::Quit {
                break;
            }
            let cmds = middle.handle_input(event);
            for cmd in cmds {
                audio.send(cmd);
            }
        }

        let elapsed = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();
        let cmds = middle.tick(elapsed);
        for cmd in cmds {
            audio.send(cmd);
        }
    }

    if let Err(e) = persistence::save_project(&project_dir, &middle.state) {
        eprintln!("Warning: could not save project: {}", e);
    }

    drop(term);
    drop(audio);
    Ok(())
}

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}