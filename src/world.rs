//! The field: lanes, what moves on them, and the toad's hops.
//!
//! A level is a stack of one-row lanes. Everything on a lane moves together
//! and is a pure function of time, so the same level can be simulated ahead
//! (the solver in `solver.rs`) exactly as it plays. Columns are half as wide
//! as rows are tall, so one hop is 2 columns sideways or 1 row up or down,
//! and a diagonal (pond only) is both at once.

/// Field width in columns; the toad lives in 0..W.
pub const W: f32 = 80.0;
/// Every lane pattern repeats every PERIOD columns.
pub const PERIOD: f32 = 100.0;
/// Field column of a lane pattern's first character: lanes run 10 columns
/// past both edges of the field, so things drive in from off the field.
pub const LEFT: f32 = -10.0;
/// One simulation step.
pub const DT: f32 = 1.0 / 60.0;
/// Ticks between hops.
pub const HOP_TICKS: u32 = 8;
/// Where the toad starts, in columns.
pub const START_X: f32 = 38.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    Grass,
    Dirt,
    Road,
    Sand,
    /// Railway track: trains come rarely and fast.
    Rail,
    Snow,
    /// Slippery: a hop onto ice carries on one more hop the same way.
    Ice,
    Cobble,
    Water,
    Moat,
    /// Like water, but falling in only splashes you back to the bank.
    Pond,
    /// The far side, where someone is waiting.
    Bank,
}

impl Ground {
    pub fn parse(s: &str) -> Ground {
        match s {
            "grass" => Ground::Grass,
            "dirt" => Ground::Dirt,
            "road" => Ground::Road,
            "sand" => Ground::Sand,
            "rail" => Ground::Rail,
            "snow" => Ground::Snow,
            "ice" => Ground::Ice,
            "cobble" => Ground::Cobble,
            "water" => Ground::Water,
            "moat" => Ground::Moat,
            "pond" => Ground::Pond,
            "bank" => Ground::Bank,
            other => panic!("unknown ground {other}"),
        }
    }

    pub fn wet(self) -> bool {
        matches!(self, Ground::Water | Ground::Moat | Ground::Pond)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Touch it and you are flat.
    Hazard,
    /// Stand on it to stay dry.
    Floater,
    /// You cannot hop into it.
    Blocker,
    /// Only to look at.
    Deco,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Car,
    Truck,
    Bike,
    Cart,
    Tractor,
    Goose,
    Moto,
    Plow,
    Sled,
    Knight,
    Carriage,
    Fire,
    Guard,
    Train,
    Log,
    Turtle,
    DivingTurtle,
    Floe,
    CrackingFloe,
    Pad,
    Barrel,
    Duck,
    DivingDuck,
    Hedge,
    Rock,
    Tree,
    Lamp,
    Cactus,
    Flowers,
    House,
    Reeds,
    /// A drawbridge over the moat: it stands still and goes up now and then.
    Bridge,
    /// A market stall: in the way.
    Stall,
}

impl Kind {
    pub fn from_char(c: char) -> Option<Kind> {
        Some(match c {
            'c' => Kind::Car,
            't' => Kind::Truck,
            'b' => Kind::Bike,
            'k' => Kind::Cart,
            'r' => Kind::Tractor,
            'g' => Kind::Goose,
            'm' => Kind::Moto,
            's' => Kind::Plow,
            'z' => Kind::Sled,
            'h' => Kind::Knight,
            'a' => Kind::Carriage,
            'f' => Kind::Fire,
            'w' => Kind::Guard,
            'R' => Kind::Train,
            'l' => Kind::Log,
            'u' => Kind::Turtle,
            'U' => Kind::DivingTurtle,
            'i' => Kind::Floe,
            'I' => Kind::CrackingFloe,
            'p' => Kind::Pad,
            'x' => Kind::Barrel,
            'd' => Kind::Duck,
            'D' => Kind::DivingDuck,
            '#' => Kind::Hedge,
            'o' => Kind::Rock,
            'T' => Kind::Tree,
            '*' => Kind::Lamp,
            '^' => Kind::Cactus,
            'y' => Kind::Flowers,
            'H' => Kind::House,
            'v' => Kind::Reeds,
            '=' => Kind::Bridge,
            'M' => Kind::Stall,
            _ => return None,
        })
    }

    pub fn role(self) -> Role {
        use Kind::*;
        match self {
            Car | Truck | Bike | Cart | Tractor | Goose | Moto | Plow | Sled | Knight
            | Carriage | Fire | Guard | Train => Role::Hazard,
            Log | Turtle | DivingTurtle | Floe | CrackingFloe | Pad | Barrel | Duck
            | DivingDuck | Bridge => Role::Floater,
            Hedge | Rock | Tree | Lamp | Cactus | House | Stall => Role::Blocker,
            Flowers | Reeds => Role::Deco,
        }
    }

    pub fn dives(self) -> bool {
        matches!(
            self,
            Kind::DivingTurtle | Kind::CrackingFloe | Kind::DivingDuck | Kind::Bridge
        )
    }
}

/// Whether a floater is there to stand on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Surface {
    Up,
    /// Going under: still holds you, for now (0..1 how far gone).
    Sinking(f32),
    Under,
}

const DIVE_CYCLE: f32 = 6.0;
const DIVE_SINK: f32 = 3.8;
const DIVE_UNDER: f32 = 4.8;

/// A lily pad starts to sink when you land on it.
pub const PAD_WOBBLE: f32 = 0.45;
pub const PAD_UNDER: f32 = 1.0;
pub const PAD_BACK: f32 = 3.0;

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: Kind,
    /// Start column in the pattern.
    pub x0: f32,
    pub len: f32,
    /// For variety: colours and the like.
    pub seed: u32,
}

#[derive(Clone, Debug)]
pub struct Lane {
    pub ground: Ground,
    /// Columns a second, + to the right.
    pub speed: f32,
    /// Skidding: how many columns the lane swings back and forth.
    pub wobble: f32,
    pub items: Vec<Item>,
}

impl Lane {
    /// Build a lane from a pattern: each run of one letter is one item.
    /// Patterns shorter than PERIOD repeat to fill it (cut off at PERIOD).
    pub fn new(ground: Ground, speed: f32, wobble: f32, pattern: &str) -> Lane {
        let given: Vec<char> = pattern.chars().take(100).collect();
        let chars: Vec<char> = if given.is_empty() {
            Vec::new()
        } else {
            (0..100).map(|i| given[i % given.len()]).collect()
        };
        let mut items = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            match Kind::from_char(chars[i]) {
                Some(kind) => {
                    let mut j = i + 1;
                    while j < chars.len() && chars[j] == chars[i] {
                        j += 1;
                    }
                    let seed = (i as u32).wrapping_mul(2_654_435_761) ^ (speed.to_bits() >> 7);
                    items.push(Item {
                        kind,
                        x0: i as f32,
                        len: (j - i) as f32,
                        seed,
                    });
                    i = j;
                }
                None => i += 1,
            }
        }
        Lane {
            ground,
            speed,
            wobble,
            items,
        }
    }

    pub fn moving(&self) -> bool {
        self.speed != 0.0 || self.wobble != 0.0
    }

    pub fn offset(&self, t: f32) -> f32 {
        self.speed * t + self.wobble * (t * 0.8).sin()
    }

    pub fn velocity(&self, t: f32) -> f32 {
        self.speed + self.wobble * 0.8 * (t * 0.8).cos()
    }

    /// The item's left column at time t, in LEFT..LEFT+PERIOD.
    pub fn x(&self, i: usize, t: f32) -> f32 {
        let it = &self.items[i];
        if !self.moving() {
            return it.x0 + LEFT;
        }
        (it.x0 + self.offset(t)).rem_euclid(PERIOD) + LEFT
    }

    /// Where the item covers at time t: itself and its wrapped copy.
    pub fn spans(&self, i: usize, t: f32) -> [(f32, f32); 2] {
        let x = self.x(i, t);
        let len = self.items[i].len;
        [(x, x + len), (x - PERIOD, x - PERIOD + len)]
    }
}

/// A drawbridge is down for a while, goes up, stays up, comes down.
pub const BRIDGE_CYCLE: f32 = 7.0;
const BRIDGE_DOWN: f32 = 4.0;
const BRIDGE_UP: f32 = 4.8;

/// Diving things dive on a cycle; each item has its own phase. Drawbridges
/// on one lane go up together.
pub fn dive(item: &Item, t: f32) -> Surface {
    if !item.kind.dives() {
        return Surface::Up;
    }
    if item.kind == Kind::Bridge {
        let p = (t + 1.5).rem_euclid(BRIDGE_CYCLE);
        return if p < BRIDGE_DOWN {
            Surface::Up
        } else if p < BRIDGE_UP {
            Surface::Sinking((p - BRIDGE_DOWN) / (BRIDGE_UP - BRIDGE_DOWN))
        } else {
            Surface::Under
        };
    }
    let p = (t + item.x0 * 0.173).rem_euclid(DIVE_CYCLE);
    if p < DIVE_SINK {
        Surface::Up
    } else if p < DIVE_UNDER {
        Surface::Sinking((p - DIVE_SINK) / (DIVE_UNDER - DIVE_SINK))
    } else {
        Surface::Under
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Dir {
    pub fn dx(self) -> f32 {
        match self {
            Dir::Left | Dir::UpLeft | Dir::DownLeft => -2.0,
            Dir::Right | Dir::UpRight | Dir::DownRight => 2.0,
            _ => 0.0,
        }
    }

    pub fn dy(self) -> i32 {
        match self {
            Dir::Up | Dir::UpLeft | Dir::UpRight => -1,
            Dir::Down | Dir::DownLeft | Dir::DownRight => 1,
            _ => 0,
        }
    }

    pub fn diagonal(self) -> bool {
        self.dx() != 0.0 && self.dy() != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Death {
    Squashed,
    Drowned,
    Swept,
    TimeUp,
    /// Fell into the pond: back to the bank, nothing lost.
    Splash,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub rows: Vec<Lane>,
    /// The pond: diagonal hops, sinking pads, nothing to cross to.
    pub pond: bool,
    /// For lily pads: when the toad landed on each (row, item), if lately.
    pub landed: Vec<Vec<Option<f32>>>,
}

impl Level {
    pub fn new(rows: Vec<Lane>, pond: bool) -> Level {
        let landed = rows.iter().map(|r| vec![None; r.items.len()]).collect();
        Level { rows, pond, landed }
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Is floater i on row r there to stand on?
    pub fn surface(&self, r: usize, i: usize, t: f32) -> Surface {
        let it = &self.rows[r].items[i];
        if it.kind == Kind::Pad {
            if let Some(t0) = self.landed[r][i] {
                let d = t - t0;
                if (0.0..PAD_WOBBLE).contains(&d) {
                    return Surface::Up;
                } else if (PAD_WOBBLE..PAD_UNDER).contains(&d) {
                    return Surface::Sinking((d - PAD_WOBBLE) / (PAD_UNDER - PAD_WOBBLE));
                } else if (PAD_UNDER..PAD_BACK).contains(&d) {
                    return Surface::Under;
                }
            }
            return Surface::Up;
        }
        dive(it, t)
    }

    /// The floater under a toad at x on row r, if any.
    pub fn floater(&self, r: usize, x: f32, t: f32) -> Option<usize> {
        let c = x + 1.0;
        let lane = &self.rows[r];
        (0..lane.items.len()).find(|&i| {
            lane.items[i].kind.role() == Role::Floater
                && self.surface(r, i, t) != Surface::Under
                && lane
                    .spans(i, t)
                    .iter()
                    .any(|&(a, b)| c >= a - 0.25 && c <= b + 0.25)
        })
    }

    /// Does anything on row r run the toad at x over?
    pub fn hazard(&self, r: usize, x: f32, t: f32) -> bool {
        let (l, rr) = (x + 0.5, x + 1.5);
        let lane = &self.rows[r];
        (0..lane.items.len()).any(|i| {
            lane.items[i].kind.role() == Role::Hazard
                && lane.spans(i, t).iter().any(|&(a, b)| rr > a && l < b)
        })
    }

    /// Is the tile at x on row r taken by something solid?
    pub fn blocked(&self, r: usize, x: f32) -> bool {
        let (l, rr) = (x + 0.3, x + 1.7);
        let lane = &self.rows[r];
        (0..lane.items.len()).any(|i| {
            lane.items[i].kind.role() == Role::Blocker
                && lane.spans(i, 0.0).iter().any(|&(a, b)| rr > a && l < b)
        })
    }

    /// Is a train about to come through on row r?
    pub fn train_coming(&self, r: usize, t: f32) -> bool {
        let lane = &self.rows[r];
        if lane.ground != Ground::Rail {
            return false;
        }
        let right = lane.velocity(t) > 0.0;
        (0..lane.items.len()).any(|i| {
            lane.items[i].kind == Kind::Train
                && lane.spans(i, t).iter().any(|&(a, b)| {
                    if right {
                        (-45.0..2.0).contains(&b)
                    } else {
                        (W - 2.0..W + 45.0).contains(&a)
                    }
                })
        })
    }

    /// The toad landed on a lily pad: it starts to sink (once).
    pub fn land(&mut self, r: usize, i: usize, t: f32) {
        if self.rows[r].items[i].kind != Kind::Pad {
            return;
        }
        let fresh = match self.landed[r][i] {
            Some(t0) => t - t0 >= PAD_BACK,
            None => true,
        };
        if fresh {
            self.landed[r][i] = Some(t);
        }
    }
}

/// What a tick did to the toad.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Step {
    Still,
    /// Hopped; `slid` when the ice made it.
    Hopped {
        slid: bool,
    },
    Bumped,
    Died(Death),
    Crossed,
}

#[derive(Clone, Debug)]
pub struct Toad {
    pub x: f32,
    pub row: usize,
    pub cool: u32,
    pub slide: Option<Dir>,
    pub facing: Dir,
    /// Ticks since the last hop, for the hop pose.
    pub air: u32,
}

impl Toad {
    pub fn start(lv: &Level, x: f32) -> Toad {
        Toad {
            x,
            row: lv.height() - 1,
            cool: 0,
            slide: None,
            facing: Dir::Up,
            air: 99,
        }
    }

    /// One tick at time t (the time after this tick), wanting to hop `want`.
    pub fn tick(&mut self, lv: &Level, t: f32, want: Option<Dir>) -> Step {
        let mut step = Step::Still;
        self.air = self.air.saturating_add(1);
        // ride whatever floats
        if lv.rows[self.row].ground.wet() && lv.floater(self.row, self.x, t - DT).is_some() {
            self.x += lv.rows[self.row].velocity(t) * DT;
        }
        self.cool = self.cool.saturating_sub(1);
        if self.cool == 0 {
            let slid = self.slide.is_some();
            let dir = self
                .slide
                .take()
                .or(want)
                .filter(|d| !d.diagonal() || lv.pond);
            if let Some(d) = dir {
                let nr = self.row as i32 + d.dy();
                let nx = (self.x + d.dx()).clamp(0.0, W - 2.0);
                let moved = nr != self.row as i32 || (nx - self.x).abs() > 0.01;
                if nr < 0 || nr >= lv.height() as i32 || !moved {
                    if !slid {
                        step = Step::Bumped;
                    }
                } else if lv.blocked(nr as usize, nx) {
                    step = Step::Bumped;
                } else {
                    self.row = nr as usize;
                    self.x = nx;
                    self.cool = HOP_TICKS;
                    self.facing = d;
                    self.air = 0;
                    if lv.rows[self.row].ground == Ground::Ice && !d.diagonal() {
                        self.slide = Some(d);
                    }
                    step = Step::Hopped { slid };
                }
            }
        }
        if self.row == 0 && !lv.pond {
            return Step::Crossed;
        }
        if self.x < -0.75 || self.x > W - 1.25 {
            return Step::Died(Death::Swept);
        }
        if lv.rows[self.row].ground.wet() && lv.floater(self.row, self.x, t).is_none() {
            return Step::Died(if lv.pond {
                Death::Splash
            } else {
                Death::Drowned
            });
        }
        if lv.hazard(self.row, self.x, t) {
            return Step::Died(Death::Squashed);
        }
        step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lv(rows: &[(&str, f32, &str)]) -> Level {
        Level::new(
            rows.iter()
                .map(|&(g, s, p)| Lane::new(Ground::parse(g), s, 0.0, p))
                .collect(),
            false,
        )
    }

    #[test]
    fn patterns_fill_the_period() {
        let l = Lane::new(Ground::Road, 3.0, 0.0, "..cccc....");
        assert_eq!(l.items.len(), 10);
        assert!(l.items.iter().all(|i| i.len == 4.0));
        let l = Lane::new(Ground::Road, 3.0, 0.0, "ccc.tt..");
        assert_eq!(l.items.len(), 25);
    }

    #[test]
    fn things_move_and_wrap() {
        let l = Lane::new(Ground::Road, 10.0, 0.0, &format!("c{}", ".".repeat(99)));
        assert_eq!(l.x(0, 0.0), LEFT);
        assert!((l.x(0, 1.0) - (LEFT + 10.0)).abs() < 1e-4);
        assert!((l.x(0, 10.0) - LEFT).abs() < 1e-3);
    }

    #[test]
    fn hop_up_and_cross() {
        let level = lv(&[("bank", 0.0, ""), ("grass", 0.0, ""), ("grass", 0.0, "")]);
        let mut toad = Toad::start(&level, START_X);
        assert_eq!(
            toad.tick(&level, DT, Some(Dir::Up)),
            Step::Hopped { slid: false }
        );
        let mut t = DT;
        let mut last = Step::Still;
        for _ in 0..HOP_TICKS {
            t += DT;
            last = toad.tick(&level, t, Some(Dir::Up));
        }
        assert_eq!(last, Step::Crossed);
    }

    #[test]
    fn cars_squash_water_drowns_logs_carry() {
        let level = lv(&[
            ("bank", 0.0, ""),
            (
                "water",
                5.0,
                &format!("{}{}", ".".repeat(46), "l".repeat(8)),
            ),
            ("road", 0.0, ""),
            ("grass", 0.0, ""),
        ]);
        let mut toad = Toad {
            x: START_X,
            row: 2,
            cool: 0,
            slide: None,
            facing: Dir::Up,
            air: 0,
        };
        // the log starts at column 36..44 at t=0
        toad.tick(&level, DT, Some(Dir::Up));
        assert_eq!(toad.row, 1);
        let x0 = toad.x;
        for k in 2..30 {
            assert_eq!(toad.tick(&level, k as f32 * DT, None), Step::Still);
        }
        assert!(toad.x > x0 + 2.0, "carried along: {} -> {}", x0, toad.x);
        let dry = lv(&[("bank", 0.0, ""), ("water", 0.0, ""), ("grass", 0.0, "")]);
        let mut toad = Toad::start(&dry, START_X);
        assert_eq!(
            toad.tick(&dry, DT, Some(Dir::Up)),
            Step::Died(Death::Drowned)
        );
        let road = lv(&[
            ("bank", 0.0, ""),
            ("road", 0.0, &format!("{}{}", ".".repeat(48), "cccc")),
            ("grass", 0.0, ""),
        ]);
        let mut toad = Toad::start(&road, START_X);
        assert_eq!(
            toad.tick(&road, DT, Some(Dir::Up)),
            Step::Died(Death::Squashed)
        );
    }

    #[test]
    fn hedges_block_and_ice_slides() {
        let level = lv(&[
            ("bank", 0.0, ""),
            ("grass", 0.0, ""),
            ("ice", 0.0, ""),
            ("grass", 0.0, &format!("{}##", ".".repeat(48))),
            ("grass", 0.0, ""),
        ]);
        let mut toad = Toad::start(&level, START_X);
        assert_eq!(toad.tick(&level, DT, Some(Dir::Up)), Step::Bumped);
        toad.x = 34.0;
        assert_eq!(
            toad.tick(&level, 2.0 * DT, Some(Dir::Up)),
            Step::Hopped { slid: false }
        );
        let mut t = 2.0 * DT;
        for k in 0..HOP_TICKS {
            t += DT;
            toad.tick(
                &level,
                t,
                if k == HOP_TICKS - 1 {
                    Some(Dir::Up)
                } else {
                    None
                },
            );
        }
        assert_eq!(toad.row, 2, "landed on the ice");
        for _ in 0..HOP_TICKS {
            t += DT;
            toad.tick(&level, t, None);
        }
        assert_eq!(toad.row, 1, "and slid on by itself");
    }

    #[test]
    fn diagonals_only_in_the_pond() {
        let mut level = lv(&[("bank", 0.0, ""), ("grass", 0.0, ""), ("grass", 0.0, "")]);
        let mut toad = Toad::start(&level, START_X);
        assert_eq!(toad.tick(&level, DT, Some(Dir::UpRight)), Step::Still);
        level.pond = true;
        assert_eq!(
            toad.tick(&level, 2.0 * DT, Some(Dir::UpRight)),
            Step::Hopped { slid: false }
        );
        assert_eq!((toad.row, toad.x), (1, START_X + 2.0));
    }

    #[test]
    fn pads_sink_when_landed_on() {
        let mut level = Level::new(
            vec![
                Lane::new(Ground::Bank, 0.0, 0.0, ""),
                Lane::new(Ground::Pond, 0.0, 0.0, &format!("{}pppp", ".".repeat(48))),
                Lane::new(Ground::Bank, 0.0, 0.0, ""),
            ],
            true,
        );
        let i = level
            .floater(1, START_X, 0.0)
            .expect("a pad above the start");
        level.land(1, i, 1.0);
        assert_eq!(level.surface(1, i, 1.2), Surface::Up);
        assert!(matches!(level.surface(1, i, 1.7), Surface::Sinking(_)));
        assert_eq!(level.surface(1, i, 2.5), Surface::Under);
        assert!(level.floater(1, START_X, 2.5).is_none());
        assert_eq!(level.surface(1, i, 4.5), Surface::Up);
    }
}
