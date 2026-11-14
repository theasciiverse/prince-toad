//! Drawing: the lanes cell by cell, with half blocks at the ends of
//! everything that moves so traffic glides half a column at a time; the
//! backdrop of each world, the night's headlights, and the scenes.

use crate::game::{self, OPTION_ITEMS};
use crate::game::{Game, Mode, Overlay, Phase, RACE_TO};
use crate::input::ACTIONS;
use crate::levels::{self, KISSES, POND, TOAD_ART, WORLDS};
use crate::save::{BOARD_LEN, BOARD_TITLES, BOARD_TODAY, LIVES_OPTIONS, NAME_LEN, PACES};
use crate::world::{DT, Death, Dir, Ground, Kind, Lane, Level, Role, Surface, W};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

type Rgb = (u8, u8, u8);

fn c(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn scale(rgb: Rgb, k: f32) -> Rgb {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    (f(rgb.0), f(rgb.1), f(rgb.2))
}

fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).clamp(0.0, 255.0) as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

fn luma(c: Rgb) -> f32 {
    0.299 * c.0 as f32 + 0.587 * c.1 as f32 + 0.114 * c.2 as f32
}

/// A colour that stands out from what is behind it.
fn contrast(fg: Rgb, bg: Rgb) -> Rgb {
    let (lf, lb) = (luma(fg), luma(bg));
    if (lf - lb).abs() >= 80.0 {
        fg
    } else if lb > 120.0 {
        mix(fg, (25, 25, 30), 0.6)
    } else {
        mix(fg, (255, 255, 250), 0.6)
    }
}

fn hash(a: i32, b: i32) -> u32 {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ (b as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 13)
}

const PANEL_BG: Rgb = (16, 22, 20);
const PANEL_FG: Rgb = (110, 200, 110);
const TEXT: Rgb = (225, 230, 215);
const DIM: Rgb = (130, 140, 130);
const GOLD: Rgb = (240, 200, 80);
const TOAD1: Rgb = (100, 210, 70);
const TOAD2: Rgb = (240, 180, 50);
const PRINCESS: Rgb = (175, 235, 140);

/// A cell to paint: character, foreground, background (None keeps what is there).
#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    fg: Rgb,
    bg: Option<Rgb>,
}

struct Canvas<'a> {
    buf: &'a mut Buffer,
    area: Rect,
}

impl Canvas<'_> {
    fn put(&mut self, x: i32, y: i32, ch: char, fg: Rgb, bg: Option<Rgb>) {
        if x < self.area.x as i32
            || y < self.area.y as i32
            || x >= self.area.right() as i32
            || y >= self.area.bottom() as i32
        {
            return;
        }
        let cell = &mut self.buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(c(fg));
        if let Some(b) = bg {
            cell.set_bg(c(b));
        }
        cell.modifier = Modifier::empty();
    }

    /// Nothing drawn at this cell yet but its background.
    fn empty_at(&self, x: i32, y: i32) -> bool {
        let a = self.area;
        x >= a.x as i32
            && y >= a.y as i32
            && x < a.right() as i32
            && y < a.bottom() as i32
            && self.buf[(x as u16, y as u16)].symbol() == " "
    }

    fn bg_at(&self, x: i32, y: i32) -> Rgb {
        if x < self.area.x as i32
            || y < self.area.y as i32
            || x >= self.area.right() as i32
            || y >= self.area.bottom() as i32
        {
            return (0, 0, 0);
        }
        match self.buf[(x as u16, y as u16)].bg {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (0, 0, 0),
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, fg: Rgb, bg: Option<Rgb>) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as i32, y, ch, fg, bg);
        }
    }

    fn centre(&mut self, y: i32, s: &str, fg: Rgb, bg: Option<Rgb>) {
        let w = s.chars().count() as i32;
        let x = self.area.x as i32 + (self.area.width as i32 - w) / 2;
        self.text(x, y, s, fg, bg);
    }

    fn fill(&mut self, r: Rect, bg: Rgb) {
        for y in r.y..r.bottom() {
            for x in r.x..r.right() {
                self.put(x as i32, y as i32, ' ', TEXT, Some(bg));
            }
        }
    }

    /// Darken everything drawn so far.
    fn dim(&mut self, k: f32) {
        for y in self.area.y..self.area.bottom() {
            for x in self.area.x..self.area.right() {
                let cell = &mut self.buf[(x, y)];
                if let Color::Rgb(r, g, b) = cell.fg {
                    cell.fg = c(scale((r, g, b), k));
                }
                if let Color::Rgb(r, g, b) = cell.bg {
                    cell.bg = c(scale((r, g, b), k));
                }
            }
        }
    }

    /// A framed panel centred on screen; returns its inside.
    fn panel(&mut self, w: u16, h: u16, title: &str) -> Rect {
        let w = w.min(self.area.width);
        let h = h.min(self.area.height);
        let x = self.area.x + (self.area.width - w) / 2;
        let y = self.area.y + (self.area.height - h) / 2;
        self.fill(Rect::new(x, y, w, h), PANEL_BG);
        let (x0, y0, x1, y1) = (x as i32, y as i32, (x + w - 1) as i32, (y + h - 1) as i32);
        for xx in x0..=x1 {
            self.put(xx, y0, '─', PANEL_FG, None);
            self.put(xx, y1, '─', PANEL_FG, None);
        }
        for yy in y0..=y1 {
            self.put(x0, yy, '│', PANEL_FG, None);
            self.put(x1, yy, '│', PANEL_FG, None);
        }
        self.put(x0, y0, '╭', PANEL_FG, None);
        self.put(x1, y0, '╮', PANEL_FG, None);
        self.put(x0, y1, '╰', PANEL_FG, None);
        self.put(x1, y1, '╯', PANEL_FG, None);
        if !title.is_empty() {
            let t = format!(" {title} ");
            let tx = x0 + (w as i32 - t.chars().count() as i32) / 2;
            self.text(tx, y0, &t, GOLD, None);
        }
        Rect::new(x + 2, y + 1, w.saturating_sub(4), h.saturating_sub(2))
    }
}

pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

pub fn draw(f: &mut Frame, g: &Game) {
    let area = f.area();
    let mut cv = Canvas {
        buf: f.buffer_mut(),
        area,
    };
    cv.fill(area, (8, 12, 10));
    if area.width < 80 || area.height < 24 {
        cv.centre(
            area.y as i32 + area.height as i32 / 2,
            &format!(
                "Prince Toad needs 80x24 (this is {}x{})",
                area.width, area.height
            ),
            TEXT,
            None,
        );
        return;
    }
    match g.phase {
        Phase::Title => title(&mut cv, g),
        Phase::Choose => choose(&mut cv, g),
        Phase::Options => options(&mut cv, g),
        Phase::Keys => keys(&mut cv, g),
        Phase::Scores(b) => scores(&mut cv, g, b),
        Phase::EnterName => enter_name(&mut cv, g),
        Phase::Prologue(i) => page(
            &mut cv,
            g,
            "Prince Toad",
            levels::PROLOGUE[i],
            TOAD_ART,
            i + 1 == levels::PROLOGUE.len(),
        ),
        Phase::Finale(i) => finale(&mut cv, g, i),
        _ => {
            field(&mut cv, g);
            hud(&mut cv, g);
            match g.phase {
                Phase::WorldCard => world_card(&mut cv, g),
                Phase::Kiss => kiss_scene(&mut cv, g),
                Phase::GameOver => game_over(&mut cv, g),
                Phase::Won(p) => won(&mut cv, g, p),
                _ => {}
            }
        }
    }
    match g.overlay {
        Some(Overlay::Pause) => pause(&mut cv, g),
        Some(Overlay::Help) => help(&mut cv, g),
        None => {}
    }
}

// ---- the field ----

fn bank_ground(world: usize) -> Ground {
    match world {
        1 => Ground::Sand,
        3 => Ground::Snow,
        5 => Ground::Cobble,
        _ => Ground::Bank,
    }
}

fn ground_cell(ground: Ground, world: usize, col: i32, row: i32, t: f32, below_road: bool) -> Cell {
    let h = hash(col, row);
    let g = if ground == Ground::Bank {
        bank_ground(world)
    } else {
        ground
    };
    match g {
        Ground::Grass | Ground::Bank => {
            let bg = if ground == Ground::Bank {
                (64, 128, 56)
            } else {
                (56, 112, 50)
            };
            Cell {
                ch: if h.is_multiple_of(7) {
                    '"'
                } else if h.is_multiple_of(11) {
                    ','
                } else {
                    ' '
                },
                fg: (84, 150, 70),
                bg: Some(bg),
            }
        }
        Ground::Dirt => Cell {
            ch: if h.is_multiple_of(9) { '·' } else { ' ' },
            fg: (152, 116, 80),
            bg: Some((126, 94, 62)),
        },
        Ground::Road => {
            let dash = below_road && col.rem_euclid(8) < 3;
            Cell {
                ch: if dash { '▁' } else { ' ' },
                fg: (200, 190, 120),
                bg: Some((50, 50, 56)),
            }
        }
        Ground::Rail => Cell {
            ch: if col.rem_euclid(2) == 0 { '╪' } else { '═' },
            fg: (150, 145, 150),
            bg: Some((74, 58, 46)),
        },
        Ground::Sand => Cell {
            ch: if h.is_multiple_of(8) { '·' } else { ' ' },
            fg: (178, 148, 96),
            bg: Some((204, 174, 116)),
        },
        Ground::Snow => Cell {
            ch: if h.is_multiple_of(10) { '·' } else { ' ' },
            fg: (190, 198, 215),
            bg: Some((226, 231, 240)),
        },
        Ground::Ice => Cell {
            ch: if h.is_multiple_of(5) {
                '╱'
            } else if h.is_multiple_of(13) {
                '·'
            } else {
                ' '
            },
            fg: (215, 235, 250),
            bg: Some((160, 200, 228)),
        },
        Ground::Cobble => Cell {
            ch: if (col + row).rem_euclid(2) == 0 {
                '▫'
            } else {
                ' '
            },
            fg: (128, 118, 110),
            bg: Some((104, 96, 90)),
        },
        Ground::Water | Ground::Moat | Ground::Pond => {
            let (bg, fg) = match g {
                Ground::Water => ((34, 78, 148), (80, 130, 200)),
                Ground::Moat => ((44, 82, 68), (72, 112, 92)),
                _ => ((30, 92, 104), (64, 132, 138)),
            };
            let drift = (t * 1.5) as i32;
            let ch = if hash(col + drift * if row % 2 == 0 { 1 } else { -1 }, row).is_multiple_of(7)
            {
                '~'
            } else {
                ' '
            };
            Cell {
                ch,
                fg,
                bg: Some(bg),
            }
        }
    }
}

fn body_color(kind: Kind, seed: u32) -> Rgb {
    const CARS: [Rgb; 6] = [
        (205, 60, 50),
        (60, 105, 205),
        (232, 190, 60),
        (222, 222, 226),
        (60, 160, 95),
        (150, 85, 180),
    ];
    match kind {
        Kind::Car => CARS[(seed >> 3) as usize % CARS.len()],
        Kind::Truck => (196, 196, 202),
        Kind::Train => (70, 90, 120),
        Kind::Cart => (170, 72, 46),
        Kind::Tractor => (64, 150, 64),
        Kind::Moto => (40, 40, 48),
        Kind::Plow => (232, 182, 40),
        Kind::Carriage => (92, 42, 112),
        Kind::Log => (116, 78, 42),
        Kind::Floe | Kind::CrackingFloe => (228, 240, 250),
        Kind::Pad => (64, 150, 70),
        Kind::Barrel => (142, 96, 50),
        Kind::Hedge => (32, 92, 38),
        Kind::Bridge => (128, 92, 56),
        Kind::Stall => (196, 64, 54),
        _ => (200, 200, 200),
    }
}

/// Whether the item is drawn as a solid body with half-block ends.
fn solid(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Car
            | Kind::Truck
            | Kind::Train
            | Kind::Cart
            | Kind::Tractor
            | Kind::Moto
            | Kind::Plow
            | Kind::Carriage
            | Kind::Log
            | Kind::Floe
            | Kind::CrackingFloe
            | Kind::Pad
            | Kind::Barrel
            | Kind::Hedge
            | Kind::Bridge
            | Kind::Stall
    )
}

/// The detail drawn in cell k of a solid item of length len going right (or not).
fn detail(
    kind: Kind,
    k: usize,
    len: usize,
    right: bool,
    seed: u32,
    surf: Surface,
) -> Option<(char, Rgb)> {
    let front = if right { len - 1 } else { 0 };
    let fwd = |n: usize| if right { len.saturating_sub(1 + n) } else { n };
    match kind {
        Kind::Car => (len >= 3 && k > 0 && k < len - 1).then_some(('▀', (170, 210, 240))),
        Kind::Train => {
            // an engine in front, carriages with windows behind
            if k == front {
                Some((if right { '▶' } else { '◀' }, (230, 70, 50)))
            } else if k == fwd(1) || k == fwd(2) {
                Some(('▀', (250, 230, 140)))
            } else if (if right { len - 1 - k } else { k }) % 6 == 3 {
                Some(('│', (30, 40, 60)))
            } else {
                Some(('▀', (190, 210, 230)))
            }
        }
        Kind::Truck => {
            let cab = [(200, 60, 50), (60, 100, 200), (60, 150, 80), (230, 140, 40)]
                [(seed >> 4) as usize % 4];
            if k == front || k == fwd(1) {
                Some((
                    if k == fwd(1) { '▀' } else { '█' },
                    if k == fwd(1) { (170, 210, 240) } else { cab },
                ))
            } else if k == fwd(2) {
                Some((' ', (0, 0, 0)))
            } else {
                None
            }
        }
        Kind::Cart => {
            if k == 1 || k + 2 == len {
                Some(('o', (60, 40, 20)))
            } else if k > 1 && k + 2 < len {
                Some(('▀', (226, 196, 92)))
            } else {
                None
            }
        }
        Kind::Tractor => {
            if k == fwd(len - 1) || k == fwd(len - 2) {
                Some(('●', (30, 30, 30)))
            } else if k == front {
                Some(('▀', (170, 210, 240)))
            } else {
                None
            }
        }
        Kind::Plow => (k == front)
            .then_some((if right { '◣' } else { '◢' }, (120, 120, 130)))
            .or((k == fwd(1)).then_some(('▀', (170, 210, 240)))),
        Kind::Carriage => {
            if k == 1 || k + 2 == len {
                Some(('o', (230, 200, 90)))
            } else if k > 1 && k + 2 < len {
                Some(('▀', (240, 220, 120)))
            } else {
                None
            }
        }
        Kind::Log => (k % 3 == 1).then_some(('═', (86, 56, 28))),
        Kind::Barrel => (k % 2 == 1).then_some(('║', (90, 60, 30))),
        Kind::Bridge => {
            if k == 0 || k + 1 == len {
                Some(('╫', (70, 60, 50)))
            } else {
                Some(('═', (92, 64, 36)))
            }
        }
        // a striped awning over the goods
        Kind::Stall => Some(if k.is_multiple_of(2) {
            ('▀', (240, 236, 220))
        } else {
            (
                ['●', '♣', '◆', '●'][(seed as usize + k) % 4],
                [
                    (240, 180, 40),
                    (90, 170, 60),
                    (200, 90, 160),
                    (230, 110, 40),
                ][(seed as usize + k) % 4],
            )
        }),
        Kind::Floe => (k % 4 == 2).then_some(('▒', (200, 220, 238))),
        Kind::CrackingFloe => match surf {
            Surface::Sinking(_) => {
                Some((if k.is_multiple_of(2) { '╱' } else { '╲' }, (90, 110, 140)))
            }
            _ => (k % 4 == 2).then_some(('▒', (200, 220, 238))),
        },
        Kind::Pad => (seed.is_multiple_of(3) && k == len / 2)
            .then_some(('✿', (240, 160, 200)))
            .or(Some(('◡', (46, 120, 54)))),
        Kind::Hedge => Some(('♣', (60, 140, 60))),
        _ => None,
    }
}

/// Glyph items: one character per cell, on whatever is below.
fn glyph(kind: Kind, k: usize, right: bool, surf: Surface) -> Option<(char, Rgb)> {
    match kind {
        Kind::Bike => Some((
            if (k == 1) == right {
                'o'
            } else if right {
                '>'
            } else {
                '<'
            },
            (240, 120, 60),
        )),
        Kind::Goose => Some(if (k == 1) == right {
            (if right { '▸' } else { '◂' }, (250, 170, 40))
        } else {
            (if right { '◖' } else { '◗' }, (245, 245, 240))
        }),
        Kind::Sled => Some(if (k == 1) == right {
            ('o', (230, 60, 60))
        } else {
            ('▬', (150, 90, 50))
        }),
        Kind::Knight => Some(('♞', (220, 220, 235))),
        Kind::Guard => Some(('♟', (210, 60, 60))),
        Kind::Turtle | Kind::DivingTurtle => match surf {
            Surface::Under => Some(('·', (90, 140, 200))),
            Surface::Sinking(s) => Some((
                if k.is_multiple_of(2) { '◖' } else { '◗' },
                mix((120, 160, 60), (34, 78, 148), s * 0.7),
            )),
            Surface::Up => Some((if k.is_multiple_of(2) { '◖' } else { '◗' }, (120, 165, 60))),
        },
        Kind::Duck | Kind::DivingDuck => {
            let (ch, fg) = if (k % 2 == 1) == right {
                (if right { '▸' } else { '◂' }, (250, 170, 40))
            } else {
                (if right { '◖' } else { '◗' }, (245, 245, 235))
            };
            match surf {
                Surface::Under => Some(('·', (90, 130, 110))),
                Surface::Sinking(s) => Some((ch, mix(fg, (44, 82, 68), s * 0.7))),
                Surface::Up => Some((ch, fg)),
            }
        }
        Kind::Rock => Some(('●', (140, 140, 140))),
        Kind::Tree => Some(('♣', (30, 120, 40))),
        Kind::Lamp => Some(('¡', (250, 230, 150))),
        Kind::Cactus => Some(('Ψ', (60, 150, 60))),
        Kind::Flowers => Some((
            '✿',
            [(240, 160, 200), (250, 230, 110), (245, 245, 245)][k % 3],
        )),
        Kind::House => Some(('⌂', (210, 130, 90))),
        Kind::Reeds => Some(('¦', (120, 160, 70))),
        _ => None,
    }
}

/// Light at each (row, column + 10) of the field, for the night.
fn light_map(lv: &Level, t: f32) -> Vec<Vec<f32>> {
    let h = lv.height();
    let mut light = vec![vec![0.1f32; 100]; h];
    let add = |light: &mut Vec<Vec<f32>>, r: i32, c: i32, v: f32| {
        if r >= 0 && (r as usize) < h && (-10..90).contains(&c) {
            let cell = &mut light[r as usize][(c + 10) as usize];
            *cell = (*cell + v).min(1.2);
        }
    };
    for (r, lane) in lv.rows.iter().enumerate() {
        for (i, it) in lane.items.iter().enumerate() {
            match it.kind {
                Kind::Lamp => {
                    let cx = it.x0 as i32 - 10;
                    for dr in -2i32..=2 {
                        for dc in -8i32..=8 {
                            let d = ((dc as f32 / 2.0).powi(2) + (dr as f32).powi(2)).sqrt();
                            if d < 4.0 {
                                add(&mut light, r as i32 + dr, cx + dc, 0.7 * (1.0 - d / 4.0));
                            }
                        }
                    }
                }
                k if k.role() == Role::Hazard && lane.moving() => {
                    let right = lane.velocity(t) > 0.0;
                    for (a, b) in lane.spans(i, t) {
                        // the body a little, the road ahead a lot
                        for cc in a.floor() as i32..b.ceil() as i32 {
                            add(&mut light, r as i32, cc, 0.12);
                        }
                        let front = if right {
                            b.ceil() as i32
                        } else {
                            a.floor() as i32 - 1
                        };
                        for n in 0..10 {
                            let cc = if right { front + n } else { front - n };
                            let v = 0.9 * (1.0 - n as f32 / 10.0);
                            add(&mut light, r as i32, cc, v);
                            if n > 1 && n < 7 {
                                add(&mut light, r as i32 - 1, cc, v * 0.35);
                                add(&mut light, r as i32 + 1, cc, v * 0.35);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    light
}

struct Geo {
    /// Screen x of field column 0.
    fx0: i32,
    /// Screen y of row 0.
    fy0: i32,
}

fn geo(cv: &Canvas, lv: &Level) -> Geo {
    let a = cv.area;
    let fx0 = a.x as i32 + (a.width as i32 - W as i32) / 2;
    let avail = a.height as i32 - 2;
    let extra = (avail - lv.height() as i32).max(0);
    // a little ground below the start, the rest is sky
    let fy0 = a.y as i32 + 1 + extra - extra.min(2);
    Geo { fx0, fy0 }
}

fn sky(world: usize, night: bool) -> (Rgb, Rgb) {
    if night {
        return ((8, 10, 28), (20, 24, 50));
    }
    match world {
        0 => ((120, 176, 228), (190, 220, 240)),
        1 => ((150, 190, 232), (230, 210, 170)),
        2 => ((110, 170, 220), (200, 225, 235)),
        3 => ((176, 188, 205), (220, 226, 235)),
        5 => ((120, 110, 150), (230, 170, 130)),
        _ => ((90, 120, 170), (245, 190, 140)),
    }
}

/// Hills, skyline or castle on the horizon: height in half rows at a column.
fn skyline(world: usize, col: i32) -> (i32, Rgb) {
    let s = |f: f32, p: f32| ((col as f32 * f + p).sin() + 1.0) * 0.5;
    match world {
        1 => {
            let block = col.div_euclid(5);
            let hgt = 2 + (hash(block, 7) % 7) as i32;
            (hgt, (96, 106, 126))
        }
        3 => (
            (s(0.06, 1.0) * 5.0 + s(0.17, 0.3) * 2.0 + 1.0) as i32,
            (238, 242, 250),
        ),
        5 => {
            let tower = col.rem_euclid(26) < 5;
            let crenel = col.rem_euclid(2) == 0;
            (
                if tower {
                    9 + crenel as i32
                } else {
                    5 + crenel as i32
                },
                (96, 88, 100),
            )
        }
        6 => (
            (s(0.11, 0.5) * 3.0 + s(0.04, 2.0) * 2.0 + 1.0) as i32,
            (64, 108, 64),
        ),
        _ => (
            (s(0.05, 0.0) * 4.0 + s(0.13, 2.0) * 2.0 + 1.0) as i32,
            if world == 4 {
                (18, 30, 26)
            } else {
                (82, 146, 72)
            },
        ),
    }
}

/// Is there a cloud at (col, rows below the top of the sky)?
fn cloud(col: i32, row: i32, t: f32) -> Option<char> {
    for j in 0..6 {
        let speed = 0.4 + (j % 3) as f32 * 0.25;
        let cx = ((hash(j, 1) % 140) as f32 + t * speed).rem_euclid(140.0) as i32 - 20;
        let cy = 1 + (hash(j, 2) % 4) as i32;
        let w = 8 + (hash(j, 3) % 8) as i32;
        let dx = (col - cx).abs();
        if row == cy && dx <= w / 2 {
            return Some('█');
        }
        if row == cy - 1 && dx <= w / 3 {
            return Some('▄');
        }
    }
    None
}

fn field(cv: &mut Canvas, g: &Game) {
    let lv = &g.level;
    let t = g.lt as f32 * DT;
    let gm = geo(cv, lv);
    let a = cv.area;
    let (top, bottom) = (a.y as i32 + 1, a.bottom() as i32 - 1);
    let left = (a.x as i32 - gm.fx0).max(-10);
    let right = (a.right() as i32 - gm.fx0).min(90);
    let light = if g.night {
        Some(light_map(lv, t))
    } else {
        None
    };
    // sky and horizon above the field
    let (hi, lo) = sky(g.world, g.night);
    let (hi, lo) = if lv.rain && !g.night {
        (mix(hi, (120, 128, 140), 0.6), mix(lo, (150, 156, 165), 0.6))
    } else {
        (hi, lo)
    };
    let sky_rows = gm.fy0 - top;
    for y in top..gm.fy0 {
        let k = if sky_rows > 1 {
            (y - top) as f32 / (sky_rows - 1) as f32
        } else {
            1.0
        };
        let base = mix(hi, lo, k);
        for x in a.x as i32..a.right() as i32 {
            let col = x - gm.fx0;
            let (hgt, hc) = skyline(g.world, col);
            let hc = if g.night { scale(hc, 0.6) } else { hc };
            let mut cell = Cell {
                ch: ' ',
                fg: base,
                bg: Some(base),
            };
            let k2 = (gm.fy0 - y) * 2;
            if k2 <= hgt {
                cell.bg = Some(hc);
                if g.world == 1 && hash(col, y).is_multiple_of(5) {
                    cell = Cell {
                        ch: '▪',
                        fg: if g.night {
                            (230, 200, 90)
                        } else {
                            (165, 180, 200)
                        },
                        bg: cell.bg,
                    };
                }
            } else if k2 - 1 == hgt {
                cell = Cell {
                    ch: '▄',
                    fg: hc,
                    bg: Some(base),
                };
            } else if g.night && hash(col, y).is_multiple_of(23) {
                let tw = ((t * 2.0 + hash(col, y + 1) as f32 * 0.001).sin() + 1.0) * 0.5;
                cell = Cell {
                    ch: '·',
                    fg: mix((60, 60, 90), (240, 240, 255), tw),
                    bg: cell.bg,
                };
            } else if let Some(ch) = (!g.night && sky_rows > 5)
                .then(|| cloud(col, y - top, t))
                .flatten()
            {
                cell = Cell {
                    ch,
                    fg: if g.world == 3 {
                        (225, 228, 236)
                    } else {
                        (248, 250, 252)
                    },
                    bg: cell.bg,
                };
            } else if g.world == 3 && !g.night {
                // snow falling
                let fall = (t * 3.0) as i32;
                if hash(col + (y - fall) / 3, y - fall).is_multiple_of(17) {
                    cell = Cell {
                        ch: '*',
                        fg: (250, 250, 255),
                        bg: cell.bg,
                    };
                }
            }
            cv.put(x, y, cell.ch, cell.fg, cell.bg);
        }
    }
    if g.night && sky_rows >= 3 {
        cv.put(gm.fx0 + 68, top + 1, '◐', (240, 240, 210), None);
    }
    // the lanes
    for (r, lane) in lv.rows.iter().enumerate() {
        let y = gm.fy0 + r as i32;
        let below_road = lane.ground == Ground::Road
            && lv.rows.get(r + 1).is_some_and(|l| l.ground == Ground::Road);
        for x in a.x as i32..a.right() as i32 {
            let col = x - gm.fx0;
            let mut cell = ground_cell(lane.ground, g.world, col, r as i32, t, below_road);
            if !(left..right).contains(&col) {
                // beyond the lanes: darker
                cell.bg = cell.bg.map(|b| scale(b, 0.55));
                cell.fg = scale(cell.fg, 0.55);
            } else if !(0..W as i32).contains(&col) {
                cell.bg = cell.bg.map(|b| scale(b, 0.8));
                cell.fg = scale(cell.fg, 0.8);
            }
            cv.put(x, y, cell.ch, cell.fg, cell.bg);
        }
        draw_items(cv, lv, r, lane, t, gm.fx0, y, left, right);
    }
    // level-crossing lights
    for r in 0..lv.height() {
        if lv.train_coming(r, t) && (t * 4.0) as i32 % 2 == 0 {
            for rr in [r as i32 - 1, r as i32 + 1] {
                if rr >= 0 && rr < lv.height() as i32 {
                    for col in [1, 78] {
                        cv.put(gm.fx0 + col, gm.fy0 + rr, '●', (255, 50, 40), None);
                    }
                }
            }
        }
    }
    // a fly
    if let Some((row, x, left)) = g.fly
        && (left > 1.0 || (t * 6.0) as i32 % 2 == 0)
    {
        let sx = gm.fx0 + x.round() as i32;
        let wing = if (t * 12.0) as i32 % 2 == 0 {
            '°'
        } else {
            '·'
        };
        cv.put(sx, gm.fy0 + row as i32, wing, (40, 40, 45), None);
        cv.put(sx + 1, gm.fy0 + row as i32, '•', (20, 20, 25), None);
    }
    // what waits on the far bank, standing tall against the sky where there is room
    if !lv.pond && g.mode == Mode::Story {
        let kiss = &KISSES[g.kiss];
        let mx = gm.fx0 + 40 - kiss.mark.chars().count() as i32 / 2;
        cv.text(mx, gm.fy0, kiss.mark, kiss.color, None);
        let art = kiss.art;
        let aw = art.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
        let ah = art.len() as i32;
        let mut heart_y = gm.fy0 - 1;
        if sky_rows >= ah + 2 {
            let ax = gm.fx0 + 40 - aw / 2;
            let colour = if g.night {
                scale(kiss.color, 0.75)
            } else {
                kiss.color
            };
            for (i, line) in art.iter().enumerate() {
                let y = gm.fy0 - ah + i as i32;
                for (j, ch) in line.chars().enumerate() {
                    if ch != ' ' {
                        let (x, y) = (ax + j as i32, y);
                        let behind = cv.bg_at(x, y);
                        cv.put(x, y, ch, contrast(colour, behind), None);
                    }
                }
            }
            heart_y = gm.fy0 - ah - 1;
        }
        if (t * 2.0) as i32 % 2 == 0 && heart_y >= top {
            cv.put(gm.fx0 + 40, heart_y, '♥', (240, 90, 120), None);
        }
    }
    // the princess
    if let Some(pr) = &g.princess
        && let Some((px, prow)) = g.princess_pos()
    {
        let k = (pr.since / 0.3).min(1.0);
        let (fx, frow) = pr.from;
        let hx = fx + (px - fx) * k;
        let hy =
            frow as f32 + (prow as f32 - frow as f32) * k - (k * std::f32::consts::PI).sin() * 1.5;
        let sx = gm.fx0 + hx.round() as i32;
        let sy = gm.fy0 + hy.round() as i32;
        cv.text(sx, sy, "▙▟", PRINCESS, None);
        let crown_y = sy - 1;
        if crown_y >= top {
            cv.put(sx, crown_y, 'w', GOLD, None);
            cv.put(sx + 1, crown_y, '♥', (240, 120, 150), None);
        }
    }
    // the toads
    for (i, p) in g.players.iter().enumerate() {
        let colour = if i == 0 { TOAD1 } else { TOAD2 };
        let sx = gm.fx0 + p.toad.x.round() as i32;
        let sy = gm.fy0 + p.toad.row as i32;
        match p.dead {
            Some((cause, dt)) => {
                let s = match cause {
                    Death::Squashed | Death::TimeUp => {
                        if dt < 0.5 {
                            "▂▂"
                        } else {
                            "✶✶"
                        }
                    }
                    _ => ["oo", "○○", "◌◌", "··"][((dt * 6.0) as usize).min(3)],
                };
                let fg = match cause {
                    Death::Squashed | Death::TimeUp => (220, 70, 60),
                    _ => (200, 230, 250),
                };
                cv.text(sx, sy, s, fg, None);
            }
            None => {
                let s = if p.toad.air < 4 {
                    "▚▞"
                } else {
                    match p.toad.facing {
                        Dir::Down | Dir::DownLeft | Dir::DownRight => "▛▜",
                        _ => "▙▟",
                    }
                };
                let blink = p.toad.air < 3;
                cv.text(
                    sx,
                    sy,
                    s,
                    if blink {
                        mix(colour, (255, 255, 255), 0.4)
                    } else {
                        colour
                    },
                    None,
                );
            }
        }
    }
    // rain over everything, and a shine on the wet roads
    if lv.rain {
        for y in top..bottom {
            for x in a.x as i32..a.right() as i32 {
                // each drop falls one row and one column at a time, along its slant
                let fall = (t * 14.0) as i32;
                let h = hash(x + y, y - fall);
                if h.is_multiple_of(23) && cv.buf[(x as u16, y as u16)].symbol() == " " {
                    cv.put(x, y, '╱', (150, 175, 215), None);
                }
            }
        }
        for (r, lane) in lv.rows.iter().enumerate() {
            if lane.ground != Ground::Road {
                continue;
            }
            let y = gm.fy0 + r as i32;
            for x in a.x as i32..a.right() as i32 {
                let bg = cv.bg_at(x, y);
                let cell = &mut cv.buf[(x as u16, y as u16)];
                if cell.symbol() == " " && hash(x, r as i32 + 40).is_multiple_of(9) {
                    cell.set_char('~');
                    cell.fg = c(mix(bg, (170, 190, 220), 0.35));
                }
            }
        }
    }
    // fog: lanes far from every toad fade into grey
    if g.fog {
        let rows: Vec<i32> = g.players.iter().map(|p| p.toad.row as i32).collect();
        for r in 0..lv.height() as i32 {
            let d = rows.iter().map(|&tr| (tr - r).abs()).min().unwrap_or(0);
            let k = ((d as f32 - 1.5) / 2.5).clamp(0.0, 0.92);
            if k <= 0.0 {
                continue;
            }
            let y = gm.fy0 + r;
            for x in a.x as i32..a.right() as i32 {
                let cell = &mut cv.buf[(x as u16, y as u16)];
                let fog = |rgb: Rgb| mix(rgb, (168, 172, 176), k);
                if let Color::Rgb(r_, g_, b_) = cell.fg {
                    cell.fg = c(fog((r_, g_, b_)));
                }
                if let Color::Rgb(r_, g_, b_) = cell.bg {
                    cell.bg = c(fog((r_, g_, b_)));
                }
            }
        }
    }
    // night: everything outside the lights goes dark
    if let Some(light) = light {
        let mut toads: Vec<(i32, i32)> = g
            .players
            .iter()
            .map(|p| (p.toad.row as i32, p.toad.x.round() as i32 + 1))
            .collect();
        if let Some((px, prow)) = g.princess_pos() {
            toads.push((prow as i32, px.round() as i32 + 1));
        }
        for (r, row) in light.iter().enumerate() {
            let y = gm.fy0 + r as i32;
            for x in a.x as i32..a.right() as i32 {
                let col = x - gm.fx0;
                let mut l = if (-10..90).contains(&col) {
                    row[(col + 10) as usize]
                } else {
                    0.16
                };
                for &(tr, tc) in &toads {
                    let d = (((col - tc) as f32 / 2.0).powi(2) + ((r as i32 - tr) as f32).powi(2))
                        .sqrt();
                    if d < 2.5 {
                        l = l.max(0.75 - d * 0.15);
                    }
                }
                if r == 0 && (col - 40).abs() < 4 {
                    l = l.max(0.8);
                }
                let l = l.min(1.0);
                let cell = &mut cv.buf[(x as u16, y as u16)];
                let night = |rgb: Rgb| mix(scale(rgb, l), (10, 14, 40), (1.0 - l) * 0.35);
                if let Color::Rgb(r_, g_, b_) = cell.fg {
                    cell.fg = c(night((r_, g_, b_)));
                }
                if let Color::Rgb(r_, g_, b_) = cell.bg {
                    cell.bg = c(night((r_, g_, b_)));
                }
            }
        }
        // headlights themselves always shine
        for (r, lane) in lv.rows.iter().enumerate() {
            if !lane.moving() {
                continue;
            }
            let going = lane.velocity(t) > 0.0;
            for (i, it) in lane.items.iter().enumerate() {
                if it.kind.role() != Role::Hazard || matches!(it.kind, Kind::Goose | Kind::Guard) {
                    continue;
                }
                for (a0, b0) in lane.spans(i, t) {
                    let hx = if going {
                        b0.ceil() as i32 - 1
                    } else {
                        a0.floor() as i32
                    };
                    if (left..right).contains(&hx) {
                        let x = gm.fx0 + hx;
                        let bg = cv.bg_at(x, gm.fy0 + r as i32);
                        cv.put(
                            x,
                            gm.fy0 + r as i32,
                            if going { '▐' } else { '▌' },
                            (255, 245, 190),
                            Some(bg),
                        );
                    }
                }
            }
        }
    }
    // below the start: more of the same ground
    let last = lv.height() - 1;
    for y in gm.fy0 + lv.height() as i32..bottom {
        for x in a.x as i32..a.right() as i32 {
            let col = x - gm.fx0;
            let mut cell = ground_cell(lv.rows[last].ground, g.world, col, y + 31, t, false);
            let k = if g.night { 0.3 } else { 0.85 };
            cell.bg = cell.bg.map(|b| scale(b, k));
            cv.put(x, y, cell.ch, scale(cell.fg, k), cell.bg);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_items(
    cv: &mut Canvas,
    lv: &Level,
    r: usize,
    lane: &Lane,
    t: f32,
    fx0: i32,
    y: i32,
    left: i32,
    right: i32,
) {
    let going_right = lane.velocity(t) >= 0.0;
    for (i, it) in lane.items.iter().enumerate() {
        let surf = if it.kind.role() == Role::Floater {
            lv.surface(r, i, t)
        } else {
            Surface::Up
        };
        let len = it.len as usize;
        for (a, b) in lane.spans(i, t) {
            if b < left as f32 || a > right as f32 {
                continue;
            }
            if solid(it.kind) {
                let mut body = body_color(it.kind, it.seed);
                if it.kind == Kind::Pad {
                    match surf {
                        Surface::Sinking(s) => body = mix(body, (30, 92, 104), s * 0.8),
                        Surface::Under => continue,
                        Surface::Up => {}
                    }
                }
                if it.kind == Kind::Bridge && surf != Surface::Up {
                    // going up, or up: only its shadow on the water and the posts
                    let s = match surf {
                        Surface::Sinking(s) => s,
                        _ => 1.0,
                    };
                    for col in a.floor() as i32..b.ceil() as i32 {
                        if !(left..right).contains(&col) {
                            continue;
                        }
                        let x = fx0 + col;
                        let end = col == a.floor() as i32 || col + 1 == b.ceil() as i32;
                        if end {
                            cv.put(x, y, '┃', (70, 60, 50), None);
                        } else if s < 1.0 {
                            let under = cv.bg_at(x, y);
                            cv.put(x, y, '▔', mix(body, under, s), Some(under));
                        }
                    }
                    continue;
                }
                if it.kind == Kind::CrackingFloe && surf == Surface::Under {
                    for col in a.floor() as i32..b.ceil() as i32 {
                        if (left..right).contains(&col) && col % 2 == 0 {
                            cv.put(fx0 + col, y, '·', (210, 225, 240), None);
                        }
                    }
                    continue;
                }
                for col in a.floor() as i32..b.ceil() as i32 {
                    if !(left..right).contains(&col) {
                        continue;
                    }
                    let lh = a <= col as f32 + 0.25 && (col as f32 + 0.25) < b;
                    let rh = a <= col as f32 + 0.75 && (col as f32 + 0.75) < b;
                    let x = fx0 + col;
                    let under = cv.bg_at(x, y);
                    match (lh, rh) {
                        (true, true) => {
                            let k = ((col as f32 + 0.5 - a).floor().max(0.0) as usize).min(len - 1);
                            match detail(it.kind, k, len, going_right, it.seed, surf) {
                                Some((ch, fg)) => cv.put(x, y, ch, fg, Some(body)),
                                None => cv.put(x, y, ' ', body, Some(body)),
                            }
                        }
                        (true, false) => cv.put(x, y, '▌', body, Some(under)),
                        (false, true) => cv.put(x, y, '▐', body, Some(under)),
                        _ => {}
                    }
                }
            } else if it.kind == Kind::Fire {
                for col in a.round() as i32..b.round() as i32 {
                    if (left..right).contains(&col) {
                        let f = hash(col, (t * 12.0) as i32);
                        let ch = ['▓', '▒', '░'][f as usize % 3];
                        let fg =
                            [(250, 90, 30), (250, 180, 40), (220, 40, 30)][(f >> 3) as usize % 3];
                        cv.put(fx0 + col, y, ch, fg, None);
                    }
                }
            } else {
                for k in 0..len {
                    let col = a.round() as i32 + k as i32;
                    if (left..right).contains(&col)
                        && let Some((ch, fg)) = glyph(it.kind, k, going_right, surf)
                    {
                        cv.put(fx0 + col, y, ch, fg, None);
                    }
                }
            }
        }
    }
}

// ---- the bars ----

fn hud(cv: &mut Canvas, g: &Game) {
    let a = cv.area;
    let y = a.y as i32;
    let bg = (18, 26, 22);
    cv.fill(Rect::new(a.x, a.y, a.width, 1), bg);
    let x0 = a.x as i32 + 1;
    match g.mode {
        Mode::Together => {
            let p1 = &g.players[0];
            let s = format!(" ▙▟×{} {:06}", lives_text(g, p1.lives), p1.score);
            cv.text(x0, y, &s, TOAD1, None);
            cv.centre(
                y,
                &format!("Road {}   both toads across", g.level_no + 1),
                DIM,
                None,
            );
            if let Some(p2) = g.players.get(1) {
                let s2 = format!("{:06} ▙▟×{} ", p2.score, lives_text(g, p2.lives));
                cv.text(
                    a.right() as i32 - s2.chars().count() as i32 - 1,
                    y,
                    &s2,
                    TOAD2,
                    None,
                );
            }
        }
        Mode::Versus => {
            let p1 = g.players[0].crossings;
            let p2 = g.players.get(1).map_or(0, |p| p.crossings);
            let s = format!(" ▙▟ Player one {p1}   ");
            cv.text(x0, y, &s, TOAD1, None);
            let mid = format!(
                "Road {}   first to {RACE_TO} crossings wins",
                g.level_no + 1
            );
            cv.centre(y, &mid, DIM, None);
            let s2 = format!("Player two {p2} ▙▟ ");
            cv.text(
                a.right() as i32 - s2.chars().count() as i32 - 1,
                y,
                &s2,
                TOAD2,
                None,
            );
        }
        _ => {
            let p = g.lead();
            let place = match g.mode {
                Mode::Story if g.level_no == POND => g.name.clone(),
                Mode::Story => format!("{}-{} {}", g.world + 1, g.level_no + 1, g.name),
                _ => format!("{}  {}", g.name, WORLDS[g.world].name),
            };
            cv.text(x0, y, "PRINCE TOAD", (130, 220, 110), None);
            cv.text(x0 + 13, y, &place, TEXT, None);
            let lives = format!("▙▟×{}", lives_text(g, p.lives));
            let score = format!("SCORE {:06}", p.score);
            let mut x = a.right() as i32 - 1;
            if g.time > 0.0 {
                let bar = 16;
                let fillw = ((p.time_left / g.time).clamp(0.0, 1.0) * bar as f32).ceil() as i32;
                x -= bar + 6;
                cv.text(x, y, "TIME", DIM, None);
                let warn = p.time_left <= 10.0;
                for i in 0..bar {
                    let on = i < fillw;
                    let col = if warn { (230, 80, 60) } else { (110, 200, 90) };
                    cv.put(
                        x + 5 + i,
                        y,
                        if on { '█' } else { '░' },
                        if on { col } else { (60, 70, 60) },
                        None,
                    );
                }
            }
            x -= score.chars().count() as i32 + 3;
            cv.text(x, y, &score, GOLD, None);
            if g.mode != Mode::Versus {
                x -= lives.chars().count() as i32 + 3;
                cv.text(x, y, &lives, TOAD1, None);
            }
        }
    }
    // the caption
    let yb = a.bottom() as i32 - 1;
    cv.fill(Rect::new(a.x, a.bottom() - 1, a.width, 1), bg);
    let line = match &g.msg {
        Some((m, _)) => m.clone(),
        None => match g.mode {
            _ if g.demo => "Demo: any key to stop".to_string(),
            Mode::Story if g.level.pond => {
                "Find the princess. Hop diagonally with Q E Z C or numpad 7 9 1 3.".to_string()
            }
            Mode::Story => format!("Waiting on the far side: {}", KISSES[g.kiss].name),
            Mode::Endless if g.daily => format!(
                "Today's road. Best today: {}",
                if g.save.daily_day == crate::game::today() {
                    g.save.daily_best
                } else {
                    0
                }
            ),
            Mode::Endless => format!("Best on the endless road: {}", g.save.best_endless),
            Mode::Versus => "Player one: W A S D    Player two: arrows".to_string(),
            Mode::Together => {
                "Both toads must get across. Player one: W A S D, player two: arrows".to_string()
            }
        },
    };
    cv.text(
        a.x as i32 + 2,
        yb,
        &line,
        if g.msg.is_some() { GOLD } else { DIM },
        None,
    );
    let keys = "H help  Esc pause";
    cv.text(
        a.right() as i32 - keys.len() as i32 - 2,
        yb,
        keys,
        (90, 110, 95),
        None,
    );
}

// ---- scenes ----

const BIG: [(char, [&str; 5]); 10] = [
    ('P', ["████ ", "█   █", "████ ", "█    ", "█    "]),
    ('R', ["████ ", "█   █", "████ ", "█  █ ", "█   █"]),
    ('I', ["███", " █ ", " █ ", " █ ", "███"]),
    ('N', ["█   █", "██  █", "█ █ █", "█  ██", "█   █"]),
    ('C', [" ████", "█    ", "█    ", "█    ", " ████"]),
    ('E', ["█████", "█    ", "████ ", "█    ", "█████"]),
    ('T', ["█████", "  █  ", "  █  ", "  █  ", "  █  "]),
    ('O', [" ███ ", "█   █", "█   █", "█   █", " ███ "]),
    ('A', [" ███ ", "█   █", "█████", "█   █", "█   █"]),
    ('D', ["████ ", "█   █", "█   █", "█   █", "████ "]),
];

fn big_text(cv: &mut Canvas, y: i32, s: &str) {
    let glyphs: Vec<Option<&[&str; 5]>> = s
        .chars()
        .map(|ch| BIG.iter().find(|(c, _)| *c == ch).map(|(_, g)| g))
        .collect();
    let width: i32 = glyphs
        .iter()
        .map(|g| g.map_or(3, |g| g[0].chars().count() as i32 + 1))
        .sum();
    let mut x = cv.area.x as i32 + (cv.area.width as i32 - width) / 2;
    for g in glyphs {
        match g {
            Some(rows) => {
                for (dy, row) in rows.iter().enumerate() {
                    let colour = mix((150, 235, 110), (50, 140, 60), dy as f32 / 4.0);
                    for (dx, ch) in row.chars().enumerate() {
                        if ch != ' ' {
                            cv.put(x + dx as i32, y + dy as i32, ch, colour, None);
                        }
                    }
                }
                x += rows[0].chars().count() as i32 + 1;
            }
            None => x += 3,
        }
    }
}

fn title(cv: &mut Canvas, g: &Game) {
    let a = cv.area;
    let t = g.phase_t;
    cv.fill(a, (10, 22, 16));
    let top = a.y as i32 + (a.height as i32 - 24) / 2;
    big_text(cv, top + 1, "PRINCE TOAD");
    cv.centre(
        top + 7,
        "a toad, a lot of roads, and one kiss that counts",
        (170, 190, 160),
        None,
    );
    let items = g.title_items();
    let mw = items.iter().map(|s| s.chars().count()).max().unwrap_or(0) as i32 + 4;
    let mx = a.x as i32 + (a.width as i32 - mw) / 2;
    for (i, it) in items.iter().enumerate() {
        let sel = i == g.menu.min(items.len() - 1);
        let s = if sel {
            format!("▙▟  {it}")
        } else {
            format!("    {it}")
        };
        cv.text(
            mx,
            top + 9 + i as i32,
            &s,
            if sel { TOAD1 } else { DIM },
            None,
        );
    }
    let best = format!(
        "Best story score {}    Best on the endless road {}",
        g.save.best_story, g.save.best_endless
    );
    let best_y = top + 10 + items.len() as i32;
    cv.centre(best_y, &best, (110, 130, 110), None);
    // a little road at the bottom, with traffic, where there is room
    let y = a.bottom() as i32 - 4;
    if y - 2 <= best_y {
        draw_title_footer(cv);
        return;
    }
    let lanes = [
        Lane::new(
            Ground::Road,
            6.0,
            0.0,
            "cccc..........tttttttt.............cccc......",
        ),
        Lane::new(
            Ground::Road,
            -4.5,
            0.0,
            "....cccc.........................cccc...........",
        ),
    ];
    let lv = Level::new(lanes.to_vec(), false);
    let fx0 = a.x as i32 + (a.width as i32 - 80) / 2;
    for (r, lane) in lv.rows.iter().enumerate() {
        for x in a.x as i32..a.right() as i32 {
            cv.put(
                x,
                y + r as i32,
                if r == 0 && (x - fx0).rem_euclid(8) < 3 {
                    '▁'
                } else {
                    ' '
                },
                (200, 190, 120),
                Some((50, 50, 56)),
            );
        }
        let left = (a.x as i32 - fx0).max(-10);
        let right = (a.right() as i32 - fx0).min(90);
        draw_items(cv, &lv, r, lane, t, fx0, y + r as i32, left, right);
    }
    for x in a.x as i32..a.right() as i32 {
        cv.put(
            x,
            y - 1,
            if hash(x, 3).is_multiple_of(6) {
                '"'
            } else {
                ' '
            },
            (84, 150, 70),
            Some((56, 112, 50)),
        );
        cv.put(
            x,
            y + 2,
            if hash(x, 5).is_multiple_of(6) {
                '"'
            } else {
                ' '
            },
            (84, 150, 70),
            Some((56, 112, 50)),
        );
    }
    // a toad waiting to cross
    let hop = (t * 0.8) as i32 % 6 == 0;
    cv.text(fx0 + 38, y + if hop { 1 } else { 2 }, "▙▟", TOAD1, None);
    draw_title_footer(cv);
}

fn draw_title_footer(cv: &mut Canvas) {
    let a = cv.area;
    let v = game_version();
    cv.text(
        a.right() as i32 - v.len() as i32 - 2,
        a.bottom() as i32 - 1,
        &v,
        (80, 100, 85),
        None,
    );
    cv.text(
        a.x as i32 + 2,
        a.bottom() as i32 - 1,
        "W S or arrows to choose, Enter to go",
        (80, 100, 85),
        None,
    );
}

fn game_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

fn choose(cv: &mut Canvas, g: &Game) {
    let all = levels::story();
    let n = g.save.reached.min(POND) + 1;
    let inner = cv.panel(56, (n as u16 + 4).min(cv.area.height), "Choose a crossing");
    let first = g.choose.saturating_sub(inner.height as usize - 2);
    for (row, i) in (first..n).enumerate().take(inner.height as usize - 1) {
        let s = &all[i];
        let sel = i == g.choose;
        let line = format!(
            "{} {:>2}  {:<22} {}",
            if sel { "▙▟" } else { "  " },
            i + 1,
            s.name,
            WORLDS[s.world].name
        );
        cv.text(
            inner.x as i32,
            inner.y as i32 + row as i32,
            &line,
            if sel { TOAD1 } else { DIM },
            None,
        );
    }
    cv.text(
        inner.x as i32,
        inner.bottom() as i32 - 1,
        "Enter to play, Esc to go back",
        (90, 110, 95),
        None,
    );
}

fn page(cv: &mut Canvas, g: &Game, title: &str, text: &str, art: &[&str], last: bool) {
    let inner = cv.panel(66, 18, title);
    let mut y = inner.y as i32 + 1;
    let aw = art.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
    let ax = inner.x as i32 + (inner.width as i32 - aw) / 2;
    for line in art {
        for (i, ch) in line.chars().enumerate() {
            let fg = match ch {
                'w' => GOLD,
                '♥' => (240, 100, 130),
                _ => TOAD1,
            };
            cv.put(ax + i as i32, y, ch, fg, None);
        }
        y += 1;
    }
    y += 1;
    let shown = ((g.phase_t * 45.0) as usize).min(text.len());
    let partial: String = text.chars().take(shown).collect();
    for line in wrap(&partial, inner.width as usize - 4) {
        cv.centre(y, &line, TEXT, None);
        y += 1;
    }
    if g.phase_t > 1.0 {
        cv.text(
            inner.right() as i32 - 18,
            inner.bottom() as i32 - 1,
            if last {
                "Enter: onwards"
            } else {
                "Enter: go on"
            },
            DIM,
            None,
        );
    }
}

/// When the text of finale page i starts.
pub fn finale_text_at(i: usize) -> f32 {
    if i == 0 { KISS_TEXT } else { 0.6 }
}

/// The ending, in three pages: the kiss (and its fizzle), half a crown each,
/// and happily ever after.
fn finale(cv: &mut Canvas, g: &Game, i: usize) {
    let t = g.phase_t;
    let inner = cv.panel(66, 18, "Happily ever after");
    let mid = inner.x as i32 + inner.width as i32 / 2;
    let ground = inner.y as i32 + 7;
    let tw = 11;
    // the two toads meet in the middle (page 0), then stand together
    let k = if i == 0 { (t / 1.2).min(1.0) } else { 1.0 };
    let hx = (inner.x as i32 + 1) + ((mid - tw - (inner.x as i32 + 1)) as f32 * k) as i32;
    let sx = (inner.right() as i32 - tw - 1)
        - ((inner.right() as i32 - tw - 1 - mid - 1) as f32 * k) as i32;
    let bob = |x: f32| {
        if k < 1.0 {
            ((x * 6.0).sin().abs() * 1.5) as i32
        } else {
            0
        }
    };
    for (n, line) in TOAD_ART.iter().enumerate() {
        let y = ground - TOAD_ART.len() as i32 + 1 + n as i32;
        for (j, ch) in line.chars().enumerate() {
            if ch != ' ' {
                cv.put(hx + j as i32, y - bob(t), ch, TOAD1, None);
                cv.put(sx + j as i32, y - bob(t + 0.3), ch, PRINCESS, None);
            }
        }
    }
    for x in inner.x as i32..inner.right() as i32 {
        cv.put(x, ground + 1, '▔', (90, 130, 80), None);
    }
    // her crown, and later half of it on him
    let crown_her = (sx + 5, ground - 4 - bob(t + 0.3));
    let crown_him = (hx + 5, ground - 4);
    match i {
        0 => cv.put(crown_her.0, crown_her.1, 'w', GOLD, None),
        1 => {
            let s = ((t - 1.2) / 0.8).clamp(0.0, 1.0);
            cv.put(
                crown_her.0,
                crown_her.1,
                if s > 0.0 { 'v' } else { 'w' },
                GOLD,
                None,
            );
            if s > 0.0 {
                let x = crown_her.0 + ((crown_him.0 - crown_her.0) as f32 * s) as i32;
                let y = crown_him.1 - ((s * std::f32::consts::PI).sin() * 2.0) as i32;
                cv.put(x, y, 'v', GOLD, None);
            }
        }
        _ => {
            cv.put(crown_her.0, crown_her.1, 'v', GOLD, None);
            cv.put(crown_him.0, crown_him.1, 'v', GOLD, None);
        }
    }
    // the kiss: smack, hope, fizzle, and it does not matter at all
    if i == 0 && t > 1.2 {
        if t < 2.2 {
            cv.text(mid - 3, ground - 6, "smack!", GOLD, None);
        }
        if (KISS_HOPE..KISS_FIZZLE + 0.8).contains(&t) {
            let rising = t < KISS_FIZZLE;
            let s = if rising {
                (t - KISS_HOPE) / (KISS_FIZZLE - KISS_HOPE)
            } else {
                (t - KISS_FIZZLE) / 0.8
            };
            for j in 0..14 {
                let a =
                    j as f32 / 14.0 * std::f32::consts::TAU + t * if rising { 3.0 } else { 0.5 };
                let r = if rising { 16.0 - 3.0 * s } else { 13.0 };
                let x = mid + (a.cos() * r) as i32;
                let mut y = ground - 2 + (a.sin() * r * 0.3) as i32;
                if !rising {
                    y += (s * 3.0) as i32;
                    if hash(j, 4) as f32 / u32::MAX as f32 > 1.0 - s {
                        continue;
                    }
                }
                let (ch, fg) = if rising {
                    (
                        ['✦', '*', '·'][(j as usize + (t * 10.0) as usize) % 3],
                        GOLD,
                    )
                } else {
                    ('·', (120, 120, 110))
                };
                if y <= ground && y > inner.y as i32 && cv.empty_at(x, y) {
                    cv.put(x, y, ch, fg, None);
                }
            }
        }
    }
    // hearts, more and more of them
    let hearts = match i {
        0 if t > 2.6 => 3,
        0 => 0,
        1 => 5,
        _ => 12,
    };
    for j in 0..hearts {
        let rise = ((t * 1.5 + hash(j, 11) as f32 / u32::MAX as f32 * 6.0) % 6.0) as i32;
        let x = mid - 8 + (hash(j, 12) % 17) as i32;
        let y = ground - 4 - rise;
        if y > inner.y as i32 {
            cv.put(x, y, '♥', (240, 100, 130), None);
        }
    }
    let text = levels::FINALE[i];
    let start = finale_text_at(i);
    let mut y = ground + 3;
    if t > start {
        let shown = (((t - start) * KISS_CPS) as usize).min(text.len());
        let partial: String = text.chars().take(shown).collect();
        for line in wrap(&partial, inner.width as usize - 6) {
            cv.centre(y, &line, TEXT, None);
            y += 1;
        }
    }
    if i + 1 == levels::FINALE.len() && t > start + text.len() as f32 / KISS_CPS + 0.6 {
        cv.centre(y + 1, "Kisses: 19   Princes: 1", GOLD, None);
    }
    if t > start + 0.4 {
        let s = if i + 1 == levels::FINALE.len() {
            "Enter: the end"
        } else {
            "Enter: go on"
        };
        cv.text(
            inner.right() as i32 - s.len() as i32 - 1,
            inner.bottom() as i32 - 1,
            s,
            DIM,
            None,
        );
    }
}

fn world_card(cv: &mut Canvas, g: &Game) {
    let w = &WORLDS[g.world];
    let pond = g.level.pond;
    let inner = cv.panel(
        60,
        if pond { 10 } else { 9 },
        &format!("World {}", g.world + 1),
    );
    // the way so far: a heart for every kiss, the toad here, the crown at the end
    if g.mode == Mode::Story {
        let n = POND + 1;
        let w = n as i32 * 2 - 1;
        let x0 = inner.x as i32 + (inner.width as i32 - w) / 2;
        let yy = inner.bottom() as i32 - 2;
        for i in 0..n {
            let (ch, fg) = if i == g.level_no {
                ('●', TOAD1)
            } else if i == POND {
                ('w', GOLD)
            } else if i < g.level_no {
                ('♥', (230, 100, 130))
            } else {
                ('·', DIM)
            };
            cv.put(x0 + i as i32 * 2, yy, ch, fg, None);
        }
    }
    let y = inner.y as i32;
    cv.centre(y + 1, w.name, GOLD, None);
    cv.centre(y + 2, w.tagline, TEXT, None);
    if pond {
        cv.centre(
            y + 4,
            "New: hop diagonally! Q E Z C, numpad 7 9 1 3,",
            TOAD1,
            None,
        );
        cv.centre(
            y + 5,
            "or two directions at once on the d-pad.",
            TOAD1,
            None,
        );
    } else {
        cv.centre(
            y + 4,
            &format!("Crossing {}: {}", g.level_no + 1, g.name),
            DIM,
            None,
        );
    }
    if g.phase_t > 0.4 {
        cv.centre(
            inner.bottom() as i32 - 1,
            "Enter or any hop to start",
            (90, 110, 95),
            None,
        );
    }
}

fn kiss_scene(cv: &mut Canvas, g: &Game) {
    cv.dim(0.35);
    let kiss = &KISSES[g.kiss];
    let t = g.phase_t;
    let inner = cv.panel(
        70,
        19,
        &format!(
            "{} kisses {}",
            if g.level_no == 0 { "The prince" } else { "He" },
            kiss.name
        ),
    );
    let base = inner.y as i32 + 1;
    // the object on the right
    let ow = kiss
        .art
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0) as i32;
    let ox = inner.x as i32 + inner.width as i32 - ow - 6;
    let oh = kiss.art.len() as i32;
    let ground = base + 7;
    // each kiss has its own little after-effect (by kiss number)
    let after = (t - 1.3).max(0.0);
    let kissed = t > 1.3;
    let shake = match g.kiss {
        // the cactus and the duchess shake the scene
        4 | 16 if kissed && after < 0.8 => {
            if (t * 30.0) as i32 % 2 == 0 {
                1
            } else {
                -1
            }
        }
        _ => 0,
    };
    let art_colour = match g.kiss {
        // the lamppost flickers
        12 if kissed && after < 1.0 && (t * 14.0) as i32 % 3 == 0 => (60, 60, 50),
        _ => kiss.color,
    };
    let moon_splashed = g.kiss == 13 && kissed;
    for (i, line) in kiss.art.iter().enumerate() {
        let y = ground - oh + 1 + i as i32;
        if moon_splashed && i < 3 {
            // the moon was only a puddle: ripples where it was
            let ripple = ["  ~ ~  ~", " ~  ~ ~ ~", "  ~  ~"][i];
            cv.text(ox + shake, y, ripple, (120, 150, 200), None);
            continue;
        }
        cv.text(ox + shake, y, line, art_colour, None);
    }
    // the toad hops over from the left
    let tw = 11;
    let start = inner.x as i32 + 2;
    let end = ox - tw;
    let k = (t / 1.2).min(1.0);
    let mut tx = start + ((end - start) as f32 * k) as i32;
    let mut hop = if k < 1.0 {
        ((t * 6.0).sin().abs() * 1.5) as i32
    } else {
        0
    };
    let mut colour = TOAD1;
    match g.kiss {
        // the cow licks him clean: he shines
        2 if kissed => colour = mix(TOAD1, (230, 250, 255), (after * 2.0).min(0.6)),
        // the rubber duck squeaks and he jumps
        7 if kissed && after < 0.6 => {
            hop = (3.0 * (after / 0.6 * std::f32::consts::PI).sin()) as i32
        }
        // the heron, the dragon: he leaves, quickly
        8 | 17 if after > 2.2 => tx -= ((after - 2.2) * 30.0) as i32,
        // stuck to the icicle: he shivers
        10 if kissed => tx += if (t * 20.0) as i32 % 2 == 0 { 1 } else { 0 },
        _ => {}
    }
    for (i, line) in TOAD_ART.iter().enumerate() {
        let y = ground - TOAD_ART.len() as i32 + 1 + i as i32 - hop;
        if tx + 10 > inner.x as i32 + 1 {
            for (j, ch) in line.chars().enumerate() {
                let x = tx + j as i32 + shake;
                if x > inner.x as i32 && ch != ' ' {
                    cv.put(x, y, ch, colour, None);
                }
            }
        }
    }
    for x in inner.x as i32..inner.right() as i32 {
        cv.put(x, ground + 1, '▔', (90, 130, 80), None);
    }
    // extras
    match g.kiss {
        // a letter falls out of the mailbox
        0 if kissed => {
            let fy = (ground - 3 + (after * 4.0) as i32).min(ground);
            cv.put(ox + 3, fy, '✉', (240, 240, 230), None);
        }
        // the cactus: ow
        4 if kissed => {
            for (j, (dx, dy)) in [(-1, -4), (9, -5), (-2, -1), (10, -2)].iter().enumerate() {
                if after > j as f32 * 0.2 {
                    cv.text(end + dx + shake, ground + dy, "ow", (250, 120, 90), None);
                }
            }
        }
        // the duchess screams
        16 if kissed && after < 2.5 => {
            let s = "A A A A H !";
            let n = ((after * 12.0) as usize).min(s.len());
            cv.text(ox - 4 + shake, ground - 7, &s[..n], (250, 110, 200), None);
        }
        // the dragon breathes smoke in its sleep
        17 if kissed => {
            for j in 0..4 {
                let rise = ((after * 3.0) as i32 + j) % 5;
                cv.put(
                    ox + 16 + j % 2,
                    ground - 3 - rise,
                    ['o', 'O', '°', '~'][j as usize],
                    (170, 170, 170),
                    None,
                );
            }
        }
        // the fish splashes back down
        6 if kissed && after < 1.5 => cv.text(ox, ground + 1, "~~ ~~ ~~", (120, 170, 230), None),
        _ => {}
    }
    let end = end + shake;
    if t > 1.2 && !matches!(g.kiss, 4 | 16) {
        // smack
        let n = ((t - 1.2) * 8.0) as i32;
        for j in 0..n.min(6) {
            let hx = end + tw / 2 + (hash(j, 9) % 9) as i32 - 2;
            let hy = ground - 4 - j / 2 - ((t - 1.2) * 2.0) as i32;
            if hy > inner.y as i32 {
                cv.put(hx, hy, '♥', (240, 90, 120), None);
            }
        }
    }
    if t > 1.2 && t < 2.2 {
        cv.text(end + 2, ground - 5, "smack!", GOLD, None);
    }
    // a moment of hope: sparkles gather round him... and fizzle out
    let toad_mid = (tx + tw / 2 + shake, ground - 2);
    if (KISS_HOPE..KISS_FIZZLE + 0.8).contains(&t) && g.kiss != 4 {
        let rising = t < KISS_FIZZLE;
        let k = if rising {
            (t - KISS_HOPE) / (KISS_FIZZLE - KISS_HOPE)
        } else {
            (t - KISS_FIZZLE) / 0.8
        };
        for j in 0..10 {
            let a = j as f32 / 10.0 * std::f32::consts::TAU + t * if rising { 3.0 } else { 0.5 };
            let r = if rising { 10.0 - 3.0 * k } else { 7.0 };
            let x = toad_mid.0 + (a.cos() * r) as i32;
            let mut y = toad_mid.1 + (a.sin() * r * 0.45) as i32;
            if !rising {
                // they drop, grey, and go out one by one
                y += (k * 3.0) as i32;
                if hash(j, 4) as f32 / u32::MAX as f32 > 1.0 - k {
                    continue;
                }
            }
            let (ch, fg) = if rising {
                (
                    ['✦', '*', '·'][(j as usize + (t * 10.0) as usize) % 3],
                    GOLD,
                )
            } else {
                ('·', (120, 120, 110))
            };
            if y < ground + 1 && y > inner.y as i32 && cv.empty_at(x, y) {
                cv.put(x, y, ch, fg, None);
            }
        }
        if !rising && k < 0.8 {
            cv.text(
                toad_mid.0 + 4,
                toad_mid.1 - 3,
                "fzzt",
                (150, 150, 140),
                None,
            );
        }
    }
    let done = kiss_text_done(kiss.text);
    if t > KISS_TEXT {
        let shown = (((t - KISS_TEXT) * KISS_CPS) as usize).min(kiss.text.len());
        let partial: String = kiss.text.chars().take(shown).collect();
        let mut y = ground + 3;
        for line in wrap(&partial, inner.width as usize - 6) {
            cv.centre(y, &line, TEXT, None);
            y += 1;
        }
        if t > done + 0.5
            && let Some(v) = levels::VERDICTS.get(g.kiss)
        {
            cv.centre(y + 1, v, GOLD, None);
        }
    }
    // the tally so far
    if g.mode == Mode::Story {
        let tally = format!("Kisses: {}   Princes: 0", g.level_no + 1);
        cv.text(
            inner.x as i32,
            inner.y as i32,
            &tally,
            (110, 130, 110),
            None,
        );
    }
    if t > KISS_ENTER && !g.demo {
        let s = if g.level_no + 1 == POND {
            "Enter: on towards the castle garden"
        } else {
            "Enter: on to the next road"
        };
        cv.text(
            inner.right() as i32 - s.len() as i32 - 1,
            inner.bottom() as i32 - 1,
            s,
            DIM,
            None,
        );
    }
}

fn lives_text(g: &Game, n: u32) -> String {
    if g.endless_lives() && n > 0 {
        "∞".to_string()
    } else {
        n.to_string()
    }
}

/// A menu line: the toad marks the chosen one.
fn menu_line(cv: &mut Canvas, x: i32, y: i32, sel: bool, s: &str) {
    let line = if sel {
        format!("▙▟ {s}")
    } else {
        format!("   {s}")
    };
    cv.text(x, y, &line, if sel { TOAD1 } else { DIM }, None);
}

fn options(cv: &mut Canvas, g: &Game) {
    let inner = cv.panel(64, 16, "Options");
    let x = inner.x as i32 + 1;
    let y = inner.y as i32 + 1;
    let pace = PACES[g.save.pace];
    let rows: [(String, &str); OPTION_ITEMS] = [
        (
            format!("Pace          < {} >", pace.0),
            match g.save.pace {
                0 => "Everything at full speed.",
                1 => "Everything at three quarters speed, the clock too.",
                _ => "Everything at half speed, the clock too.",
            },
        ),
        (
            format!("Lives         < {} >", LIVES_OPTIONS[g.save.lives]),
            match g.save.lives {
                0 => "Five lives, and one more every 5000 points.",
                1 => "Nine lives, like a cat. A toad can dream.",
                _ => "Squashed toads just start the road again.",
            },
        ),
        (
            format!(
                "Sound         < {} >",
                if g.muted {
                    "off"
                } else if g.quiet {
                    "quiet"
                } else {
                    "on"
                }
            ),
            "Also M or F8, any time.",
        ),
        ("Keys".to_string(), "Choose your own keys for the hops."),
        ("Done".to_string(), ""),
    ];
    for (i, (label, note)) in rows.iter().enumerate() {
        let sel = i == g.opt_sel;
        menu_line(cv, x, y + i as i32 * 2, sel, label);
        if sel && !note.is_empty() {
            cv.text(x + 3, y + i as i32 * 2 + 1, note, (150, 170, 150), None);
        }
    }
    cv.text(
        x,
        inner.bottom() as i32 - 1,
        "Up and down to choose, left and right to change",
        (90, 110, 95),
        None,
    );
    if g.save.pace > 0 || g.save.lives > 0 {
        cv.text(
            x,
            inner.bottom() as i32 - 2,
            "Scores made this way are marked in the tables.",
            (90, 110, 95),
            None,
        );
    }
}

fn keys(cv: &mut Canvas, g: &Game) {
    let n = ACTIONS.len();
    let inner = cv.panel(60, n as u16 + 9, "Keys");
    let x = inner.x as i32 + 1;
    let y = inner.y as i32 + 1;
    for (i, a) in ACTIONS.iter().enumerate() {
        let sel = i == g.keys_sel;
        let key = if sel && g.key_wait {
            "press a key...".to_string()
        } else {
            g.key_for(i).to_uppercase()
        };
        menu_line(cv, x, y + i as i32, sel, &format!("{:<22}{}", a.1, key));
    }
    menu_line(
        cv,
        x,
        y + n as i32 + 1,
        g.keys_sel == n,
        "Put the usual keys back",
    );
    menu_line(cv, x, y + n as i32 + 2, g.keys_sel == n + 1, "Done");
    let hint = if g.key_wait {
        "Press the new key, or Esc to keep the old one"
    } else {
        "Enter to choose a key. The arrows always work too."
    };
    cv.text(x, inner.bottom() as i32 - 1, hint, (90, 110, 95), None);
    if let Some((m, _)) = &g.msg {
        cv.text(x, inner.bottom() as i32 - 2, m, GOLD, None);
    }
}

fn scores(cv: &mut Canvas, g: &Game, b: usize) {
    let inner = cv.panel(80, BOARD_LEN as u16 + 6, "High scores");
    let x = inner.x as i32 + 1;
    let y = inner.y as i32;
    // the three tables as tabs
    let mut tx = x;
    for (i, t) in BOARD_TITLES.iter().enumerate() {
        let s = format!(" {t} ");
        let (fg, bg) = if i == b {
            ((20, 30, 20), Some(TOAD1))
        } else {
            (DIM, None)
        };
        cv.text(tx, y, &s, fg, bg);
        tx += s.chars().count() as i32 + 2;
    }
    let list = &g.save.boards[b];
    let shown_today = b != BOARD_TODAY || g.save.daily_day == game::today();
    if list.is_empty() || !shown_today {
        cv.centre(y + 4, "Nobody yet. The road is waiting.", TEXT, None);
    } else {
        for (i, e) in list.iter().enumerate() {
            let lit = g.highlight == Some((b, i));
            let colour = if lit {
                GOLD
            } else if i == 0 {
                TOAD1
            } else {
                TEXT
            };
            let line = format!(
                "{} {}. {:<10} {:>7}  {}",
                if lit { "▙▟" } else { "  " },
                i + 1,
                e.name,
                e.score,
                e.reached
            );
            let yy = y + 2 + i as i32;
            cv.text(x, yy, &line, colour, None);
            if !e.tag.is_empty() {
                let tag = format!("({})", e.tag);
                cv.text(
                    inner.right() as i32 - tag.chars().count() as i32 - 1,
                    yy,
                    &tag,
                    (120, 140, 120),
                    None,
                );
            }
        }
    }
    cv.text(
        x,
        inner.bottom() as i32 - 1,
        "Left and right for the other tables, Enter to go on",
        (90, 110, 95),
        None,
    );
}

fn enter_name(cv: &mut Canvas, g: &Game) {
    let inner = cv.panel(56, 10, "A high score!");
    let y = inner.y as i32;
    let score = g.players.first().map_or(0, |p| p.score);
    cv.centre(
        y + 1,
        &format!("{score} points. That goes in the table."),
        GOLD,
        None,
    );
    cv.centre(y + 3, "Your name:", TEXT, None);
    let blink = (g.phase_t * 2.0) as i32 % 2 == 0;
    let field = format!("{}{}", g.name_buf, if blink { "▁" } else { " " });
    let fx = inner.x as i32 + (inner.width as i32 - NAME_LEN as i32) / 2;
    for i in 0..NAME_LEN as i32 + 1 {
        cv.put(fx + i, y + 4, ' ', TEXT, Some((30, 44, 36)));
    }
    cv.text(fx, y + 4, &field, TOAD1, Some((30, 44, 36)));
    cv.centre(y + 6, "Type, then Enter", (90, 110, 95), None);
}

/// Kiss scene timing, in seconds: the hope, the fizzle, the story.
pub const KISS_HOPE: f32 = 1.35;
pub const KISS_FIZZLE: f32 = 2.0;
pub const KISS_TEXT: f32 = 2.3;
/// Enter goes on from here.
pub const KISS_ENTER: f32 = 2.4;
const KISS_CPS: f32 = 45.0;

/// When the story of a kiss has finished typing.
pub fn kiss_text_done(text: &str) -> f32 {
    KISS_TEXT + text.chars().count() as f32 / KISS_CPS
}

fn game_over(cv: &mut Canvas, g: &Game) {
    cv.dim(0.4);
    let inner = cv.panel(56, 9, "Game over");
    let y = inner.y as i32;
    cv.centre(
        y + 1,
        "The toad sits down by the road for a while.",
        TEXT,
        None,
    );
    cv.centre(y + 3, &format!("Score {}", g.lead().score), GOLD, None);
    let again = if g.mode == Mode::Story {
        "Enter: try this crossing again"
    } else {
        "Enter: a new endless road"
    };
    cv.centre(y + 5, again, TOAD1, None);
    cv.centre(y + 6, "Esc: back to the title", DIM, None);
}

fn won(cv: &mut Canvas, g: &Game, p: usize) {
    cv.dim(0.4);
    let inner = cv.panel(56, 9, "The race is won");
    let y = inner.y as i32;
    let who = if p == 0 { "Player one" } else { "Player two" };
    cv.centre(
        y + 1,
        &format!("{who} crosses {RACE_TO} roads first"),
        if p == 0 { TOAD1 } else { TOAD2 },
        None,
    );
    cv.centre(
        y + 2,
        "and gets to kiss whatever is there. Lucky toad.",
        TEXT,
        None,
    );
    cv.centre(
        y + 4,
        &format!(
            "{} to {}",
            g.players[0].crossings,
            g.players.get(1).map_or(0, |q| q.crossings)
        ),
        GOLD,
        None,
    );
    cv.centre(y + 6, "Enter: race again    Esc: title", DIM, None);
}

fn pause(cv: &mut Canvas, g: &Game) {
    cv.dim(0.5);
    let inner = cv.panel(36, 9, "Paused");
    let pace = format!("Pace: {}", PACES[g.save.pace].0);
    let items = [
        "Go on",
        "Controls",
        if g.muted { "Sound: off" } else { "Sound: on" },
        pace.as_str(),
        "Back to the title",
    ];
    for (i, it) in items.iter().enumerate() {
        let sel = i == g.pause_sel;
        let s = if sel {
            format!("▙▟ {it}")
        } else {
            format!("   {it}")
        };
        cv.text(
            inner.x as i32 + 4,
            inner.y as i32 + 1 + i as i32,
            &s,
            if sel { TOAD1 } else { DIM },
            None,
        );
    }
}

fn help(cv: &mut Canvas, g: &Game) {
    cv.dim(0.5);
    let inner = cv.panel(64, 21, "Controls");
    let mine: Vec<String> = (0..4).map(|i| g.key_for(i).to_uppercase()).collect();
    let hop = format!("{}, or the arrows", mine.join(" "));
    let lines = [
        ("Hop", hop.as_str()),
        (
            "Hop diagonally",
            "Q E Z C or numpad 7 9 1 3 (the pond only)",
        ),
        ("Pause", "Esc or P"),
        ("Sound", "M or F8"),
        ("This help", "H or F1"),
        ("Two players", "one: W A S D, two: arrows"),
        ("Own keys", "Options on the title, then Keys"),
        ("Gamepad", "d-pad or stick hops, A on, Start pause"),
        ("", ""),
        ("Roads", "do not touch anything that moves"),
        ("Water", "stay on logs, floes and barrels"),
        ("Turtles", "and ducks dive now and then"),
        ("Ice", "a hop onto ice slides one more"),
        ("Lily pads", "sink a moment after you land"),
    ];
    for (i, (k, v)) in lines.iter().enumerate() {
        let y = inner.y as i32 + 1 + i as i32;
        cv.text(inner.x as i32 + 1, y, k, GOLD, None);
        cv.text(inner.x as i32 + 18, y, v, TEXT, None);
    }
    let pads = if g.pad_names.is_empty() {
        "No gamepad found".to_string()
    } else {
        format!("Gamepad: {}", g.pad_names.join(", "))
    };
    cv.text(
        inner.x as i32 + 1,
        inner.bottom() as i32 - 2,
        &pads,
        DIM,
        None,
    );
    cv.text(
        inner.x as i32 + 1,
        inner.bottom() as i32 - 1,
        "Any key to close",
        (90, 110, 95),
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Start};
    use crate::save::Save;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn screen(g: &Game, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, g)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn every_level_draws_at_both_sizes() {
        for n in 0..=POND {
            let mut g = Game::new(Start::Story(n), Save::default());
            g.phase = Phase::Play;
            for (w, h) in [(100, 30), (80, 24)] {
                let s = screen(&g, w, h);
                assert!(s.contains("▙▟"), "level {n} at {w}x{h} shows no toad");
                assert!(s.contains("PRINCE TOAD"));
            }
        }
        let mut g = Game::new(Start::Endless, Save::default());
        g.fog = true;
        assert!(
            screen(&g, 100, 30).contains("▙▟"),
            "the toad shows through the fog"
        );
        let g = Game::new(Start::Title, Save::default());
        assert!(screen(&g, 100, 30).contains("A new story"));
        assert!(screen(&g, 60, 20).contains("needs 80x24"));
    }

    #[test]
    fn every_kiss_scene_fits_its_panel() {
        for k in 0..levels::VERDICTS.len() {
            let mut g = Game::new(Start::Story(k), Save::default());
            g.phase = Phase::Kiss;
            g.phase_t = kiss_text_done(KISSES[k].text) + 1.0;
            for (w, h) in [(100, 30), (80, 24)] {
                let s = screen(&g, w, h);
                assert!(s.contains(levels::VERDICTS[k]), "kiss {k} at {w}x{h}:\n{s}");
                assert!(s.contains("Enter: on"), "kiss {k} at {w}x{h}:\n{s}");
                let last = KISSES[k].text.split(' ').next_back().unwrap();
                assert!(s.contains(last), "kiss {k} text cut at {w}x{h}");
            }
        }
    }

    #[test]
    fn the_finale_ends_with_a_prince() {
        let mut g = Game::new(Start::Story(POND), Save::default());
        for i in 0..levels::FINALE.len() {
            g.phase = Phase::Finale(i);
            g.phase_t = 20.0;
            for (w, h) in [(100, 30), (80, 24)] {
                let s = screen(&g, w, h);
                let last = levels::FINALE[i].split(' ').next_back().unwrap();
                assert!(s.contains(last), "finale {i} at {w}x{h}:\n{s}");
                assert_eq!(
                    s.contains("Princes: 1"),
                    i + 1 == levels::FINALE.len(),
                    "finale {i}"
                );
            }
        }
    }

    #[test]
    fn wrapping() {
        assert_eq!(
            wrap("one two three four", 9),
            vec!["one two", "three", "four"]
        );
    }
}
