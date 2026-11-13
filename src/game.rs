//! The game: story, endless road and two players; menus, lives, score and
//! the scenes between crossings.

use crate::audio::{Audio, Sfx};
use crate::endless;
use crate::input::{Ev, Input};
use crate::levels::{self, GIGGLES, KISSES, POND};
use crate::save::Save;
use crate::solver;
use crate::world::{DT, Death, Dir, HOP_TICKS, Level, Role, START_X, Step, Surface, Toad, W};

pub const LIVES: u32 = 5;
pub const EXTRA_LIFE: u32 = 5000;
/// Crossings to win a two-player race.
pub const RACE_TO: u32 = 5;

pub fn version_string() -> String {
    format!("Prince Toad {}", env!("CARGO_PKG_VERSION"))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Story,
    Endless,
    Versus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    Choose,
    Prologue(usize),
    WorldCard,
    Play,
    /// A toad made it; the short pause before the scene.
    Crossed,
    Kiss,
    GameOver,
    Finale(usize),
    /// A two-player race is won.
    Won(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Overlay {
    Pause,
    Help,
}

pub struct Player {
    pub toad: Toad,
    pub lives: u32,
    pub score: u32,
    /// The highest row reached this try (smaller is further).
    pub best_row: usize,
    /// Seconds left on this try.
    pub time_left: f32,
    /// Dead, and for how long.
    pub dead: Option<(Death, f32)>,
    pub queued: Option<(Dir, u32)>,
    pub crossings: u32,
    pub start_x: f32,
    next_life: u32,
}

impl Player {
    fn new(lv: &Level, start_x: f32, time: f32) -> Player {
        let toad = Toad::start(lv, start_x);
        Player {
            best_row: toad.row,
            toad,
            lives: LIVES,
            score: 0,
            time_left: time,
            dead: None,
            queued: None,
            crossings: 0,
            start_x,
            next_life: EXTRA_LIFE,
        }
    }

    fn respawn(&mut self, lv: &Level, time: f32) {
        self.toad = Toad::start(lv, self.start_x);
        self.best_row = self.toad.row;
        self.time_left = time;
        self.dead = None;
        self.queued = None;
    }
}

/// The princess in the pond, sitting on a lily pad.
pub struct Princess {
    pub row: usize,
    pub item: usize,
    pub off: f32,
    pub escapes: usize,
    /// Seconds since her last hop.
    pub since: f32,
    /// Where she hopped from, for the hop's arc.
    pub from: (f32, usize),
}

pub struct Demo {
    plan: Vec<Option<Dir>>,
    start: u64,
    every: u64,
}

pub struct Game {
    pub mode: Mode,
    pub phase: Phase,
    pub overlay: Option<Overlay>,
    /// Seconds in this phase.
    pub phase_t: f32,
    pub menu: usize,
    pub choose: usize,
    pub pause_sel: usize,
    /// Story crossing number, or endless crossings made.
    pub level_no: usize,
    pub level: Level,
    pub name: String,
    pub world: usize,
    pub night: bool,
    pub time: f32,
    /// Which kiss waits on the far side (story), or a random one.
    pub kiss: usize,
    /// Ticks in this level; lanes are a function of it.
    pub lt: u64,
    pub players: Vec<Player>,
    /// Who crossed last.
    pub winner: usize,
    pub princess: Option<Princess>,
    /// A line at the bottom for a while.
    pub msg: Option<(String, f32)>,
    pub save: Save,
    pub demo: bool,
    /// Sound off right now (shown in the menus).
    pub muted: bool,
    bot: Option<Demo>,
    pub exit: bool,
    seed: u64,
    /// Score at the start of this crossing, to go back to on a retry.
    level_score: u32,
    pub pad_names: Vec<String>,
}

pub enum Start {
    Title,
    Story(usize),
    Endless,
    Versus,
    /// The demo, from this crossing.
    Demo(usize),
}

impl Game {
    pub fn new(start: Start, save: Save) -> Game {
        let spec = &levels::story()[0];
        let level = spec.build();
        let mut g = Game {
            mode: Mode::Story,
            phase: Phase::Title,
            overlay: None,
            phase_t: 0.0,
            menu: 0,
            choose: 0,
            pause_sel: 0,
            level_no: 0,
            players: vec![Player::new(&level, START_X, spec.time)],
            level,
            name: spec.name.to_string(),
            world: 0,
            night: false,
            time: spec.time,
            kiss: 0,
            lt: 0,
            winner: 0,
            princess: None,
            msg: None,
            save,
            demo: false,
            muted: false,
            bot: None,
            exit: false,
            seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(7),
            level_score: 0,
            pad_names: Vec::new(),
        };
        match start {
            Start::Title => {}
            Start::Story(n) => g.start_story(n.min(POND), false),
            Start::Endless => g.start_endless(),
            Start::Versus => g.start_versus(),
            Start::Demo(n) => {
                g.demo = true;
                g.start_story(n.min(POND - 1), false);
            }
        }
        g
    }

    fn set_phase(&mut self, p: Phase) {
        log::info!(
            "phase {:?} -> {:?} (level {} {})",
            self.phase,
            p,
            self.level_no,
            self.name
        );
        self.phase = p;
        self.phase_t = 0.0;
    }

    pub fn title_items(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.save.reached > 0 && self.save.reached <= POND {
            v.push(format!(
                "Continue the story: {}",
                levels::story()[self.save.reached].name
            ));
        }
        v.push("A new story".into());
        v.push("The endless road".into());
        v.push("Two players".into());
        if self.save.reached > 0 {
            v.push("Choose a crossing".into());
        }
        v.push(format!("Sound: {}", if self.muted { "off" } else { "on" }));
        v.push("Quit".into());
        v
    }

    // ---- starting things ----

    fn load_story(&mut self, n: usize) {
        let all = levels::story();
        let spec = &all[n];
        self.level_no = n;
        self.level = spec.build();
        self.name = spec.name.to_string();
        self.world = spec.world;
        self.night = spec.night;
        self.time = spec.time;
        self.kiss = spec.kiss;
        self.lt = 0;
        self.bot = None;
        self.princess = None;
        if self.level.pond {
            self.place_princess(None);
        }
        let lives = self.players.first().map_or(LIVES, |p| p.lives);
        let score = self.players.first().map_or(0, |p| p.score);
        let next = self.players.first().map_or(EXTRA_LIFE, |p| p.next_life);
        let mut p = Player::new(&self.level, START_X, self.time);
        p.lives = lives;
        p.score = score;
        p.next_life = next;
        self.players = vec![p];
        self.level_score = score;
    }

    pub fn start_story(&mut self, n: usize, fresh: bool) {
        self.mode = Mode::Story;
        self.players = vec![];
        self.load_story(n);
        if fresh && n == 0 {
            self.set_phase(Phase::Prologue(0));
        } else {
            self.set_phase(Phase::WorldCard);
        }
    }

    fn load_made(&mut self, made: endless::Made, n: usize) {
        self.level = made.level;
        self.world = made.world;
        self.night = made.night;
        self.time = made.time;
        self.level_no = n;
        self.name = format!("Road {}", n + 1);
        self.lt = 0;
        self.bot = None;
        self.princess = None;
        self.kiss = (self.seed as usize + n * 7) % (KISSES.len() - 1);
        let xs: Vec<f32> = if self.mode == Mode::Versus {
            vec![30.0, 46.0]
        } else {
            vec![START_X]
        };
        for (i, x) in xs.iter().enumerate() {
            if let Some(p) = self.players.get_mut(i) {
                p.start_x = *x;
                p.respawn(&self.level, made.time);
            } else {
                self.players.push(Player::new(&self.level, *x, made.time));
            }
        }
    }

    pub fn start_endless(&mut self) {
        self.mode = Mode::Endless;
        self.players = vec![];
        self.seed = self.seed.wrapping_add(1);
        let made = endless::make(0, self.seed);
        self.load_made(made, 0);
        self.set_phase(Phase::Play);
    }

    pub fn start_versus(&mut self) {
        self.mode = Mode::Versus;
        self.players = vec![];
        self.seed = self.seed.wrapping_add(1);
        let made = endless::make(1, self.seed);
        self.load_made(made, 0);
        self.set_phase(Phase::Play);
    }

    /// Put the princess on a lily pad near the top, away from `away`.
    fn place_princess(&mut self, away: Option<(f32, usize)>) {
        let t = self.lt as f32 * DT;
        let lv = &self.level;
        let mut cands = Vec::new();
        for row in 1..lv.height().min(5) {
            for (i, it) in lv.rows[row].items.iter().enumerate() {
                if it.kind.role() != Role::Floater || lv.surface(row, i, t) != Surface::Up {
                    continue;
                }
                let x = lv.rows[row].x(i, t) + it.len / 2.0 - 1.0;
                // somewhere in the middle, where the current won't take her off soon
                if !(12.0..=W - 14.0).contains(&x) {
                    continue;
                }
                if let Some((ax, ar)) = away {
                    let d = (x - ax).abs() + (row as f32 - ar as f32).abs() * 2.0;
                    if d < 16.0 {
                        continue;
                    }
                }
                cands.push((row, i, it.len / 2.0 - 1.0, x));
            }
        }
        if cands.is_empty() {
            return;
        }
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let (row, item, off, _) = cands[(self.seed >> 33) as usize % cands.len()];
        let from = self.princess_pos().unwrap_or((W / 2.0, 1));
        let escapes = self.princess.as_ref().map_or(0, |p| p.escapes);
        self.princess = Some(Princess {
            row,
            item,
            off,
            escapes,
            since: 0.0,
            from,
        });
    }

    pub fn princess_pos(&self) -> Option<(f32, usize)> {
        let p = self.princess.as_ref()?;
        let t = self.lt as f32 * DT;
        Some((self.level.rows[p.row].x(p.item, t) + p.off, p.row))
    }

    // ---- input ----

    pub fn handle(&mut self, input: &mut Input, audio: &mut Audio) {
        input.diagonals = self.phase == Phase::Play && self.level.pond;
        let events = std::mem::take(&mut input.events);
        if !events.is_empty() {
            self.pad_names = input.pad_names.clone();
        }
        for e in events {
            log::debug!("event {e:?} in {:?} overlay {:?}", self.phase, self.overlay);
            self.on_event(e, audio);
        }
    }

    fn on_event(&mut self, e: Ev, audio: &mut Audio) {
        match e {
            Ev::Exit => {
                self.exit = true;
                return;
            }
            Ev::Mute => {
                audio.toggle_mute();
                self.save.muted = audio.muted;
                self.muted = audio.muted;
                self.save.store();
                self.flash(if audio.muted { "Sound off" } else { "Sound on" });
                return;
            }
            _ => {}
        }
        if self.demo {
            // any key ends the demo
            if matches!(e, Ev::Confirm | Ev::Pause | Ev::Back) {
                self.exit = true;
            }
            return;
        }
        if let Some(o) = self.overlay {
            match (o, e) {
                (Overlay::Help, _) => {
                    self.overlay = if self.phase == Phase::Play {
                        Some(Overlay::Pause)
                    } else {
                        None
                    }
                }
                (Overlay::Pause, Ev::Hop(_, Dir::Up)) => self.pause_sel = (self.pause_sel + 3) % 4,
                (Overlay::Pause, Ev::Hop(_, Dir::Down)) => {
                    self.pause_sel = (self.pause_sel + 1) % 4
                }
                (Overlay::Pause, Ev::Pause | Ev::Back) => self.overlay = None,
                (Overlay::Pause, Ev::Help) => self.overlay = Some(Overlay::Help),
                (Overlay::Pause, Ev::Confirm) => match self.pause_sel {
                    0 => self.overlay = None,
                    1 => self.overlay = Some(Overlay::Help),
                    2 => {
                        audio.toggle_mute();
                        self.save.muted = audio.muted;
                        self.muted = audio.muted;
                        self.save.store();
                    }
                    _ => {
                        self.overlay = None;
                        self.set_phase(Phase::Title);
                    }
                },
                _ => {}
            }
            return;
        }
        if e == Ev::Help {
            self.overlay = Some(Overlay::Help);
            return;
        }
        match self.phase {
            Phase::Title => {
                let items = self.title_items();
                match e {
                    Ev::Hop(_, Dir::Up) => {
                        self.menu = (self.menu + items.len() - 1) % items.len();
                        audio.play(Sfx::Select);
                    }
                    Ev::Hop(_, Dir::Down) => {
                        self.menu = (self.menu + 1) % items.len();
                        audio.play(Sfx::Select);
                    }
                    Ev::Confirm => {
                        self.title_choice(&items[self.menu.min(items.len() - 1)].clone(), audio)
                    }
                    Ev::Pause | Ev::Back => self.exit = true,
                    _ => {}
                }
            }
            Phase::Choose => {
                let n = self.save.reached.min(POND) + 1;
                match e {
                    Ev::Hop(_, Dir::Up) => self.choose = (self.choose + n - 1) % n,
                    Ev::Hop(_, Dir::Down) => self.choose = (self.choose + 1) % n,
                    Ev::Confirm => {
                        self.players.clear();
                        self.start_story(self.choose, false);
                    }
                    Ev::Pause | Ev::Back => self.set_phase(Phase::Title),
                    _ => {}
                }
            }
            Phase::Prologue(i) => match e {
                Ev::Confirm | Ev::Hop(..) if self.phase_t > 0.3 => {
                    if i + 1 < levels::PROLOGUE.len() {
                        self.set_phase(Phase::Prologue(i + 1));
                    } else {
                        self.set_phase(Phase::WorldCard);
                    }
                }
                Ev::Pause | Ev::Back => self.set_phase(Phase::WorldCard),
                _ => {}
            },
            Phase::WorldCard => match e {
                Ev::Confirm | Ev::Hop(..) if self.phase_t > 0.4 => self.set_phase(Phase::Play),
                Ev::Pause => self.overlay = Some(Overlay::Pause),
                _ => {}
            },
            Phase::Play => match e {
                Ev::Hop(p, d) => {
                    let p = if self.mode == Mode::Versus {
                        p.min(1)
                    } else {
                        0
                    };
                    if let Some(pl) = self.players.get_mut(p)
                        && pl.dead.is_none()
                    {
                        pl.queued = Some((d, 0));
                    }
                }
                Ev::Pause | Ev::Back => {
                    self.pause_sel = 0;
                    self.overlay = Some(Overlay::Pause);
                }
                _ => {}
            },
            Phase::Crossed => {}
            Phase::Kiss => {
                if matches!(e, Ev::Confirm | Ev::Hop(..)) && self.phase_t > 2.0 {
                    self.after_kiss();
                }
            }
            Phase::GameOver => {
                if matches!(e, Ev::Confirm) && self.phase_t > 1.0 {
                    // try this crossing again, with fresh lives and the score it started with
                    let score = self.level_score / 2;
                    let n = self.level_no;
                    if self.mode == Mode::Story {
                        self.players[0].lives = LIVES;
                        self.players[0].score = score;
                        self.load_story(n);
                        self.set_phase(Phase::WorldCard);
                    } else {
                        self.start_endless();
                    }
                } else if matches!(e, Ev::Pause | Ev::Back) {
                    self.set_phase(Phase::Title);
                }
            }
            Phase::Finale(i) => {
                if matches!(e, Ev::Confirm | Ev::Hop(..)) && self.phase_t > 1.0 {
                    if i + 1 < levels::FINALE.len() {
                        self.set_phase(Phase::Finale(i + 1));
                    } else {
                        self.set_phase(Phase::Title);
                    }
                }
            }
            Phase::Won(_) => {
                if matches!(e, Ev::Confirm) && self.phase_t > 1.0 {
                    self.start_versus();
                } else if matches!(e, Ev::Pause | Ev::Back) {
                    self.set_phase(Phase::Title);
                }
            }
        }
    }

    fn title_choice(&mut self, item: &str, audio: &mut Audio) {
        audio.play(Sfx::Croak1);
        self.players.clear();
        if item.starts_with("Continue") {
            self.start_story(self.save.reached.min(POND), false);
        } else if item.starts_with("A new story") {
            self.start_story(0, true);
        } else if item.starts_with("The endless") {
            self.start_endless();
        } else if item.starts_with("Two players") {
            self.start_versus();
        } else if item.starts_with("Choose") {
            self.choose = self.save.reached.min(POND);
            self.set_phase(Phase::Choose);
        } else if item.starts_with("Sound") {
            audio.toggle_mute();
            self.save.muted = audio.muted;
            self.muted = audio.muted;
            self.save.store();
        } else {
            self.exit = true;
        }
    }

    pub fn flash(&mut self, s: &str) {
        self.msg = Some((s.to_string(), 3.0));
    }

    // ---- the clock ----

    /// One fixed step of DT seconds.
    pub fn tick(&mut self, audio: &mut Audio) {
        if self.overlay.is_some() {
            return;
        }
        self.phase_t += DT;
        if let Some((_, t)) = &mut self.msg {
            *t -= DT;
            if *t <= 0.0 {
                self.msg = None;
            }
        }
        match self.phase {
            Phase::Play => self.play_tick(audio),
            Phase::WorldCard if self.demo && self.phase_t > 2.5 => self.set_phase(Phase::Play),
            Phase::Crossed if self.phase_t > 0.7 => {
                if self.mode == Mode::Story {
                    self.set_phase(Phase::Kiss);
                } else {
                    self.next_endless();
                }
            }
            Phase::Kiss => {
                if (self.phase_t - 1.2).abs() < DT / 2.0 + 1e-4 {
                    audio.play(Sfx::Kiss);
                }
                if self.demo && self.phase_t > 6.0 {
                    self.after_kiss();
                }
            }
            Phase::Finale(0) if (self.phase_t - DT).abs() < 1e-4 => audio.play(Sfx::Fanfare),
            _ => {}
        }
    }

    fn after_kiss(&mut self) {
        let next = self.level_no + 1;
        if next > POND || (self.demo && next >= POND) {
            if self.demo {
                self.exit = true;
            }
            return;
        }
        let world_changes = levels::story()[next].world != self.world;
        self.load_story(next);
        if self.save.reached < next {
            self.save.reached = next;
            self.save.store();
        }
        if world_changes {
            self.set_phase(Phase::WorldCard);
        } else {
            self.set_phase(Phase::Play);
        }
    }

    fn next_endless(&mut self) {
        let n = self.level_no + 1;
        let d = if self.mode == Mode::Versus {
            n as u32 + 1
        } else {
            n as u32
        };
        let made = endless::make(d, self.seed.wrapping_add(n as u64));
        self.load_made(made, n);
        self.set_phase(Phase::Play);
    }

    fn play_tick(&mut self, audio: &mut Audio) {
        // the demo plays itself
        if self.demo {
            self.bot_tick();
        }
        self.lt += 1;
        let t = self.lt as f32 * DT;
        let n = self.players.len();
        for i in 0..n {
            if self.phase != Phase::Play {
                break;
            }
            self.player_tick(i, t, audio);
        }
        if self.level.pond && self.phase == Phase::Play {
            self.princess_tick(audio);
        }
    }

    fn bot_tick(&mut self) {
        let p = &self.players[0];
        if p.dead.is_some() {
            self.bot = None;
            return;
        }
        if self.bot.is_none() {
            let left = (p.time_left / DT) as u64;
            // at a human pace where it can, flat out where it must
            let (plan, every) = match solver::solve_every(
                &self.level,
                &p.toad,
                self.lt,
                left,
                2 * HOP_TICKS as u64,
            ) {
                Some(plan) => (plan, 2 * HOP_TICKS as u64),
                None => (
                    solver::solve(&self.level, &p.toad, self.lt, left).unwrap_or_default(),
                    HOP_TICKS as u64,
                ),
            };
            log::debug!("demo plan of {} hops from tick {}", plan.len(), self.lt);
            self.bot = Some(Demo {
                plan,
                start: self.lt,
                every,
            });
        }
        let bot = self.bot.as_ref().unwrap();
        let k = self.lt - bot.start;
        if k.is_multiple_of(bot.every)
            && let Some(Some(d)) = bot.plan.get((k / bot.every) as usize)
        {
            self.players[0].queued = Some((*d, 0));
        }
    }

    fn player_tick(&mut self, i: usize, t: f32, audio: &mut Audio) {
        let time = self.time;
        let story = self.mode == Mode::Story;
        let p = &mut self.players[i];
        if let Some((cause, dt)) = &mut p.dead {
            *dt += DT;
            let wait = if *cause == Death::Splash { 0.6 } else { 1.0 };
            if *dt >= wait {
                if self.mode == Mode::Versus || *cause == Death::Splash {
                    p.respawn(&self.level, time);
                } else if p.lives > 1 {
                    p.lives -= 1;
                    p.respawn(&self.level, time);
                } else {
                    p.lives = 0;
                    audio.play(Sfx::GameOver);
                    if self.mode == Mode::Endless {
                        let s = p.score;
                        if s > self.save.best_endless {
                            self.save.best_endless = s;
                            self.save.store();
                        }
                    }
                    self.set_phase(Phase::GameOver);
                }
            }
            return;
        }
        // the clock (not in the pond)
        if time > 0.0 {
            let before = p.time_left;
            p.time_left -= DT;
            if p.time_left <= 10.0
                && before.ceil() != p.time_left.ceil()
                && p.time_left > 0.0
                && i == 0
            {
                audio.play(Sfx::Tick);
            }
            if p.time_left <= 0.0 {
                p.dead = Some((Death::TimeUp, 0.0));
                audio.play(Sfx::Splat);
                log::info!("player {i} out of time");
                return;
            }
        }
        // a queued hop goes when the toad is ready, or goes stale
        let mut want = None;
        if let Some((d, age)) = p.queued {
            if p.toad.cool <= 1 && p.toad.slide.is_none() {
                want = Some(d);
                p.queued = None;
            } else if age > 14 {
                p.queued = None;
            } else {
                p.queued = Some((d, age + 1));
            }
        }
        let step = p.toad.tick(&self.level, t, want);
        match step {
            Step::Hopped { slid } => {
                if slid {
                    audio.play(Sfx::Slide);
                } else {
                    audio.croak();
                }
                if p.toad.row < p.best_row {
                    p.best_row = p.toad.row;
                    p.score += 10;
                }
                if self.level.pond
                    && let Some(item) = self.level.floater(p.toad.row, p.toad.x, t)
                {
                    self.level.land(p.toad.row, item, t);
                }
            }
            Step::Bumped => audio.play(Sfx::Bump),
            Step::Died(cause) => {
                log::info!(
                    "player {i} died: {cause:?} at row {} x {:.1}",
                    p.toad.row,
                    p.toad.x
                );
                p.dead = Some((cause, 0.0));
                audio.play(match cause {
                    Death::Squashed | Death::TimeUp => Sfx::Splat,
                    Death::Drowned | Death::Swept => Sfx::Splash,
                    Death::Splash => Sfx::Plop,
                });
                if cause == Death::Splash {
                    self.flash("Splash! Back to the bank.");
                }
            }
            Step::Crossed => {
                let bonus = 50
                    + if time > 0.0 {
                        (p.time_left as u32) * 10
                    } else {
                        0
                    };
                p.score += bonus;
                p.crossings += 1;
                self.winner = i;
                log::info!(
                    "player {i} crossed {} with {:.1}s left, score {}",
                    self.name,
                    p.time_left,
                    p.score
                );
                audio.play(Sfx::Crossed);
                if story && self.level_no + 1 > self.save.reached {
                    self.save.reached = self.level_no + 1;
                    if p.score > self.save.best_story {
                        self.save.best_story = p.score;
                    }
                    self.save.store();
                }
                if self.mode == Mode::Versus && p.crossings >= RACE_TO {
                    self.set_phase(Phase::Won(i));
                    audio.play(Sfx::Fanfare);
                } else {
                    self.set_phase(Phase::Crossed);
                }
            }
            Step::Still => {}
        }
        let p = &mut self.players[i];
        if self.mode != Mode::Versus && p.score >= p.next_life {
            p.next_life += EXTRA_LIFE;
            p.lives += 1;
            audio.play(Sfx::Life);
            self.msg = Some(("An extra life!".into(), 2.0));
        }
    }

    fn princess_tick(&mut self, audio: &mut Audio) {
        let Some((px, prow)) = self.princess_pos() else {
            self.place_princess(None);
            return;
        };
        let toad = &self.players[0].toad;
        let alive = self.players[0].dead.is_none();
        let (tx, trow) = (toad.x, toad.row);
        if let Some(pr) = &mut self.princess {
            pr.since += DT;
        }
        let pr = self.princess.as_ref().unwrap();
        let near = (tx - px).abs() <= 7.0 && (trow as i32 - prow as i32).abs() <= 2;
        let met = alive
            && ((trow == prow && (tx - px).abs() <= 3.0)
                || ((trow as i32 - prow as i32).abs() == 1 && (tx - px).abs() <= 2.0));
        if pr.escapes >= 3 {
            if met {
                log::info!("the prince meets the princess");
                audio.play(Sfx::Kiss);
                self.players[0].score += 1000;
                if self.players[0].score > self.save.best_story {
                    self.save.best_story = self.players[0].score;
                }
                self.save.finished = true;
                self.save.store();
                self.set_phase(Phase::Finale(0));
            }
        } else if alive && near && pr.since > 0.8 {
            let escapes = pr.escapes + 1;
            self.place_princess(Some((tx, trow)));
            if let Some(pr) = &mut self.princess {
                pr.escapes = escapes;
            }
            audio.play(Sfx::Giggle);
            self.msg = Some((GIGGLES[escapes - 1].to_string(), 3.0));
            if escapes == 3 {
                self.msg = Some((format!("{} {}", GIGGLES[2], GIGGLES[3]), 4.0));
            }
        } else if !(4.0..=W - 6.0).contains(&px) {
            // her pad is drifting away: she hops to another, quietly
            let escapes = pr.escapes;
            self.place_princess(Some((px, prow)));
            if let Some(pr) = &mut self.princess {
                pr.escapes = escapes;
            }
        }
    }

    /// Seconds left to show, of player 0.
    pub fn lead(&self) -> &Player {
        &self.players[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(start: Start) -> Game {
        Game::new(start, Save::default())
    }

    #[test]
    fn the_demo_plays_the_whole_story_up_to_the_pond() {
        let mut g = game(Start::Demo(0));
        let mut audio = Audio::silent();
        let mut kisses = 0;
        let mut was_kiss = false;
        for _ in 0..60 * 60 * 30 {
            g.tick(&mut audio);
            if g.phase == Phase::Kiss && !was_kiss {
                kisses += 1;
            }
            was_kiss = g.phase == Phase::Kiss;
            if g.exit {
                break;
            }
        }
        assert!(
            g.exit,
            "demo still running at crossing {} ({:?})",
            g.level_no, g.phase
        );
        assert_eq!(kisses, POND, "every crossing ends in a kiss");
        assert!(g.players[0].lives >= LIVES, "the demo never dies");
    }

    #[test]
    fn hopping_into_a_river_costs_a_life() {
        let mut g = game(Start::Story(6));
        let mut audio = Audio::silent();
        let mut input = Input::new();
        g.set_phase(Phase::Play);
        // to the river bank and straight in, three rows up the water, at a gap or not
        for _ in 0..40 {
            input.events.push(Ev::Hop(0, Dir::Up));
            g.handle(&mut input, &mut audio);
            for _ in 0..HOP_TICKS {
                g.tick(&mut audio);
            }
        }
        for _ in 0..120 {
            g.tick(&mut audio);
        }
        assert!(
            g.players[0].lives < LIVES || g.phase != Phase::Play,
            "{:?} lives {}",
            g.phase,
            g.players[0].lives
        );
    }

    #[test]
    fn the_princess_hops_away_three_times_then_waits() {
        let mut g = game(Start::Story(POND));
        let mut audio = Audio::silent();
        g.set_phase(Phase::Play);
        assert!(g.princess.is_some());
        for round in 0..3 {
            let (px, prow) = g.princess_pos().unwrap();
            g.players[0].toad.x = px;
            g.players[0].toad.row = (prow + 2).min(g.level.height() - 1);
            for _ in 0..60 {
                g.princess_tick(&mut audio);
            }
            assert_eq!(g.princess.as_ref().unwrap().escapes, round + 1);
        }
        let (px, prow) = g.princess_pos().unwrap();
        g.players[0].toad.x = px + 1.0;
        g.players[0].toad.row = prow;
        g.princess_tick(&mut audio);
        assert_eq!(g.phase, Phase::Finale(0));
    }

    #[test]
    fn endless_and_versus_start() {
        let g = game(Start::Endless);
        assert_eq!(g.players.len(), 1);
        let g = game(Start::Versus);
        assert_eq!(g.players.len(), 2);
        assert!(g.players[0].toad.x != g.players[1].toad.x);
    }
}
