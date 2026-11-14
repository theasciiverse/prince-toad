//! The story: seven worlds, eighteen crossings, eighteen kisses and a pond.
//!
//! A level is written top to bottom, the far bank first and the start last.
//! Each row is a ground, a speed in columns a second (+ to the right) and a
//! pattern where each run of a letter is one thing (see `world::Kind::from_char`;
//! `.` is empty). Patterns shorter than 100 repeat.

use crate::world::{Ground, Lane, Level};

pub struct World {
    pub name: &'static str,
    pub tagline: &'static str,
}

pub const WORLDS: [World; 7] = [
    World {
        name: "The Village Lanes",
        tagline: "Carts, bicycles and one very serious goose.",
    },
    World {
        name: "The Highway",
        tagline: "Everything here is in a hurry.",
    },
    World {
        name: "The River",
        tagline: "Logs float. Turtles mostly float.",
    },
    World {
        name: "The Winter Road",
        tagline: "Ice is slippery. So is hope.",
    },
    World {
        name: "The Night Road",
        tagline: "Watch for the headlights.",
    },
    World {
        name: "The Castle",
        tagline: "Surely a castle has a princess in it.",
    },
    World {
        name: "The Quiet Pond",
        tagline: "No traffic. Only lily pads, and someone waiting.",
    },
];

pub struct Kiss {
    /// "a mailbox"
    pub name: &'static str,
    /// What stands on the far bank in the game.
    pub mark: &'static str,
    pub color: (u8, u8, u8),
    pub art: &'static [&'static str],
    pub text: &'static str,
}

pub struct Spec {
    pub name: &'static str,
    pub world: usize,
    /// Seconds for one try.
    pub time: f32,
    pub kiss: usize,
    pub night: bool,
    pub rows: Vec<(Ground, f32, f32, &'static str)>,
}

impl Spec {
    pub fn build(&self) -> Level {
        let pond = self.rows.iter().any(|r| r.0 == Ground::Pond);
        Level::new(
            self.rows
                .iter()
                .map(|&(g, s, w, p)| Lane::new(g, s, w, p))
                .collect(),
            pond,
        )
    }
}

fn r(g: &str, speed: f32, p: &'static str) -> (Ground, f32, f32, &'static str) {
    (Ground::parse(g), speed, 0.0, p)
}

/// A lane whose traffic skids back and forth as it goes.
fn skid(g: &str, speed: f32, wobble: f32, p: &'static str) -> (Ground, f32, f32, &'static str) {
    (Ground::parse(g), speed, wobble, p)
}

pub const POND: usize = 18;

#[rustfmt::skip]
pub fn story() -> Vec<Spec> {
    vec![
        // ---- the village ----
        Spec {
            name: "Mill Lane",
            world: 0,
            time: 30.0,
            kiss: 0,
            night: false,
            rows: vec![
                r("bank", 0.0, "...T.......y..........T........y......T.....y......."),
                r("grass", 0.0, "..y..........T...........y............y.....T......"),
                r("dirt", 2.5, "kkkkk..................kkkkk....................."),
                r("dirt", -3.5, "bb...........bb..............bb.................."),
                r("grass", 0.0, "........y.......................y.........y......."),
                r("dirt", 3.0, "..kkkkk.......................kkkkk..............."),
                r("grass", 0.0, "..........y...............................y........"),
            ],
        },
        Spec {
            name: "Market Road",
            world: 0,
            time: 30.0,
            kiss: 1,
            night: false,
            rows: vec![
                r("bank", 0.0, "....H.......y.........T.........y.......H......y..."),
                r("dirt", -3.0, "kkkkk.............kkkkk................kkkkk......"),
                r("dirt", 5.0, "bb.........bb..................bb..........bb....."),
                r("grass", 0.0, "##########.....######......#####......#########..."),
                r("dirt", -2.0, "gg..gg.................gg..gg..gg................"),
                r("dirt", 4.0, "rrrr.....................rrrr....................."),
                r("grass", 0.0, "....y..............y.....................y........"),
            ],
        },
        Spec {
            name: "Pasture Gate",
            world: 0,
            time: 32.0,
            kiss: 2,
            night: false,
            rows: vec![
                r("bank", 0.0, "..T......y.....T..........y..........T.....y......"),
                r("dirt", 3.0, "kkkkk.............kkkkk...........kkkkk..........."),
                r("dirt", -5.0, "bb.........bb...........bb..........bb............"),
                r("dirt", 6.5, "bb.................bb..................bb........"),
                r("grass", 0.0, "######.....########.......#######.....##########."),
                r("dirt", -3.0, "rrrr............rrrr..................rrrr........"),
                r("dirt", 2.0, "gg..gg..gg...............gg..gg..................."),
                r("dirt", -4.0, "kkkkk..............kkkkk.........................."),
                r("grass", 0.0, "....y.................y...................y......."),
            ],
        },
        // ---- the highway ----
        Spec {
            name: "The Ring Road",
            world: 1,
            time: 32.0,
            kiss: 3,
            night: false,
            rows: vec![
                r("bank", 0.0, "....^..........^..................^.........^....."),
                r("road", -5.0, "cccc...........cccc.................cccc.........."),
                r("road", 6.0, "..cccc.................cccc.......cccc............"),
                r("road", -7.5, "mm.........................mm....................."),
                r("grass", 0.0, "..y......................y......................y."),
                r("road", 4.5, "tttttttt.................tttttttt................."),
                r("road", -6.0, "cccc.........cccc......................cccc......."),
                r("road", 5.5, "....cccc..............cccc..............cccc......"),
                r("sand", 0.0, "..........^.........................^............."),
            ],
        },
        Spec {
            name: "Desert Stretch",
            world: 1,
            time: 36.0,
            kiss: 4,
            night: false,
            rows: vec![
                r("bank", 0.0, ".^........^.........o......^..........o.....^....."),
                r("sand", 0.0, "....^...........o..........^..............^......."),
                r("rail", -24.0, "RRRRRRRRRRRRRRRRRRRRRRRR............................................................................"),
                r("sand", 0.0, "..........o..................^..........o........."),
                r("road", 9.0, "cccc........................cccc.................."),
                r("road", -7.5, "tttttttt....................tttttttt................"),
                r("road", 10.5, "mm...............................mm..............."),
                r("sand", 0.0, "..o.........^.............o.........^.........o..."),
                r("road", -6.0, "cccc.......cccc...........cccc...................."),
                r("road", 8.0, "tttttttt..........................cccc............"),
                r("road", -9.5, "cccc.............................................."),
                r("sand", 0.0, "........^..............................^.........."),
            ],
        },
        Spec {
            name: "Rush Hour",
            world: 1,
            time: 36.0,
            kiss: 5,
            night: false,
            rows: vec![
                r("bank", 0.0, "....H.......T.........H.........T.......H......T..."),
                r("road", 8.0, "cccc.........cccc...........cccc.................."),
                r("road", -6.0, "tttttttt.............cccc.............cccc........"),
                r("road", 10.0, "mm.............cccc.........................mm...."),
                r("road", -8.0, "cccc...cccc......................cccc............."),
                r("grass", 0.0, "....y.................y.....................y....."),
                r("road", 7.0, "tttttttttt.................cccc...................."),
                r("road", -12.0, "cccc.....................................cccc....."),
                r("road", 9.0, "cccc......cccc...................cccc.............."),
                r("road", -5.0, "tttttttt..........tttttttt........................"),
                r("grass", 0.0, "........y.........................y..............."),
            ],
        },
        // ---- the river ----
        Spec {
            name: "Willow Ford",
            world: 2,
            time: 32.0,
            kiss: 6,
            night: false,
            rows: vec![
                r("bank", 0.0, "..T....v.......T.......vv.......T.....v......T....."),
                r("water", 2.0, "llllllllll..........llllllllll.........llllllll...."),
                r("water", -2.5, "uuuuuu........uuuuuu...........uuuuuu.............."),
                r("water", 3.0, "llllllll...........lllllllllll..........llllll......"),
                r("grass", 0.0, "....y.........................y..................."),
                r("dirt", -5.0, "kkkkk...............kkkkk........................."),
                r("dirt", 6.0, "bb..............bb..................bb............."),
                r("grass", 0.0, "..........y.........................y.............."),
            ],
        },
        Spec {
            name: "Reed Bend",
            world: 2,
            time: 34.0,
            kiss: 7,
            night: false,
            rows: vec![
                r("bank", 0.0, "v.v...T......vvv......T.........vv.....T....v.v..."),
                r("water", -3.0, "UUUUUU.......uuuuuu.........UUUUUU.........uuuuuu..."),
                r("water", 2.5, "llllllll..........lllllllllllll............llll......."),
                r("water", -4.0, "uuuu.......UUUUUU...........uuuu........UUUUUU...."),
                r("water", 3.5, "lllllllllll...............llllll..............lllll."),
                r("grass", 0.0, "...y..........................y..............y...."),
                r("road", 7.0, "cccc...............cccc............................"),
                r("road", -6.0, "tttttttt......................tttttttt.............."),
                r("road", 9.0, "mm.............................mm.................."),
                r("grass", 0.0, "..............y....................y..............."),
            ],
        },
        Spec {
            name: "Heron Rapids",
            world: 2,
            time: 36.0,
            kiss: 8,
            night: false,
            rows: vec![
                r("bank", 0.0, "..vv....T.......v.v.......T.....vv......T...v....."),
                r("water", 4.5, "llllllll.............lllllll..............llllll...."),
                r("water", -3.0, "UUUU........UUUUUU..........UUUU..........uuuuuu...."),
                r("water", 5.5, "llllll..............llllllllll...........lllll......"),
                r("water", -4.0, "UUUUUU..........uuuu..........UUUUUU..............."),
                r("water", 3.0, "lllllllll.........llll..............llllllll......."),
                r("grass", 0.0, "....y......#####...............#####.......y......"),
                r("road", -8.0, "cccc..............cccc...................cccc....."),
                r("road", 10.0, "tttttttt.........................mm..............."),
                r("road", -7.0, "cccc.........cccc................................."),
                r("grass", 0.0, "..............y....................y..............."),
            ],
        },
        // ---- winter ----
        Spec {
            name: "First Snow",
            world: 3,
            time: 32.0,
            kiss: 9,
            night: false,
            rows: vec![
                r("bank", 0.0, "..T.........T..........T.........T.........T......"),
                r("snow", 0.0, "....o.................T.................o........."),
                r("snow", -4.0, "zz..............zz................zz.............."),
                r("ice", 0.0, ""),
                r("snow", 5.0, "ssssssssss...............................ssssssssss"),
                r("snow", 0.0, "..T...........................T...................."),
                skid("snow", -3.0, 2.0, "cccc...........cccc...................cccc........"),
                r("ice", 0.0, ""),
                r("snow", 0.0, "........o.........................o..............."),
            ],
        },
        Spec {
            name: "The Frozen Well",
            world: 3,
            time: 34.0,
            kiss: 10,
            night: false,
            rows: vec![
                r("bank", 0.0, "....T.........o.........T.........o.........T....."),
                r("water", 2.0, "iiiiiiii..........IIIIII..........iiiiiiii.........."),
                r("water", -2.5, "IIIIII..........iiiiiiii..........IIIIII............"),
                r("water", 3.0, "iiiiiiiiii...............iiiiii...................."),
                r("snow", 0.0, "....T...................o.................T......."),
                r("ice", 0.0, ""),
                skid("snow", 5.0, 2.5, "cccc..............cccc............................"),
                r("snow", -6.0, "zz.................zz.................zz..........."),
                r("ice", 0.0, ""),
                r("snow", 0.0, "........o................................o........"),
            ],
        },
        Spec {
            name: "Mitten Pass",
            world: 3,
            time: 38.0,
            kiss: 11,
            night: false,
            rows: vec![
                r("bank", 0.0, "..T....T.........T....T.........T....T.......T...."),
                r("snow", -7.0, "zz........zz.......................zz............."),
                r("ice", 0.0, ""),
                skid("snow", 4.0, 3.0, "ssssssssss..........................cccc.........."),
                r("snow", 0.0, "#####.....o....#######........o.....#######......."),
                r("water", -3.0, "IIIIIII.........iiiiii..........IIIIIII............"),
                r("water", 2.5, "iiiiiiiiii..............iiiiiiii..................."),
                r("snow", 0.0, "......T...........................T..............."),
                skid("snow", -5.0, 2.5, "cccc...............cccc..............cccc........."),
                r("ice", 0.0, ""),
                r("snow", 6.0, "zz................zz..........zz..................."),
                r("snow", 0.0, "...o......................................o......."),
            ],
        },
        // ---- night ----
        Spec {
            name: "Lamplight Lane",
            world: 4,
            time: 32.0,
            kiss: 12,
            night: true,
            rows: vec![
                r("bank", 0.0, "....T.........*.........T.........*.........T....."),
                r("road", -6.0, "cccc...............cccc.................cccc......"),
                r("road", 7.0, "cccc......................cccc....................."),
                r("grass", 0.0, "..*..................*....................*......."),
                r("road", -8.0, "tttttttt..........................cccc............"),
                r("road", 5.0, "cccc..........cccc................................."),
                r("grass", 0.0, "..........*.........................*............."),
            ],
        },
        Spec {
            name: "Moon Puddle",
            world: 4,
            time: 34.0,
            kiss: 13,
            night: true,
            rows: vec![
                r("bank", 0.0, "..vv.....*.........vvv.........*........vv........"),
                r("water", 2.5, "llllllllll............llllllll..........lllllll...."),
                r("water", -3.0, "UUUUUU..........uuuuuu............UUUUUU............"),
                r("water", 3.5, "llllllll.................lllllll.................."),
                r("grass", 0.0, "....*......................*......................"),
                r("road", -9.0, "cccc......................cccc...................."),
                r("road", 6.0, "tttttttt.............cccc.........................."),
                r("road", -7.0, "cccc...........cccc..............................."),
                r("grass", 0.0, "...........*.........................*............"),
            ],
        },
        Spec {
            name: "Crow Field",
            world: 4,
            time: 38.0,
            kiss: 14,
            night: true,
            rows: vec![
                r("bank", 0.0, "..y..T......y.....T..*...y.....T......y....T...y.."),
                r("dirt", 4.0, "rrrr.....................rrrr......................"),
                r("road", -10.0, "cccc................................cccc.........."),
                r("road", 8.0, "cccc............cccc..............................."),
                r("grass", 0.0, "....*....#####............#####.........*........."),
                r("road", -6.0, "tttttttt...................cccc...................."),
                r("road", 11.0, "mm..................................mm............."),
                r("road", -8.0, "cccc.............cccc............................."),
                r("grass", 0.0, "........*..........................*.............."),
                r("dirt", 3.0, "kkkkk...................kkkkk......................"),
                r("dirt", -5.0, "bb..............bb.................bb.............."),
                r("grass", 0.0, "....y...........................y................."),
            ],
        },
        // ---- the castle ----
        Spec {
            name: "The Courtyard",
            world: 5,
            time: 32.0,
            kiss: 15,
            night: false,
            rows: vec![
                r("bank", 0.0, "#####...#####...#####...#####...#####...#####......"),
                r("cobble", -3.0, "w.....w.....w..............w.....w.....w..........."),
                r("cobble", 8.0, "h..............h.........................h........"),
                r("cobble", -5.0, "aaaaaa................aaaaaa........................"),
                r("grass", 0.0, "....y.........y..........y..........y.........y..."),
                r("cobble", 6.0, "aaaaaa............h...........aaaaaa........h......"),
                r("cobble", -9.0, "h...........h.............................h........"),
                r("cobble", 0.0, ".........o......................o................."),
            ],
        },
        Spec {
            name: "The Moat",
            world: 5,
            time: 36.0,
            kiss: 16,
            night: false,
            rows: vec![
                r("bank", 0.0, "y...y...y...y...y...y...y...y...y...y...y...y...y."),
                r("moat", 2.0, "xxx.....xxx...........xxx.......xxx..............."),
                r("moat", -2.5, "DDDD..........dddd............DDDD................."),
                r("moat", 3.0, "xxx......xxx..........xxx..........xxx............."),
                r("moat", -2.0, "dddd.........DDDD..........dddd..........DDDD......."),
                r("cobble", 0.0, "....o..........................o.................."),
                r("cobble", 7.0, "h..........h.....................h................."),
                r("cobble", -4.0, "aaaaaa..................aaaaaa......................"),
                r("cobble", -2.5, "w.....w.....w.....................w.....w.........."),
                r("cobble", 0.0, "..........o.......................o..............."),
            ],
        },
        Spec {
            name: "The Dragon's Doorstep",
            world: 5,
            time: 40.0,
            kiss: 17,
            night: false,
            rows: vec![
                r("bank", 0.0, "....o.............o..............o.............o.."),
                r("cobble", 14.0, "fff..........................................."),
                r("cobble", -6.0, "aaaaaa.................h..........aaaaaa.........."),
                r("moat", 3.0, "xxx......xxxx...........xxx...........xxxx........."),
                r("moat", -3.5, "DDDD..........DDDD...........dddd..........DDDD...."),
                r("moat", 2.5, "xxxx..........xxx.............xxxx......xxx........"),
                r("cobble", 0.0, "#####.........o.......#####.........o.......#####."),
                r("cobble", -16.0, "ffff..........................................."),
                r("cobble", 8.0, "h.........h.......................h..............."),
                r("cobble", -5.0, "aaaaaa.............w.....w...............aaaaaa...."),
                r("cobble", 0.0, "........o..........................o............."),
            ],
        },
        // ---- the pond ----
        Spec {
            name: "The Quiet Pond",
            world: 6,
            time: 0.0,
            kiss: 18,
            night: false,
            rows: vec![
                r("bank", 0.0, "v.vv..v...vvv..v.v...v..vv.v...vvv...v.v..vv...v.v"),
                r("pond", 0.0, "....ppp......pp.......ppp.......pp.......ppp......."),
                r("pond", 0.6, "..pp.......ppp......pp.......pp.......ppp.........."),
                r("pond", -0.5, ".......pp.......ppp.......pp.......ppp.......pp...."),
                r("pond", 0.0, "...ppp.......pp.......ppp........pp........ppp....."),
                r("pond", 0.8, ".pp........pp.......ppp..........pp.......pp......."),
                r("pond", -0.7, "......ppp.......pp.........pp.......ppp.......pp..."),
                r("pond", 0.0, "..pp.........ppp.........pp..........ppp......pp..."),
                r("pond", 0.5, "........pp........pp........ppp........pp.........."),
                r("pond", -0.4, "...ppp.........pp........pp.........ppp......pp...."),
                r("bank", 0.0, "v...v.....v.v.......v.........v.v.......v....v...v."),
            ],
        },
    ]
}

#[rustfmt::skip]
pub const KISSES: [Kiss; 19] = [
    Kiss {
        name: "a mailbox",
        mark: "▆▖",
        color: (200, 60, 50),
        art: &[
            r" .------.",
            r" | MAIL |]",
            r" '------'|",
            r"    ||",
            r"    ||",
            r"  __||__",
        ],
        text: "He kisses the mailbox. It is cold and tastes of stamps. A letter falls out. It is for somebody else.",
    },
    Kiss {
        name: "a garden gnome",
        mark: "▲",
        color: (220, 50, 60),
        art: &[r"    /\", r"   /  \", r"  ( .. )", r"  /|##|\", r"   d  b"],
        text: "He kisses the garden gnome. The gnome keeps smiling. It has smiled for thirty years and sees no reason to stop now.",
    },
    Kiss {
        name: "a cow",
        mark: "▛▜",
        color: (235, 235, 225),
        art: &[
            r"   _____",
            r" <(o  o)>______",
            r"   ( ~~ )      )~",
            r"    `--'||---||",
            r"         ''   ''",
        ],
        text: "He kisses the cow. She looks at him for a long time. Then she licks him from head to toe. Still a toad, but very clean.",
    },
    Kiss {
        name: "a cardboard princess",
        mark: "▐█▌",
        color: (240, 150, 190),
        art: &[
            r"   _/\/\_",
            r"   (o  o)",
            r"    \__/",
            r"  .--||--.",
            r"  |TRY OUR|",
            r"  |PANCAKE|",
            r"  '-------'",
        ],
        text: "A princess! She smiles. She is flat. She is cardboard, and she says TRY OUR PANCAKES. He kisses her anyway. She falls over.",
    },
    Kiss {
        name: "a cactus",
        mark: "Ψ",
        color: (90, 170, 80),
        art: &[
            r"     _",
            r"    | |  _",
            r"  _ | | | |",
            r" | || |_| |",
            r"  \_|  __/",
            r"    | |",
        ],
        text: "He kisses the cactus. Ow. Ow. Ow. Ow. Ow.",
    },
    Kiss {
        name: "a traffic cone",
        mark: "▲",
        color: (250, 140, 40),
        art: &[
            r"     /\",
            r"    /==\",
            r"   /    \",
            r"  /======\",
            r" /________\",
        ],
        text: "He kisses a traffic cone in a lovely orange dress. It does not say a word. A very good listener, though.",
    },
    Kiss {
        name: "a fish",
        mark: "◀◆",
        color: (120, 190, 230),
        art: &[r"      __", r"  ><_/  \_", r"  > (  o  >", r"     \__/"],
        text: "He kisses a fish who came up to see what the fuss was about. The fish is shocked. The fish goes down and tells all the other fish.",
    },
    Kiss {
        name: "a rubber duck",
        mark: "◖▸",
        color: (250, 220, 60),
        art: &[r"    __", r"   (o >", r"  _/ (_", r" (____/"],
        text: "He kisses a rubber duck stuck in the reeds. It squeaks. He jumps a whole metre into the air.",
    },
    Kiss {
        name: "a heron",
        mark: "┃",
        color: (180, 190, 200),
        art: &[
            r"      __",
            r"    <(o )",
            r"       \\",
            r"       ||",
            r"      /||",
            r"       ||",
            r"       /\",
        ],
        text: "He kisses a leg. It goes up and up and up. At the top is a heron, and the heron looks hungry. He leaves. Quickly.",
    },
    Kiss {
        name: "a snowman",
        mark: "8",
        color: (245, 245, 255),
        art: &[
            r"    _|=|_",
            r"    (o o)",
            r"   -( > )-",
            r"   (  :  )",
            r"  (   :   )",
        ],
        text: "He kisses the snowman. Its carrot nose falls off. Its coal smile looks a little less sure of itself.",
    },
    Kiss {
        name: "an icicle on the well",
        mark: "▼",
        color: (180, 220, 250),
        art: &[
            r" ____________",
            r" \/\/\/\/\/\/",
            r"  | |    | |",
            r"  | |____| |",
            r"  |________|",
        ],
        text: "He kisses an icicle hanging from the well. His lips stick. He spends a long, thoughtful hour there until the sun comes round.",
    },
    Kiss {
        name: "a lost mitten",
        mark: "▟",
        color: (220, 50, 50),
        art: &[
            r"    _",
            r"   / )__",
            r"  |  |  |",
            r"  |     |",
            r"   \___/",
            r"   [###]",
        ],
        text: "He kisses a lost red mitten. It is warm inside. For one moment he thinks about staying.",
    },
    Kiss {
        name: "a lamppost",
        mark: "¡",
        color: (250, 230, 150),
        art: &[
            r"   .-.", r"  ( o )", r"   '|'", r"    |", r"    |", r"  __|__",
        ],
        text: "He kisses the lamppost. It flickers. For one wonderful second he thinks it worked. It was only the bulb.",
    },
    Kiss {
        name: "the moon",
        mark: "◐",
        color: (240, 240, 200),
        art: &[
            r"     .--.",
            r"    (  (",
            r"     '--'",
            r" ~~~~~~~~~~~~",
            r"  ~~~~~~~~~~",
        ],
        text: "He kisses the moon. The moon turns out to be a puddle with the moon in it. Splash.",
    },
    Kiss {
        name: "a scarecrow",
        mark: "†",
        color: (200, 170, 90),
        art: &[
            r"  v  _^_  v",
            r"   \(o o)/",
            r"  --|>-<|--",
            r"    | |",
            r"    | |",
        ],
        text: "He kisses the scarecrow. The two crows on its arms laugh at him all the way to the castle.",
    },
    Kiss {
        name: "a princess statue",
        mark: "♀",
        color: (200, 200, 205),
        art: &[
            r"     www",
            r"    (- -)",
            r"    /| |\",
            r"     | |",
            r"    _|_|_",
            r"   [_____]",
        ],
        text: "At last, a princess, so still and so lovely. He kisses her foot. She is made of stone. The pigeons got here first.",
    },
    Kiss {
        name: "a duchess",
        mark: "♛",
        color: (190, 90, 200),
        art: &[
            r"     ___",
            r"    (o O)  !!",
            r"    /|=|\",
            r"   / |=| \",
            r"    /___\",
        ],
        text: "He kisses the hand of a duchess out for her evening walk. She SCREAMS. Then she screams again, louder, in case anybody missed it.",
    },
    Kiss {
        name: "a sleeping dragon",
        mark: "◣▆",
        color: (90, 160, 90),
        art: &[
            r"          __",
            r"    _____/ o\__",
            r"   /  zZ     __>",
            r"  / /\______/",
            r" /_/  \_\  \_\",
        ],
        text: "He kisses the sleeping dragon on the nose. It smiles in its sleep and mumbles five more minutes. Smoke curls out. He does not wait.",
    },
    Kiss {
        name: "the princess",
        mark: "▙▟",
        color: (150, 220, 120),
        art: &[
            r"     w",
            r"  (o) (o)",
            r" (  ___  )",
            r"  \_   _/",
            r"  /_\ /_\",
        ],
        text: "He kisses her. Nothing happens. He is still a toad. She is still a toad. It is perfect.",
    },
];

/// The prince as a toad, for the scenes.
/// The last word on each kiss, after its story: one per story kiss.
pub const VERDICTS: [&str; 18] = [
    "Still a toad.",
    "Still a toad. The gnome is still a gnome.",
    "He sparkles. It is only cow spit.",
    "Still a toad. Now he wants pancakes.",
    "Still a toad. A slightly prickly one.",
    "Still a toad. The cone will keep his secret.",
    "Still a toad. Every fish in the river knows.",
    "Still a toad. Squeak.",
    "Still a toad. Still in one piece, which is something.",
    "Still a toad. A colder one.",
    "Still a toad. Thawed, at least.",
    "Still a toad. Warm feet, though.",
    "Still a toad. Lit up, for a second.",
    "Still a toad. A wet one.",
    "Still a toad. The crows are still laughing.",
    "Still a toad. The pigeons are not impressed.",
    "Still a toad. Her ears will ring until Tuesday.",
    "Still a toad. Not toast, though!",
];

pub const TOAD_ART: &[&str] = &[r"  (o) (o)", r" (  ___  )", r"  \_   _/", r"  /_\ /_\"];

pub const PROLOGUE: [&str; 3] = [
    "Once upon a time there was a prince who said the wrong thing to the wrong old woman at the wrong well. POOF.",
    "Now he is a toad. Only a kiss can turn him back. Everybody knows that.",
    "So he sets off to find someone to kiss. They are always on the other side of the road.",
];

pub const FINALE: [&str; 3] = [
    "He kisses her. Nothing happens. He is still a toad. She is still a toad.",
    "It is perfect. He becomes her prince, and she gives him half of her tiny crown, which is not easy, but they manage.",
    "And they live happily ever after, as toads. That is the happy ending.",
];

/// What the princess says as she hops away.
pub const GIGGLES: [&str; 4] = [
    "She giggles and hops away.",
    "Not so fast, she says, and hops off.",
    "She hops away again. She is enjoying this.",
    "She stops, and waits for him.",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Ground, Role, START_X};

    #[test]
    fn every_level_is_sound() {
        let all = story();
        assert_eq!(all.len(), POND + 1);
        for (n, s) in all.iter().enumerate() {
            let lv = s.build();
            assert!(
                lv.height() >= 5 && lv.height() <= 22,
                "{}: {} rows",
                s.name,
                lv.height()
            );
            assert_eq!(
                lv.rows[0].ground,
                Ground::Bank,
                "{}: far bank on top",
                s.name
            );
            let last = lv.height() - 1;
            assert!(!lv.rows[last].ground.wet(), "{}: dry start", s.name);
            assert!(!lv.blocked(last, START_X), "{}: room to start", s.name);
            assert!(!lv.hazard(last, START_X, 0.0), "{}: safe start", s.name);
            for (i, row) in lv.rows.iter().enumerate() {
                let floats = row.items.iter().any(|it| it.kind.role() == Role::Floater);
                assert_eq!(
                    row.ground.wet(),
                    floats,
                    "{} row {i}: water needs floaters and only water has them",
                    s.name
                );
                if !row.moving() {
                    assert!(
                        row.items.iter().all(|it| it.kind.role() != Role::Hazard),
                        "{} row {i}: standing hazard",
                        s.name
                    );
                }
                // there must be a way through every hedge row
                assert!(
                    (0..40).any(|c| !lv.blocked(i, c as f32 * 2.0)),
                    "{} row {i}: walled off",
                    s.name
                );
            }
            assert_eq!(s.kiss, n, "{}: kisses in story order", s.name);
            assert!(s.world < WORLDS.len());
        }
    }

    #[test]
    fn no_dashes_in_the_story() {
        let mut all: Vec<&str> = KISSES.iter().flat_map(|k| [k.name, k.text]).collect();
        all.extend(PROLOGUE);
        all.extend(FINALE);
        all.extend(GIGGLES);
        all.extend(VERDICTS);
        all.extend(WORLDS.iter().flat_map(|w| [w.name, w.tagline]));
        for s in all {
            assert!(!s.contains('\u{2013}') && !s.contains('\u{2014}'), "{s}");
        }
    }
}
