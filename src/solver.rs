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

#[cfg(test)]
mod human {
    use super::*;
    use crate::levels;
    use crate::world::START_X;

    /// Ticks between decisions: about four hops a second at most.
    const EVERY: u64 = 14;
    /// How far ahead the player looks, in decisions.
    const DEPTH: usize = 3;

    /// Does `plan` (one action per decision) keep the toad alive, judging
    /// the lanes `late` ticks behind the truth? Rows gained, or None.
    fn judge(lv: &Level, toad: &Toad, now: u64, late: u64, plan: &[Option<Dir>]) -> Option<i32> {
        let mut t = toad.clone();
        let start = t.row as i32;
        for (d, &a) in plan.iter().enumerate() {
            for k in 0..EVERY {
                let tick = (now + d as u64 * EVERY + k + 1).saturating_sub(late);
                match t.tick(lv, tick as f32 * DT, if k == 0 { a } else { None }) {
                    Step::Died(_) => return None,
                    Step::Crossed => return Some(10_000),
                    _ => {}
                }
            }
        }
        // a player sees the gaps in a hedge from afar: count being near one
        let reach = if t.row == 0 {
            0
        } else {
            (0..40)
                .find(|&k| {
                    let dx = k as f32;
                    [t.x - dx, t.x + dx]
                        .iter()
                        .any(|&x| (0.0..78.0).contains(&x) && !lv.blocked(t.row - 1, x))
                })
                .unwrap_or(40)
        };
        Some((start - t.row as i32) * 100 - reach)
    }

    /// The best first move a short-sighted player sees.
    fn choose(lv: &Level, toad: &Toad, now: u64, late: u64) -> Option<Dir> {
        let mut best: Option<(i32, usize, Option<Dir>)> = None;
        let n = ACTIONS.len();
        for code in 0..n.pow(DEPTH as u32) {
            let mut c = code;
            let plan: Vec<Option<Dir>> = (0..DEPTH)
                .map(|_| {
                    let a = ACTIONS[c % n];
                    c /= n;
                    a
                })
                .collect();
            if let Some(g) = judge(lv, toad, now, late, &plan) {
                // prefer gains, then the earlier actions in ACTIONS' order
                let rank = code % n;
                if best.is_none_or(|(bg, br, _)| g > bg || (g == bg && rank < br)) {
                    best = Some((g, rank, plan[0]));
                }
            }
        }
        best.map_or(None, |b| b.2)
    }

    /// One crossing played by a short-sighted player from tick `t0`: deaths
    /// before getting across (gives up at 10) and seconds taken by the last try.
    fn play(lv: &Level, t0: u64, late: u64, limit: f32, why: &mut Vec<String>) -> (u32, f32) {
        // a boss road's chaser and resting banks, as the game runs them
        let mut lv = lv.clone();
        let lv = &mut lv;
        let mut deaths = 0;
        let mut now = t0;
        let start = now;
        let mut check = lv.height() - 1;
        'tries: while deaths < 10 {
            let mut toad = Toad::start_at(check, START_X);
            if let Some(ch) = &mut lv.chase {
                ch.restart(check, now as f32 * DT);
            }
            let begin = now;
            loop {
                if (now - begin) as f32 * DT > limit {
                    deaths += 1;
                    continue 'tries;
                }
                let a = choose(lv, &toad, now, late);
                for k in 0..EVERY {
                    now += 1;
                    match toad.tick(lv, now as f32 * DT, if k == 0 { a } else { None }) {
                        Step::Died(d) => {
                            why.push(format!("{d:?}@{}", toad.row));
                            deaths += 1;
                            now += 60;
                            continue 'tries;
                        }
                        Step::Crossed => return (deaths, (now - start) as f32 * DT),
                        Step::Hopped { .. }
                            if toad.row < check && lv.checkpoints.contains(&toad.row) =>
                        {
                            check = toad.row;
                            if let Some(ch) = &mut lv.chase {
                                ch.restart(check, now as f32 * DT);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        (deaths, limit)
    }

    /// How a short-sighted player with a slow eye fares on each crossing:
    /// `cargo test human -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn human_report() {
        for (n, s) in levels::story().iter().enumerate() {
            let lv = s.build();
            if lv.pond {
                continue;
            }
            for late in [6u64, 12] {
                let mut why = Vec::new();
                let runs: Vec<(u32, f32)> = (0..24)
                    .map(|i| play(&lv, i * 37, late, s.time, &mut why))
                    .collect();
                why.sort();
                let mut tally: Vec<(usize, String)> = Vec::new();
                for w in why {
                    match tally.iter_mut().find(|t| t.1 == w) {
                        Some(t) => t.0 += 1,
                        None => tally.push((1, w)),
                    }
                }
                tally.sort_by(|a, b| b.0.cmp(&a.0));
                tally.truncate(4);
                let deaths: u32 = runs.iter().map(|r| r.0).sum();
                let worst = runs.iter().map(|r| r.0).max().unwrap_or(0);
                let secs: f32 = runs.iter().map(|r| r.1).sum::<f32>() / runs.len() as f32;
                println!(
                    "{:>2} {:<24} eye {:>2} deaths per try {:>4.2} worst {:>2} secs {:>5.1} of {}",
                    n + 1,
                    s.name,
                    late,
                    deaths as f32 / runs.len() as f32,
                    worst,
                    secs,
                    s.time
                );
                println!("      {tally:?}");
            }
        }
    }
}
