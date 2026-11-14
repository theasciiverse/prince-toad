//! Keyboard and gamepad, turned into hops and commands.
//!
//! - Player one: W A S D, diagonals Q E Z C (pond only)
//! - Player two: arrows, numpad 8 4 6 2, diagonals numpad 7 9 1 3
//!   (or Home, PgUp, End, PgDn with Num Lock off)
//! - Alone, every one of those keys moves you.
//! - Keys chosen in the options come first.
//! - Enter or Space: go on, Esc or P: pause, H or F1: help, M or F8: sound,
//!   Ctrl+C: quit at once.
//!
//! Gamepad (gilrs, as tspace): d-pad or left stick hops, two d-pad
//! directions together hop diagonally (pond only), A goes on, Start
//! pauses, B goes back, Back shows help. The first pad is player one, the
//! second player two. Only pads in SDL's controller database count;
//! `PRINCE_TOAD_ANY_PAD=1` accepts everything.

use crate::world::Dir;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ev {
    Hop(usize, Dir),
    Confirm,
    Back,
    Pause,
    Help,
    Mute,
    Exit,
    /// A key as it is, while the game wants one (a name, a key to choose).
    Raw(KeyCode),
}

/// The hops that can be given other keys in the options: (save name, label,
/// player, direction).
pub const ACTIONS: [(&str, &str, usize, Dir); 8] = [
    ("p1_up", "Player one: up", 0, Dir::Up),
    ("p1_left", "Player one: left", 0, Dir::Left),
    ("p1_down", "Player one: down", 0, Dir::Down),
    ("p1_right", "Player one: right", 0, Dir::Right),
    ("p2_up", "Player two: up", 1, Dir::Up),
    ("p2_left", "Player two: left", 1, Dir::Left),
    ("p2_down", "Player two: down", 1, Dir::Down),
    ("p2_right", "Player two: right", 1, Dir::Right),
];

/// The keys every action has without any choosing.
pub const DEFAULT_KEYS: [&str; 8] = ["w", "a", "s", "d", "up", "left", "down", "right"];

/// A key's name in the save and on screen.
pub fn key_name(k: KeyCode) -> Option<String> {
    Some(match k {
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char(c) if !c.is_control() => c.to_lowercase().to_string(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::Insert => "insert".into(),
        KeyCode::Delete => "delete".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::F(n) => format!("f{n}"),
        _ => return None,
    })
}

pub fn parse_key(s: &str) -> Option<KeyCode> {
    Some(match s {
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "insert" => KeyCode::Insert,
        "delete" => KeyCode::Delete,
        "tab" => KeyCode::Tab,
        _ => {
            let mut cs = s.chars();
            let c = cs.next()?;
            if cs.as_str().is_empty() {
                KeyCode::Char(c)
            } else if let Some(n) = s.strip_prefix('f').and_then(|n| n.parse().ok()) {
                KeyCode::F(n)
            } else {
                return None;
            }
        }
    })
}

/// Keys that keep their own jobs and cannot be given to a hop.
pub fn reserved(k: KeyCode) -> bool {
    matches!(
        k,
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace | KeyCode::F(1) | KeyCode::F(8)
    )
}

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings
        || std::env::var("PRINCE_TOAD_ANY_PAD").is_ok_and(|v| v == "1")
}

/// A d-pad press waits this many frames for a second direction, in the pond.
#[cfg(feature = "gamepad")]
const CHORD_FRAMES: u32 = 4;

#[cfg(feature = "gamepad")]
#[derive(Default)]
struct PadState {
    id: Option<gilrs::GamepadId>,
    /// A d-pad direction waiting to see if it becomes a diagonal.
    pending: Option<(Dir, u32)>,
    /// The stick's last direction, so it hops once per push.
    stick: Option<Dir>,
}

pub struct Input {
    pub events: Vec<Ev>,
    /// Diagonals allowed (the pond): pad presses wait a moment for a chord.
    pub diagonals: bool,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(feature = "gamepad")]
    pads: [PadState; 2],
    pub pad_names: Vec<String>,
    /// Keys chosen in the options, looked at before the usual ones.
    pub binds: Vec<(KeyCode, usize, Dir)>,
    /// Hand keys over as they are (`Ev::Raw`).
    pub raw: bool,
}

fn combine(a: Dir, b: Dir) -> Option<Dir> {
    use Dir::*;
    Some(match (a, b) {
        (Up, Left) | (Left, Up) => UpLeft,
        (Up, Right) | (Right, Up) => UpRight,
        (Down, Left) | (Left, Down) => DownLeft,
        (Down, Right) | (Right, Down) => DownRight,
        _ => return None,
    })
}

impl Input {
    pub fn new() -> Input {
        Input {
            events: Vec::new(),
            diagonals: false,
            #[cfg(feature = "gamepad")]
            gilrs: None,
            #[cfg(feature = "gamepad")]
            pads: Default::default(),
            pad_names: Vec::new(),
            binds: Vec::new(),
            raw: false,
        }
    }

    /// Take the chosen keys from the save's (action, key) pairs.
    pub fn set_binds(&mut self, keys: &[(String, String)]) {
        self.binds = keys
            .iter()
            .filter_map(|(a, k)| {
                let (_, _, p, d) = ACTIONS.iter().find(|x| x.0 == a)?;
                Some((parse_key(k)?, *p, *d))
            })
            .collect();
    }

    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g
                    .gamepads()
                    .map(|(id, p)| (id, p.name().to_string(), usable_pad(&p)))
                    .collect();
                log::info!(
                    "gamepads (name, used): {:?}",
                    all.iter().map(|(_, n, u)| (n, u)).collect::<Vec<_>>()
                );
                for (id, name, _) in all.into_iter().filter(|(_, _, u)| *u).take(2) {
                    let slot = self.pad_names.len();
                    self.pads[slot].id = Some(id);
                    self.pad_names.push(name);
                }
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    /// Drain gamepad events; call once per frame.
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            let mut events = Vec::new();
            while let Some(ev) = g.next_event() {
                events.push(ev);
            }
            for ev in events {
                let g = self.gilrs.as_ref().unwrap();
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                let slot = match self.pads.iter().position(|p| p.id == Some(ev.id)) {
                    Some(s) => s,
                    None => match self.pads.iter().position(|p| p.id.is_none()) {
                        Some(s) => {
                            self.pads[s].id = Some(ev.id);
                            log::info!("gamepad {} joined as player {}", pad.name(), s + 1);
                            self.pad_names.push(pad.name().to_string());
                            s
                        }
                        None => continue,
                    },
                };
                match ev.event {
                    EventType::Disconnected => {
                        log::info!("gamepad of player {} disconnected", slot + 1);
                        self.pads[slot] = PadState::default();
                    }
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad {} button {b:?}", slot + 1);
                        let dir = match b {
                            Button::DPadUp => Some(Dir::Up),
                            Button::DPadDown => Some(Dir::Down),
                            Button::DPadLeft => Some(Dir::Left),
                            Button::DPadRight => Some(Dir::Right),
                            _ => None,
                        };
                        if let Some(d) = dir {
                            let p = &mut self.pads[slot];
                            match p.pending.take() {
                                Some((first, _)) => match combine(first, d) {
                                    Some(diag) => self.events.push(Ev::Hop(slot, diag)),
                                    None => {
                                        self.events.push(Ev::Hop(slot, first));
                                        p.pending = Some((d, 0));
                                    }
                                },
                                None if self.diagonals => p.pending = Some((d, 0)),
                                None => self.events.push(Ev::Hop(slot, d)),
                            }
                            continue;
                        }
                        match b {
                            Button::South => self.events.push(Ev::Confirm),
                            Button::East => self.events.push(Ev::Back),
                            Button::Start => self.events.push(Ev::Pause),
                            Button::Select => self.events.push(Ev::Help),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            // pending d-pad presses that found no partner, and the sticks
            for slot in 0..2 {
                let Some(id) = self.pads[slot].id else {
                    continue;
                };
                if let Some((d, n)) = self.pads[slot].pending {
                    if n + 1 >= CHORD_FRAMES {
                        self.events.push(Ev::Hop(slot, d));
                        self.pads[slot].pending = None;
                    } else {
                        self.pads[slot].pending = Some((d, n + 1));
                    }
                }
                let g = self.gilrs.as_ref().unwrap();
                let p = g.gamepad(id);
                let axis = |a: Axis| p.axis_data(a).map_or(0.0, |d| d.value());
                let (x, y) = (axis(Axis::LeftStickX), axis(Axis::LeftStickY));
                let (h, v) = (
                    if x > 0.5 {
                        1
                    } else if x < -0.5 {
                        -1
                    } else {
                        0
                    },
                    if y > 0.5 {
                        -1
                    } else if y < -0.5 {
                        1
                    } else {
                        0
                    },
                );
                let dir = match (h, v) {
                    (0, -1) => Some(Dir::Up),
                    (0, 1) => Some(Dir::Down),
                    (-1, 0) => Some(Dir::Left),
                    (1, 0) => Some(Dir::Right),
                    (-1, -1) => Some(if self.diagonals { Dir::UpLeft } else { Dir::Up }),
                    (1, -1) => Some(if self.diagonals {
                        Dir::UpRight
                    } else {
                        Dir::Up
                    }),
                    (-1, 1) => Some(if self.diagonals {
                        Dir::DownLeft
                    } else {
                        Dir::Down
                    }),
                    (1, 1) => Some(if self.diagonals {
                        Dir::DownRight
                    } else {
                        Dir::Down
                    }),
                    _ => None,
                };
                if dir != self.pads[slot].stick {
                    if let Some(d) = dir {
                        self.events.push(Ev::Hop(slot, d));
                    }
                    self.pads[slot].stick = dir;
                }
            }
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) {
        if k.kind == KeyEventKind::Release {
            return;
        }
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            self.events.push(Ev::Exit);
            return;
        }
        if self.raw {
            self.events.push(Ev::Raw(k.code));
            return;
        }
        let code = match k.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            c => c,
        };
        if let Some((_, p, d)) = self.binds.iter().find(|b| b.0 == code) {
            self.events.push(Ev::Hop(*p, *d));
            return;
        }
        let hop = |p: usize, d: Dir| Some(Ev::Hop(p, d));
        let ev = match k.code {
            KeyCode::Char('w' | 'W') => hop(0, Dir::Up),
            KeyCode::Char('a' | 'A') => hop(0, Dir::Left),
            KeyCode::Char('s' | 'S') => hop(0, Dir::Down),
            KeyCode::Char('d' | 'D') => hop(0, Dir::Right),
            KeyCode::Char('q' | 'Q') => hop(0, Dir::UpLeft),
            KeyCode::Char('e' | 'E') => hop(0, Dir::UpRight),
            KeyCode::Char('z' | 'Z') => hop(0, Dir::DownLeft),
            KeyCode::Char('c' | 'C') => hop(0, Dir::DownRight),
            KeyCode::Up | KeyCode::Char('8') => hop(1, Dir::Up),
            KeyCode::Left | KeyCode::Char('4') => hop(1, Dir::Left),
            KeyCode::Down | KeyCode::Char('2') => hop(1, Dir::Down),
            KeyCode::Right | KeyCode::Char('6') => hop(1, Dir::Right),
            KeyCode::Home | KeyCode::Char('7') => hop(1, Dir::UpLeft),
            KeyCode::PageUp | KeyCode::Char('9') => hop(1, Dir::UpRight),
            KeyCode::End | KeyCode::Char('1') => hop(1, Dir::DownLeft),
            KeyCode::PageDown | KeyCode::Char('3') => hop(1, Dir::DownRight),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Ev::Confirm),
            KeyCode::Esc | KeyCode::Char('p' | 'P') => Some(Ev::Pause),
            KeyCode::Backspace => Some(Ev::Back),
            KeyCode::F(1) | KeyCode::Char('h' | 'H' | '?') => Some(Ev::Help),
            KeyCode::F(8) | KeyCode::Char('m' | 'M') => Some(Ev::Mute),
            _ => None,
        };
        if let Some(e) = ev {
            self.events.push(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn press(c: KeyCode) -> KeyEvent {
        KeyEvent {
            code: c,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn key_names_round_trip_and_chosen_keys_win() {
        for k in [
            KeyCode::Char('i'),
            KeyCode::Up,
            KeyCode::F(5),
            KeyCode::Char(' '),
            KeyCode::Home,
        ] {
            assert_eq!(parse_key(&key_name(k).unwrap()), Some(k));
        }
        for d in DEFAULT_KEYS {
            assert!(parse_key(d).is_some());
        }
        let mut i = Input::new();
        i.set_binds(&[("p1_up".into(), "i".into()), ("nope".into(), "x".into())]);
        assert_eq!(i.binds.len(), 1);
        i.on_key(press(KeyCode::Char('I')));
        i.on_key(press(KeyCode::Char('w')));
        assert_eq!(i.events, vec![Ev::Hop(0, Dir::Up), Ev::Hop(0, Dir::Up)]);
        i.events.clear();
        i.raw = true;
        i.on_key(press(KeyCode::Char('x')));
        assert_eq!(i.events, vec![Ev::Raw(KeyCode::Char('x'))]);
    }
}
