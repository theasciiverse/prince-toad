//! Endless roads: levels made up as you go, harder the further you get.
//! Every one is checked with the solver before it is played, so there is
//! always a way across.

use crate::solver;
use crate::world::{DT, Ground, Lane, Level, START_X, Toad};

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next() % n.max(1) as u64) as u32
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (self.next() % 10_000) as f32 / 10_000.0 * (b - a)
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.range(0.0, 1.0) < p
    }
}

/// A generated level and what to call it.
pub struct Made {
    pub level: Level,
    pub world: usize,
    pub night: bool,
    /// Fog: only the lanes near the toads can be seen.
    pub fog: bool,
    pub rain: bool,
    pub time: f32,
}

/// Traffic of one world: (letter, length).
fn traffic(world: usize) -> &'static [(char, usize)] {
    match world {
        0 => &[('k', 5), ('b', 2), ('r', 4), ('g', 2)],
        1 => &[('c', 4), ('c', 4), ('t', 8), ('m', 2)],
        2 => &[('c', 4), ('k', 5), ('b', 2)],
        3 => &[('z', 2), ('c', 4), ('s', 10)],
        4 => &[('c', 4), ('t', 8), ('m', 2)],
        _ => &[('h', 1), ('a', 6), ('w', 1), ('f', 3)],
    }
}

fn floats(world: usize) -> &'static [(char, usize)] {
    match world {
        3 => &[('i', 8), ('I', 6), ('i', 6)],
        5 => &[('x', 3), ('d', 4), ('D', 4), ('x', 4)],
        _ => &[('l', 8), ('l', 6), ('u', 6), ('U', 6), ('l', 10)],
    }
}

/// A 100-column pattern of things from `kinds`, with gaps at least `gap`.
fn pattern(rng: &mut Rng, kinds: &[(char, usize)], gap: usize, extra: usize) -> String {
    let mut p = vec!['.'; 100];
    let mut x = rng.below(10) as usize;
    while x < 100 {
        let (c, len) = kinds[rng.below(kinds.len() as u32) as usize];
        if x + len + gap > 100 {
            break;
        }
        for cell in p.iter_mut().skip(x).take(len) {
            *cell = c;
        }
        x += len + gap + rng.below(extra as u32 + 1) as usize;
    }
    p.into_iter().collect()
}

fn safe_row(rng: &mut Rng, world: usize) -> Lane {
    let (ground, deco) = match world {
        1 => (Ground::Sand, '^'),
        3 => (Ground::Snow, 'o'),
        5 => (Ground::Cobble, 'o'),
        _ => (Ground::Grass, if rng.chance(0.4) { '#' } else { 'y' }),
    };
    let mut p = vec!['.'; 100];
    for _ in 0..rng.below(4) {
        let x = rng.below(96) as usize;
        let len = if deco == '#' {
            3 + rng.below(4) as usize
        } else {
            1
        };
        for c in p.iter_mut().skip(x).take(len) {
            *c = deco;
        }
    }
    // never block the start
    for c in p.iter_mut().skip(46).take(6) {
        *c = '.';
    }
    Lane::new(ground, 0.0, 0.0, &p.into_iter().collect::<String>())
}

/// One endless level at difficulty `d` (0, 1, 2, ... crossings made).
pub fn make(d: u32, seed: u64) -> Made {
    for attempt in 0..40u64 {
        let mut rng = Rng::new(
            seed.wrapping_add(attempt * 7919)
                .wrapping_add(d as u64 * 104_729),
        );
        // after many tries, ease off
        let d = if attempt >= 20 { d / 2 } else { d };
        let world = ((d / 3) % 6) as usize;
        let night = world == 4;
        let fog = matches!(world, 0 | 2) && d >= 4 && rng.chance(0.3);
        let rain = matches!(world, 1 | 4) && d >= 4 && rng.chance(0.3);
        let lanes_total = (4 + d.min(10)) as usize;
        let speed_base = 2.5 + d.min(20) as f32 * 0.35;
        let road_ground = match world {
            0 => Ground::Dirt,
            3 => Ground::Snow,
            5 => Ground::Cobble,
            _ => Ground::Road,
        };
        let water_ground = if world == 5 {
            Ground::Moat
        } else {
            Ground::Water
        };
        let mut rows: Vec<Lane> = vec![Lane::new(
            Ground::Bank,
            0.0,
            0.0,
            "..y.....T.......y.........y......T.........y.....",
        )];
        let mut made = 0;
        let mut dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
        while made < lanes_total {
            let block = (2 + rng.below(3) as usize).min(lanes_total - made);
            let wet = d >= 2 && rng.chance(if world == 2 { 0.7 } else { 0.35 });
            for _ in 0..block {
                dir = -dir;
                if wet && world != 3 && rng.chance(0.15) {
                    // stepping stones of old planks, going nowhere
                    let mut p = vec!['.'; 100];
                    for _ in 0..2 + rng.below(3) {
                        let x = rng.below(94) as usize;
                        for c in p.iter_mut().skip(x).take(4 + rng.below(3) as usize) {
                            *c = if world == 5 { 'x' } else { 'l' };
                        }
                    }
                    rows.push(Lane::new(
                        water_ground,
                        0.0,
                        0.0,
                        &p.into_iter().collect::<String>(),
                    ));
                } else if wet {
                    let speed = dir * rng.range(1.5, 2.0 + speed_base * 0.4);
                    let gap = 4 + rng.below(4) as usize;
                    let p = pattern(&mut rng, floats(world), gap, 6);
                    rows.push(Lane::new(water_ground, speed, 0.0, &p));
                } else if (world == 1 || world == 4) && d >= 3 && rng.chance(0.12) {
                    let train = format!(
                        "{}{}",
                        "R".repeat(20 + rng.below(12) as usize),
                        ".".repeat(70)
                    );
                    rows.push(Lane::new(
                        Ground::Rail,
                        dir * rng.range(20.0, 26.0),
                        0.0,
                        &train,
                    ));
                } else if world == 3 && rng.chance(0.25) {
                    rows.push(Lane::new(Ground::Ice, 0.0, 0.0, ""));
                } else {
                    let speed = dir * rng.range(speed_base * 0.7, speed_base * 1.3);
                    let gap = (8.0 + speed.abs() * 0.8) as usize;
                    let p = pattern(&mut rng, traffic(world), gap, 20);
                    let wobble = if world == 3 && rng.chance(0.4) {
                        2.0
                    } else {
                        0.0
                    };
                    rows.push(Lane::new(road_ground, speed, wobble, &p));
                }
                made += 1;
            }
            rows.push(safe_row(&mut rng, world));
        }
        let mut level = Level::new(rows, false);
        level.rain = rain;
        let time = 24.0 + level.height() as f32 * 1.2;
        let toad = Toad::start(&level, START_X);
        if let Some(plan) = solver::solve(&level, &toad, 0, (time / DT) as u64)
            && (plan.len() as f32 * 8.0 * DT) < time * 0.7
        {
            return Made {
                level,
                world,
                night,
                fog,
                rain,
                time,
            };
        }
        log::debug!("endless level {d} attempt {attempt} has no easy way across");
    }
    // a plain field: always crossable
    let level = Level::new(
        vec![
            Lane::new(Ground::Bank, 0.0, 0.0, ""),
            Lane::new(Ground::Grass, 0.0, 0.0, ""),
        ],
        false,
    );
    Made {
        level,
        world: 0,
        night: false,
        fog: false,
        rain: false,
        time: 30.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endless_levels_fit_and_can_be_crossed() {
        for d in 0..24 {
            let m = make(d, 1234 + d as u64);
            assert!(
                m.level.height() >= 5 && m.level.height() <= 22,
                "d {d}: {} rows",
                m.level.height()
            );
            assert!(!m.level.rows.last().unwrap().ground.wet());
        }
    }
}
