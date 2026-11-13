//! All sound is made here in code; Prince Toad ships no audio files.
//!
//! Every effect is rendered once at start-up into a sample buffer and
//! played on rodio's mixer. Without an audio device, or with `--mute`,
//! every call does nothing. Layout after tspace's.

use rodio::MixerDeviceSink;
use rodio::buffer::SamplesBuffer;
use std::f32::consts::TAU;
use std::num::NonZero;
use std::time::{Duration, Instant};

const SR: u32 = 44_100;
const SRF: f32 = SR as f32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Croak1,
    Croak2,
    Croak3,
    Slide,
    Bump,
    Splat,
    Splash,
    Plop,
    Kiss,
    Crossed,
    Tick,
    GameOver,
    Giggle,
    Fanfare,
    Select,
    Honk,
    Life,
}

const ALL_SFX: [Sfx; 17] = [
    Sfx::Croak1,
    Sfx::Croak2,
    Sfx::Croak3,
    Sfx::Slide,
    Sfx::Bump,
    Sfx::Splat,
    Sfx::Splash,
    Sfx::Plop,
    Sfx::Kiss,
    Sfx::Crossed,
    Sfx::Tick,
    Sfx::GameOver,
    Sfx::Giggle,
    Sfx::Fanfare,
    Sfx::Select,
    Sfx::Honk,
    Sfx::Life,
];

pub struct Audio {
    // the sink must outlive everything playing on its mixer
    sink: Option<MixerDeviceSink>,
    bank: Vec<SamplesBuffer>,
    pub muted: bool,
    last: [Option<Instant>; ALL_SFX.len()],
    croak: usize,
}

impl Audio {
    pub fn new(muted: bool) -> Audio {
        let sink = if muted {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        let bank = if sink.is_some() {
            ALL_SFX.iter().map(|&s| buffer(render_sfx(s))).collect()
        } else {
            Vec::new()
        };
        Audio {
            sink,
            bank,
            muted,
            last: [None; ALL_SFX.len()],
            croak: 0,
        }
    }

    /// No sound at all, for tests.
    #[cfg(test)]
    pub fn silent() -> Audio {
        Audio {
            sink: None,
            bank: Vec::new(),
            muted: true,
            last: [None; ALL_SFX.len()],
            croak: 0,
        }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    pub fn play(&mut self, sfx: Sfx) {
        if self.muted {
            return;
        }
        let Some(sink) = &self.sink else { return };
        let i = ALL_SFX.iter().position(|&s| s == sfx).unwrap_or(0);
        let now = Instant::now();
        if self.last[i].is_some_and(|t| now.duration_since(t) < Duration::from_millis(40)) {
            return;
        }
        self.last[i] = Some(now);
        sink.mixer().add(self.bank[i].clone());
    }

    /// A croak per hop, never quite the same twice in a row.
    pub fn croak(&mut self) {
        self.croak = (self.croak + 1) % 3;
        self.play([Sfx::Croak1, Sfx::Croak2, Sfx::Croak3][self.croak]);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }
}

fn buffer(samples: Vec<f32>) -> SamplesBuffer {
    SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(SR).unwrap(), samples)
}

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn tri(ph: f32) -> f32 {
    let p = ph - ph.floor();
    4.0 * (p - 0.5).abs() - 1.0
}

fn square(ph: f32) -> f32 {
    if ph - ph.floor() < 0.5 { 1.0 } else { -1.0 }
}

/// `dur` seconds of samples from `f(t, noise)`.
fn synth(dur: f32, mut f: impl FnMut(f32, &mut Noise) -> f32) -> Vec<f32> {
    let mut n = Noise(0x1234_5678);
    (0..(dur * SRF) as usize)
        .map(|i| f(i as f32 / SRF, &mut n))
        .collect()
}

/// A little tune: (midi note, seconds), 0 a rest, with a soft attack.
fn tune(notes: &[(f32, f32)], gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut ph = 0.0f32;
    for &(n, d) in notes {
        let len = (d * SRF) as usize;
        for i in 0..len {
            let t = i as f32 / SRF;
            if n <= 0.0 {
                out.push(0.0);
                continue;
            }
            ph += midi(n) / SRF;
            let env = (t / 0.01).min(1.0) * ((d - t) / 0.03).clamp(0.0, 1.0);
            out.push(wave(ph) * env * gain);
        }
    }
    out
}

/// A toad's croak: a buzzy low tone in a few quick pulses, bending down.
fn croak(base: f32, dur: f32, gain: f32) -> Vec<f32> {
    let (mut ph, mut lp) = (0.0f32, 0.0f32);
    synth(dur, |t, n| {
        ph += base * (1.0 - 0.25 * t / dur) / SRF;
        let pulse = 0.55 + 0.45 * (TAU * 38.0 * t).sin();
        let raw = square(ph) * 0.6 + tri(ph * 2.0) * 0.3 + n.next() * 0.1;
        lp += (raw - lp) * 0.25;
        let env = (t / 0.008).min(1.0) * (1.0 - t / dur).max(0.0);
        lp * pulse * env * gain
    })
}

fn render_sfx(s: Sfx) -> Vec<f32> {
    match s {
        Sfx::Croak1 => croak(150.0, 0.11, 0.32),
        Sfx::Croak2 => croak(168.0, 0.10, 0.30),
        Sfx::Croak3 => croak(140.0, 0.12, 0.32),
        Sfx::Slide => {
            let mut lp = 0.0f32;
            synth(0.18, |t, n| {
                lp += (n.next() - lp) * (0.4 - t);
                lp * (1.0 - t / 0.18) * 0.25
            })
        }
        Sfx::Bump => synth(0.06, |t, n| {
            ((TAU * 110.0 * t).sin() * 0.7 + n.next() * 0.3) * (-t * 60.0).exp() * 0.35
        }),
        Sfx::Splat => {
            let (mut ph, mut lp) = (0.0f32, 0.0f32);
            synth(0.4, |t, n| {
                ph += (300.0 * (-t * 9.0).exp() + 50.0) / SRF;
                lp += (n.next() - lp) * 0.2;
                ((ph * TAU).sin() * 0.5 + lp * 0.8).clamp(-1.0, 1.0) * (-t * 7.0).exp() * 0.6
            })
        }
        Sfx::Splash => {
            let (mut lp, mut hp) = (0.0f32, 0.0f32);
            synth(0.7, |t, n| {
                let x = n.next();
                lp += (x - lp) * 0.3;
                hp = x - lp;
                let bub = (TAU * (400.0 + 900.0 * t) * t).sin() * (-t * 10.0).exp() * 0.3;
                (hp * 0.6 * (-t * 4.0).exp() + bub).clamp(-1.0, 1.0) * 0.55
            })
        }
        // a soft drop into the pond
        Sfx::Plop => {
            let mut ph = 0.0f32;
            synth(0.25, |t, _| {
                ph += (700.0 * (-t * 14.0).exp() + 180.0) / SRF;
                (ph * TAU).sin() * (-t * 12.0).exp() * 0.4
            })
        }
        // mwah: a little breath, a rising whistle, a pop
        Sfx::Kiss => {
            let mut ph = 0.0f32;
            synth(0.45, |t, n| {
                let breath = n.next()
                    * 0.15
                    * if t < 0.2 {
                        (t / 0.2) * (1.0 - t / 0.2) * 4.0
                    } else {
                        0.0
                    };
                ph += (500.0 + 2200.0 * (t / 0.3).min(1.0)) / SRF;
                let whistle = if (0.05..0.32).contains(&t) {
                    (ph * TAU).sin() * 0.25 * ((t - 0.05) / 0.27)
                } else {
                    0.0
                };
                let pop = if t > 0.32 {
                    (TAU * 900.0 * t).sin() * (-(t - 0.32) * 60.0).exp() * 0.5
                } else {
                    0.0
                };
                (breath + whistle + pop).clamp(-1.0, 1.0)
            })
        }
        Sfx::Crossed => tune(
            &[(67.0, 0.1), (72.0, 0.1), (76.0, 0.1), (79.0, 0.25)],
            0.2,
            square,
        ),
        Sfx::Tick => tune(&[(84.0, 0.03)], 0.15, square),
        Sfx::GameOver => tune(
            &[(67.0, 0.25), (66.0, 0.25), (65.0, 0.25), (64.0, 0.6)],
            0.2,
            tri,
        ),
        Sfx::Giggle => tune(
            &[(84.0, 0.06), (88.0, 0.06), (84.0, 0.06), (91.0, 0.12)],
            0.14,
            tri,
        ),
        Sfx::Fanfare => tune(
            &[
                (72.0, 0.15),
                (72.0, 0.15),
                (72.0, 0.15),
                (76.0, 0.45),
                (74.0, 0.15),
                (76.0, 0.15),
                (79.0, 0.3),
                (84.0, 0.8),
            ],
            0.2,
            square,
        ),
        Sfx::Select => tune(&[(79.0, 0.04)], 0.16, tri),
        Sfx::Honk => {
            let mut ph = 0.0f32;
            synth(0.3, |t, _| {
                ph += 330.0 / SRF;
                (square(ph) * 0.5 + square(ph * 1.26) * 0.5)
                    * (t / 0.01).min(1.0)
                    * ((0.3 - t) / 0.05).clamp(0.0, 1.0)
                    * 0.12
            })
        }
        Sfx::Life => tune(
            &[(76.0, 0.08), (79.0, 0.08), (84.0, 0.08), (88.0, 0.2)],
            0.16,
            tri,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_render_and_stay_in_range() {
        for s in ALL_SFX {
            let v = render_sfx(s);
            assert!(!v.is_empty(), "{s:?}");
            assert!(
                v.iter().all(|x| x.is_finite() && x.abs() <= 1.0),
                "{s:?} out of range"
            );
        }
    }
}
