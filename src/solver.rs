//! Finds a way across: a breadth-first search over hops, one decision every
//! HOP_TICKS ticks, simulated with the very same `Toad::tick` the game uses.
//! It proves in the tests that every crossing can be made, and it plays the
//! demo. Lily pads that sink are not modelled; in the pond it simply never
//! waits, and reaching the far bank counts as across.

use crate::world::{DT, Dir, HOP_TICKS, Level, Step, Toad};
use std::collections::HashSet;

const ACTIONS: [Option<Dir>; 5] = [
    Some(Dir::Up),
    None,
    Some(Dir::Left),
    Some(Dir::Right),
    Some(Dir::Down),
];
/// In the pond the toad never sits still (pads sink) and may hop diagonally.
const POND_ACTIONS: [Option<Dir>; 6] = [
    Some(Dir::Up),
    Some(Dir::UpLeft),
    Some(Dir::UpRight),
    Some(Dir::Left),
    Some(Dir::Right),
    Some(Dir::Down),
];

struct Node {
    toad: Toad,
    parent: usize,
    action: Option<Dir>,
}

/// A plan: one action per decision, starting at tick `from`, that crosses
/// within `max_ticks`. None when there is no way.
pub fn solve(lv: &Level, toad: &Toad, from: u64, max_ticks: u64) -> Option<Vec<Option<Dir>>> {
    solve_every(lv, toad, from, max_ticks, HOP_TICKS as u64)
}

/// The same, deciding only every `every` ticks (a multiple of HOP_TICKS):
/// a slower, more human pace.
pub fn solve_every(
    lv: &Level,
    toad: &Toad,
    from: u64,
    max_ticks: u64,
    every: u64,
) -> Option<Vec<Option<Dir>>> {
    let mut layers: Vec<Vec<Node>> = vec![vec![Node {
        toad: toad.clone(),
        parent: 0,
        action: None,
    }]];
    let decisions = max_ticks / every;
    for d in 0..decisions {
        let base = from + d * every;
        let mut next = Vec::new();
        let mut seen = HashSet::new();
        for (pi, node) in layers.last().unwrap().iter().enumerate() {
            let actions: &[Option<Dir>] = if lv.pond { &POND_ACTIONS } else { &ACTIONS };
            for &action in actions {
                let mut t = node.toad.clone();
                let mut alive = true;
                let mut crossed = false;
                for k in 0..every {
                    let step = t.tick(
                        lv,
                        (base + k + 1) as f32 * DT,
                        if k == 0 { action } else { None },
                    );
                    match step {
                        Step::Died(_) => {
                            alive = false;
                            break;
                        }
                        Step::Crossed => {
                            crossed = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if lv.pond && t.row == 0 {
                    crossed = true;
                }
                if crossed {
                    let mut plan = vec![action];
                    let mut li = layers.len() - 1;
                    let mut i = pi;
                    while li > 0 {
                        let n = &layers[li][i];
                        plan.push(n.action);
                        i = n.parent;
                        li -= 1;
                    }
                    plan.reverse();
                    return Some(plan);
                }
                if !alive {
                    continue;
                }
                let key = (t.row, (t.x * 2.0).round() as i32, t.slide.is_some(), t.cool);
                if seen.insert(key) {
                    next.push(Node {
                        toad: t,
                        parent: pi,
                        action,
                    });
                }
            }
        }
        if next.is_empty() {
            return None;
        }
        layers.push(next);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels;
    use crate::world::START_X;

    #[test]
    fn every_crossing_can_be_made_in_time() {
        for s in levels::story().iter() {
            let lv = s.build();
            let toad = Toad::start(&lv, START_X);
            let budget = (if lv.pond { 60.0 } else { s.time } / DT) as u64;
            let plan =
                solve(&lv, &toad, 0, budget).unwrap_or_else(|| panic!("{}: no way across", s.name));
            let secs = plan.len() as f32 * HOP_TICKS as f32 * DT;
            assert!(
                lv.pond || secs < s.time * 0.6,
                "{}: {secs:.1}s of {} is too tight",
                s.name,
                s.time
            );
            // and the plan really works when replayed, pads sinking and all
            let mut lv = lv.clone();
            let mut t = toad.clone();
            let mut tick = 0u64;
            let mut done = false;
            'outer: for a in &plan {
                for k in 0..HOP_TICKS {
                    tick += 1;
                    let step = t.tick(&lv, tick as f32 * DT, if k == 0 { *a } else { None });
                    if let Step::Hopped { .. } = step
                        && let Some(i) = lv.floater(t.row, t.x, tick as f32 * DT)
                    {
                        lv.land(t.row, i, tick as f32 * DT);
                    }
                    if lv.pond && t.row == 0 {
                        done = true;
                        break 'outer;
                    }
                    match step {
                        Step::Crossed => {
                            done = true;
                            break 'outer;
                        }
                        Step::Died(d) => panic!("{}: replay died {d:?}", s.name),
                        _ => {}
                    }
                }
            }
            assert!(done, "{}: replay did not cross", s.name);
        }
    }
}

#[cfg(test)]
mod report {
    use super::*;
    use crate::levels;
    use crate::world::START_X;

    /// How long the fastest way across takes, flat out and at a human pace:
    /// `cargo test report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn crossing_times() {
        for (n, s) in levels::story().iter().enumerate() {
            let lv = s.build();
            let toad = Toad::start(&lv, START_X);
            let budget = (if lv.pond { 60.0 } else { s.time } / DT) as u64;
            let fast = solve(&lv, &toad, 0, budget)
                .map_or(-1.0, |p| p.len() as f32 * HOP_TICKS as f32 * DT);
            let calm = solve_every(&lv, &toad, 0, budget, 3 * HOP_TICKS as u64)
                .map_or(-1.0, |p| p.len() as f32 * 3.0 * HOP_TICKS as f32 * DT);
            println!(
                "{:>2} {:<24} rows {:>2} time {:>4} fastest {:>5.1}s at a third of the pace {:>5.1}s",
                n + 1,
                s.name,
                lv.height(),
                s.time,
                fast,
                calm
            );
        }
    }
}
