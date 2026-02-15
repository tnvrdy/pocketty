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
    // Enable keyboard enhancement for real press/release detection.
    // Falls back gracefully if the terminal doesn't support it.
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    );
    let _guard = RawModeGuard; // auto drops when out of scope
    let audio = audio::start_audio()?;
    let project_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let state = persistence::load_project(&project_dir)
        .unwrap_or_default();
    // remember previously recorded samples
    let saved_paths: Vec<String> = state.sounds.iter()
        .map(|s| s.sample_path.clone())
        .collect();
    let mut middle = Middle::with_state(state);

    const SAMPLE_RATE: u32 = 44100;
    let wav_paths = loader::sample_loader::index_wav_in_dir(&project_dir)
        .unwrap_or_default();
    let num_loaded = wav_paths.len().min(shared::NUM_SOUNDS); // always refresh from disk
    for (slot, path) in wav_paths.into_iter().take(shared::NUM_SOUNDS).enumerate() {
        if let Ok(cmd) = middle.load_sample_into_slot(slot as u8, &path, SAMPLE_RATE) {
            audio.send(cmd);
        }
    }
    for slot in num_loaded..shared::NUM_SOUNDS { // clear any samples removed from disk
        middle.clear_slot(slot as u8);
    }

    for slot in 0..shared::NUM_SOUNDS {
        let sample_path = &saved_paths[slot];
        if sample_path.is_empty() {
            continue;
        }
        let path = std::path::Path::new(sample_path);
        let already_loaded = middle.state.sounds[slot].sample_id.is_some();
        if !already_loaded && path.exists() {
            if let Ok(cmd) = middle.load_sample_into_slot(slot as u8, path, SAMPLE_RATE) {
                audio.send(cmd);
            }
        }
    }

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut term = Terminal::new(backend)?;
    term.clear()?;

    let tick_rate = std::time::Duration::from_millis(16); // ~60fps
    let mut last_tick = Instant::now();
    let blink_start = Instant::now();
    let mut tui_state = tui::mode::TuiState::default();

    loop {
        // Always update blink and UI at the tick rate
        let blink_on = (blink_start.elapsed().as_millis() / 250) % 2 == 0;
        // Sync recording capture state from engine → middle → display
        middle.set_capturing(audio.is_capturing());
        let ds = middle.display_state().clone();

        tui_state.playing = ds.playing;
        tui_state.write_mode = ds.write_mode;
        tui_state.param_page = ds.param_page;

        term.draw(|frame| {
            tui::view::render(frame, frame.area(), &ds, blink_on);
        })?;

        let events = tui::input::poll_input(tick_rate, &mut tui_state)?;
        for event in events {
            if event == InputEvent::Quit {
                // save before quitting
                let _ = persistence::save_project(&project_dir, &middle.state);
                drop(term);
                drop(audio);
                return Ok(());
            }
            let cmds = middle.handle_input(event);
            for cmd in cmds {
                audio.send(cmd);
            }
        }

        // Check if a recording just finished; save the WAV to the project dir
        if let Some(rec) = audio.poll_completed_recording() {
            let _ = middle.on_recording_complete(rec.sample_id, &rec.buffer, &project_dir);
        }

        let elapsed = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();
        let cmds = middle.tick(elapsed);
        for cmd in cmds {
            audio.send(cmd);
        }
    }
    #[allow(unreachable_code)]
    Ok(())
}

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::PopKeyboardEnhancementFlags
        );
        let _ = terminal::disable_raw_mode();
    }
}
