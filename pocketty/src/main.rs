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

    let tick_rate = std::time::Duration::from_millis(16);
    let mut last_tick = Instant::now();

    // This is the current idea of TUI <-> middle <-> audio interaction
    // Essentially, the main loop will poll the keyboard and send events
    // to and tick the middle/sequencer, and then it echoes any audio commands
    // to the audio engine. Then it renders the current display state.
    // The frame rate of our program is determined by the speed of this main loop.
    //
    // let mut middle = Middle::new();
    //
    // loop {
    //     // Input
    //     if let Some(event) = poll_input(tick_rate)? {
    //         if event == InputEvent::Quit { break; }
    //         let cmds = middle.handle_input(event);
    //         for cmd in cmds { audio.send(cmd); }
    //     }
    //
    //     // Middle
    //     let elapsed = last_tick.elapsed().as_secs_f64();
    //     last_tick = Instant::now();
    //     let cmds = middle.tick(elapsed);
    //     for cmd in cmds { audio.send(cmd); }
    //
    //     // TUI
    //     let ds = middle.display_state();
    //     // ... render the display based on the values in that struct (see shared.rs)
    // }

    // We don't have a TUI, so we'll just make a lazy input loop.
    loop {
        if crossterm::event::poll(tick_rate)? {
            use crossterm::event::{Event, KeyCode, KeyEventKind};
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    let event = match key.code {
                        // Right now you have to manually press Shift+<button> to release it. probably won't want this in the final version.
                        KeyCode::Esc => Some(InputEvent::Quit),
                        KeyCode::Char(' ') => Some(InputEvent::PlayPress),
                        KeyCode::Char('1') => Some(InputEvent::GridDown(0)),
                        KeyCode::Char('2') => Some(InputEvent::GridDown(1)),
                        KeyCode::Char('3') => Some(InputEvent::GridDown(2)),
                        KeyCode::Char('4') => Some(InputEvent::GridDown(3)),
                        KeyCode::Char('q') => Some(InputEvent::GridDown(4)),
                        KeyCode::Char('w') => Some(InputEvent::GridDown(5)),
                        KeyCode::Char('e') => Some(InputEvent::GridDown(6)),
                        KeyCode::Char('r') => Some(InputEvent::GridDown(7)),
                        KeyCode::Char('a') => Some(InputEvent::GridDown(8)),
                        KeyCode::Char('s') => Some(InputEvent::GridDown(9)),
                        KeyCode::Char('d') => Some(InputEvent::GridDown(10)),
                        KeyCode::Char('f') => Some(InputEvent::GridDown(11)),
                        KeyCode::Char('z') => Some(InputEvent::GridDown(12)),
                        KeyCode::Char('x') => Some(InputEvent::GridDown(13)),
                        KeyCode::Char('c') => Some(InputEvent::GridDown(14)),
                        KeyCode::Char('v') => Some(InputEvent::GridDown(15)),
                        KeyCode::Char('g') => Some(InputEvent::SoundDown),
                        KeyCode::Char('G') => Some(InputEvent::SoundUp),
                        KeyCode::Char('h') => Some(InputEvent::PatternDown),
                        KeyCode::Char('H') => Some(InputEvent::PatternUp),
                        KeyCode::Char('t') => Some(InputEvent::WriteDown),
                        KeyCode::Char('T') => Some(InputEvent::WriteUp),
                        KeyCode::Char('b') => Some(InputEvent::RecordDown),
                        KeyCode::Char('B') => Some(InputEvent::RecordUp),
                        KeyCode::Char('y') => Some(InputEvent::FxDown),
                        KeyCode::Char('Y') => Some(InputEvent::FxUp),
                        KeyCode::Char('n') => Some(InputEvent::BpmDown),
                        KeyCode::Char('N') => Some(InputEvent::BpmUp),
                        KeyCode::Char('[') => Some(InputEvent::KnobTurnA(-0.05)),
                        KeyCode::Char(']') => Some(InputEvent::KnobTurnA(0.05)),
                        KeyCode::Char('-') => Some(InputEvent::KnobTurnB(-0.05)),
                        KeyCode::Char('=') => Some(InputEvent::KnobTurnB(0.05)),
                        _ => None,
                    };

                    if let Some(ev) = event {
                        if ev == InputEvent::Quit {
                            break;
                        }
                        let cmds = middle.handle_input(ev);
                        for cmd in cmds {
                            audio.send(cmd);
                        }
                    }
                }
            }
        }

        let elapsed = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();
        let cmds = middle.tick(elapsed);
        for cmd in cmds {
            audio.send(cmd);
        }
        let _ds = middle.display_state();
    }

    if let Err(e) = persistence::save_project(&project_dir, &middle.state) {
        eprintln!("Warning: could not save project: {}", e);
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



// fn run() -> anyhow::Result<()> {
//     terminal::enable_raw_mode()?;
//     let _guard = RawModeGuard; // auto drops when out of scope

//     let backend = CrosstermBackend::new(std::io::stdout());
//     let mut tui = Terminal::new(backend)?;

//     let audio = audio::start_audio()?;

//     // example of how to play it
//     let (sample_id, buffer) = loader::sample_loader::load(Path::new("src/audio_samples/808CHH01.wav"), 44100)?;
//     let length = buffer.data.len().min(44100);
//     audio.send(AudioCommand::RegisterSample { id: sample_id, buffer });
//     audio.send(AudioCommand::Trigger(TriggerParams {
//         sample_id,
//         trim_start: 0,
//         length,
//         gain: 1.0,
//         pitch: 1.0,
//         effect_chain: vec![],
//     }));


//     let mut pads_lit = [false; shared::NUM_PADS];
//     loop {
//         tui.draw(|frame| {
//             tui::grid::draw_pad_grid(frame, frame.area(), &pads_lit);
//         });
//         let action = tui::input::read_action()?;
//         match action {
//             UiAction::Quit => break,
//             UiAction::PadDown(pad) => {
//                 pads_lit[pad.0 as usize] = true; // must index w usize not u8
//                 if let Some(cmd) = middle::action_to_audio(action) {
//                     audio.send(cmd);
//                 }
//             }
//         }
//         pads_lit.fill(false); // will remove for lit while held (would have to handle key-release)
//     }
//     drop(audio);
//     Ok(())
// }