//! Prince Toad: a prince turned toad crosses road after road, looking for
//! a kiss on the other side.

mod audio;
mod endless;
mod game;
mod input;
mod levels;
mod render;
mod save;
mod solver;
mod world;

use game::{Game, Phase, Start};
use input::Input;
use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use world::DT;

const FRAME: Duration = Duration::from_micros(16_667);
const SNAPSHOT_EVERY: Duration = Duration::from_secs(1);

const HELP: &str = "prince-toad [options]

A prince has been turned into a toad. Only a kiss can turn him back, and
whoever he might kiss is always on the other side of the road.

  --endless         straight onto the endless road
  --two             two players at one keyboard (W A S D against the arrows)
  --crossing N      start the story at crossing N (1-19; 19 is the pond)
  --demo            watch the toad cross the story by itself (with
                    --crossing, from there)
  --mute            no sound
  --version

In the game, H shows the controls. Progress and settings are kept in
$XDG_STATE_HOME/prince-toad/save.txt (~/.local/state/...), with the
session log beside it (level from RUST_LOG, default info).";

/// Session log in the state folder; the terminal is the game's.
fn init_logging() -> Option<PathBuf> {
    let path = save::state_dir()?.join("session.log");
    let file = std::fs::File::create(&path).ok()?;
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_env("RUST_LOG")
        .target(env_logger::Target::Pipe(Box::new(file)))
        .format(|buf, rec| {
            writeln!(
                buf,
                "{} {:<5} {}",
                buf.timestamp_millis(),
                rec.level(),
                rec.args()
            )
        })
        .init();
    Some(path)
}

struct Args {
    start: Start,
    mute: bool,
}

fn parse_args() -> Result<Option<Args>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut a = Args {
        start: Start::Title,
        mute: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(None);
            }
            "--version" => {
                println!("{}", game::version_string());
                return Ok(None);
            }
            "--endless" => a.start = Start::Endless,
            "--two" => a.start = Start::Versus,
            "--demo" => {
                a.start = match a.start {
                    Start::Story(n) => Start::Demo(n),
                    _ => Start::Demo(0),
                }
            }
            "--mute" => a.mute = true,
            "--crossing" => {
                let v = args.get(i + 1).ok_or("--crossing needs a number")?;
                let n: usize = v
                    .parse()
                    .map_err(|_| format!("--crossing {v}: not a number"))?;
                if !(1..=levels::POND + 1).contains(&n) {
                    return Err(format!("--crossing {n}: try 1 to {}", levels::POND + 1));
                }
                a.start = match a.start {
                    Start::Demo(_) => Start::Demo(n - 1),
                    _ => Start::Story(n - 1),
                };
                i += 1;
            }
            other => return Err(format!("unknown option {other} (try --help)")),
        }
        i += 1;
    }
    Ok(Some(a))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("prince-toad: {e}");
            return ExitCode::from(2);
        }
    };
    let log_path = init_logging();
    log::info!("{} starting", game::version_string());
    let save = save::Save::load();
    let demo = matches!(args.start, Start::Demo(_));
    // open audio before the alternate screen, so any ALSA chatter lands above it
    let mut audio = audio::Audio::new(args.mute);
    if save.muted && !args.mute {
        log::info!("sound is off in the saved settings");
        audio.muted = true;
    }
    log::info!(
        "audio available {} muted {}",
        audio.available(),
        audio.muted
    );
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        );
    }
    log::info!("kitty keyboard protocol: {kitty}");
    let mut input = Input::new();
    input.open_pad();
    let mut game = Game::new(args.start, save);
    game.demo = demo;
    game.muted = audio.muted;
    let result = run(&mut terminal, &mut game, &mut input, &mut audio);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            eprintln!("prince-toad: {e}");
            if let Some(p) = log_path {
                eprintln!("session log: {}", p.display());
            }
            ExitCode::FAILURE
        }
    }
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    game: &mut Game,
    input: &mut Input,
    audio: &mut audio::Audio,
) -> io::Result<()> {
    let mut last = Instant::now();
    let mut acc = 0.0f32;
    let mut snapshot = Instant::now();
    loop {
        let frame_start = Instant::now();
        input.poll_pad();
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(k) => {
                    log::debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
                    input.on_key(k);
                }
                Event::Resize(w, h) => log::info!("resize {w}x{h}"),
                _ => {}
            }
        }
        game.handle(input, audio);
        let now = Instant::now();
        acc += now.duration_since(last).as_secs_f32().min(0.1);
        last = now;
        while acc >= DT {
            acc -= DT;
            game.tick(audio);
        }
        if game.exit {
            return Ok(());
        }
        terminal.draw(|f| render::draw(f, game))?;
        if snapshot.elapsed() >= SNAPSHOT_EVERY {
            snapshot = Instant::now();
            if game.phase == Phase::Play {
                let p = game.lead();
                log::debug!(
                    "snapshot {:?} level {} {} t {:.1} toad row {} x {:.1} lives {} score {} time {:.1} dead {:?} princess {:?}",
                    game.mode,
                    game.level_no,
                    game.name,
                    game.lt as f32 * DT,
                    p.toad.row,
                    p.toad.x,
                    p.lives,
                    p.score,
                    p.time_left,
                    p.dead,
                    game.princess_pos()
                );
            } else {
                log::debug!("snapshot phase {:?} overlay {:?}", game.phase, game.overlay);
            }
        }
        let spent = frame_start.elapsed();
        if spent < FRAME {
            let _ = event::poll(FRAME - spent)?;
        }
    }
}
