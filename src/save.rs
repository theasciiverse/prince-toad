//! Progress, settings and the high-score tables, as plain `key=value`
//! lines in `$XDG_STATE_HOME/prince-toad/save.txt` (~/.local/state/...).
//! Tests never see a state folder, so they never touch a real save.

use std::path::PathBuf;

/// The state folder: save and session log. None in tests.
pub fn state_dir() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".local/state"))
        })?;
    let dir = base.join("prince-toad");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// The high-score tables.
pub const BOARDS: [&str; 3] = ["story", "endless", "today"];
pub const BOARD_TITLES: [&str; 3] = ["The story", "The endless road", "Today's road"];
pub const BOARD_STORY: usize = 0;
pub const BOARD_ENDLESS: usize = 1;
pub const BOARD_TODAY: usize = 2;
/// Entries kept per table.
pub const BOARD_LEN: usize = 8;
pub const NAME_LEN: usize = 10;

/// How fast the world runs, and what it is called.
pub const PACES: [(&str, f32); 3] = [("Normal", 1.0), ("Gentle", 0.75), ("Slow", 0.5)];
/// Lives to start with; the last never runs out.
pub const LIVES_OPTIONS: [&str; 3] = ["5", "9", "Never run out"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub score: u32,
    pub name: String,
    /// How far: "Crossing 7: Willow Ford", "Road 12".
    pub reached: String,
    /// The settings it was played with, empty for the usual ones.
    pub tag: String,
    /// Which game it came from, so a retried story keeps one line.
    pub run: u64,
}

/// Keep only what fits a field between semicolons on one line.
pub fn clean(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| *c != ';' && *c != '=' && !c.is_control())
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Save {
    /// The furthest story crossing unlocked (0 = the first).
    pub reached: usize,
    pub best_story: u32,
    pub best_endless: u32,
    pub finished: bool,
    pub muted: bool,
    /// Sound on but quieter.
    pub quiet: bool,
    /// Today's road: which day, and the best score on it.
    pub daily_day: u64,
    pub daily_best: u32,
    /// Index into PACES.
    pub pace: usize,
    /// Index into LIVES_OPTIONS.
    pub lives: usize,
    /// Keys chosen in the options: (action, key), see `input::ACTIONS`.
    pub keys: Vec<(String, String)>,
    /// The name last typed into a high-score table.
    pub name: String,
    pub boards: [Vec<Entry>; 3],
    /// The endless road's record per world: the quickest crossing, in
    /// tenths of a second (0: none yet). Made at the normal pace only.
    pub records: [u32; RECORD_WORLDS],
    /// The story's boss roads: the quickest climb of each, in tenths of a
    /// second (0: none yet). Made at the normal pace only.
    pub boss_records: [u32; BOSSES],
}

/// Boss roads in the story (1.1).
pub const BOSSES: usize = 4;
/// Where the boss roads went into the story in 1.1, in the old numbering:
/// a save from before them moves its place up past each one.
const BOSSES_BEFORE_OLD: [usize; BOSSES] = [6, 12, 15, 18];

/// The endless road goes through the six worlds that have traffic.
pub const RECORD_WORLDS: usize = 6;
/// Short names for the records line.
pub const RECORD_NAMES: [&str; RECORD_WORLDS] =
    ["Village", "Highway", "River", "Winter", "Night", "Castle"];

/// "4.2s" from tenths.
pub fn tenths(t: u32) -> String {
    format!("{}.{}s", t / 10, t % 10)
}

impl Save {
    pub fn parse(text: &str) -> Save {
        let mut s = Save::default();
        let mut numbered = false;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "reached" => s.reached = v.parse().unwrap_or(0),
                "story" => numbered = v == "2",
                "boss_record" => {
                    if let Some((k, t)) = v.split_once(':')
                        && let (Ok(k), Ok(t)) = (k.parse::<usize>(), t.parse::<u32>())
                        && k < BOSSES
                    {
                        s.boss_records[k] = t;
                    }
                }
                "best_story" => s.best_story = v.parse().unwrap_or(0),
                "best_endless" => s.best_endless = v.parse().unwrap_or(0),
                "finished" => s.finished = v == "1",
                "muted" => s.muted = v == "1",
                "quiet" => s.quiet = v == "1",
                "daily_day" => s.daily_day = v.parse().unwrap_or(0),
                "daily_best" => s.daily_best = v.parse().unwrap_or(0),
                "pace" => s.pace = v.parse::<usize>().unwrap_or(0).min(PACES.len() - 1),
                "lives" => s.lives = v.parse::<usize>().unwrap_or(0).min(LIVES_OPTIONS.len() - 1),
                "name" => s.name = clean(v, NAME_LEN),
                "record" => {
                    if let Some((w, t)) = v.split_once(':')
                        && let (Ok(w), Ok(t)) = (w.parse::<usize>(), t.parse::<u32>())
                        && w < RECORD_WORLDS
                    {
                        s.records[w] = t;
                    }
                }
                "key" => {
                    if let Some((a, k)) = v.split_once(':') {
                        s.keys.push((a.to_string(), k.to_string()));
                    }
                }
                "score" => {
                    let f: Vec<&str> = v.split(';').collect();
                    if f.len() >= 6
                        && let Some(b) = BOARDS.iter().position(|b| *b == f[0])
                        && let Ok(score) = f[1].parse()
                    {
                        s.boards[b].push(Entry {
                            score,
                            name: clean(f[2], NAME_LEN),
                            reached: clean(f[3], 40),
                            tag: clean(f[4], 40),
                            run: f[5].parse().unwrap_or(0),
                        });
                    }
                }
                _ => {}
            }
        }
        for b in &mut s.boards {
            b.sort_by(|a, b| b.score.cmp(&a.score));
            b.truncate(BOARD_LEN);
        }
        if !numbered && s.reached > 0 {
            s.reached += BOSSES_BEFORE_OLD.iter().filter(|&&b| b < s.reached).count();
        }
        s
    }

    pub fn text(&self) -> String {
        let mut t = format!(
            "story=2\nreached={}\nbest_story={}\nbest_endless={}\nfinished={}\nmuted={}\nquiet={}\ndaily_day={}\ndaily_best={}\npace={}\nlives={}\nname={}\n",
            self.reached,
            self.best_story,
            self.best_endless,
            self.finished as u8,
            self.muted as u8,
            self.quiet as u8,
            self.daily_day,
            self.daily_best,
            self.pace,
            self.lives,
            self.name
        );
        for (a, k) in &self.keys {
            t.push_str(&format!("key={a}:{k}\n"));
        }
        for (w, r) in self.records.iter().enumerate() {
            if *r > 0 {
                t.push_str(&format!("record={w}:{r}\n"));
            }
        }
        for (k, r) in self.boss_records.iter().enumerate() {
            if *r > 0 {
                t.push_str(&format!("boss_record={k}:{r}\n"));
            }
        }
        for (b, entries) in self.boards.iter().enumerate() {
            for e in entries {
                t.push_str(&format!(
                    "score={};{};{};{};{};{}\n",
                    BOARDS[b], e.score, e.name, e.reached, e.tag, e.run
                ));
            }
        }
        t
    }

    /// Where a score would go in a table (None: it does not make it), and
    /// whether this game already has a line there.
    pub fn place(&self, board: usize, score: u32, run: u64) -> Option<usize> {
        if score == 0 {
            return None;
        }
        let b = &self.boards[board];
        if let Some(old) = b.iter().find(|e| e.run == run && run != 0)
            && old.score >= score
        {
            return None;
        }
        let others: Vec<&Entry> = b.iter().filter(|e| e.run != run || run == 0).collect();
        let at = others
            .iter()
            .position(|e| score > e.score)
            .unwrap_or(others.len());
        (at < BOARD_LEN).then_some(at)
    }

    /// Put an entry in its table (replacing this game's older line); where it went.
    pub fn insert(&mut self, board: usize, e: Entry) -> Option<usize> {
        self.place(board, e.score, e.run)?;
        let b = &mut self.boards[board];
        if e.run != 0 {
            b.retain(|x| x.run != e.run);
        }
        let at = b.iter().position(|x| e.score > x.score).unwrap_or(b.len());
        b.insert(at, e);
        b.truncate(BOARD_LEN);
        Some(at)
    }

    pub fn load() -> Save {
        state_dir()
            .and_then(|d| std::fs::read_to_string(d.join("save.txt")).ok())
            .map(|t| Save::parse(&t))
            .unwrap_or_default()
    }

    pub fn store(&self) {
        let Some(dir) = state_dir() else { return };
        let path = dir.join("save.txt");
        let tmp = dir.join("save.txt.new");
        if std::fs::write(&tmp, self.text())
            .and_then(|_| std::fs::rename(&tmp, &path))
            .is_err()
        {
            log::warn!("could not write {}", path.display());
        } else {
            log::debug!("saved: {:?}", self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(score: u32, run: u64) -> Entry {
        Entry {
            score,
            name: format!("T{score}"),
            reached: "Road 3".into(),
            tag: String::new(),
            run,
        }
    }

    #[test]
    fn round_trip_and_no_folder_in_tests() {
        let mut s = Save {
            reached: 7,
            best_story: 1234,
            best_endless: 99,
            finished: true,
            muted: true,
            quiet: true,
            daily_day: 20_000,
            daily_best: 55,
            pace: 2,
            lives: 1,
            keys: vec![
                ("p1_up".into(), "i".into()),
                ("p2_left".into(), "home".into()),
            ],
            name: "Greta".into(),
            boards: Default::default(),
            records: [31, 0, 52, 0, 0, 120],
            boss_records: [412, 0, 0, 655],
        };
        s.insert(BOARD_ENDLESS, entry(500, 1));
        s.insert(
            BOARD_STORY,
            Entry {
                tag: "Slow, 9 lives".into(),
                ..entry(900, 2)
            },
        );
        assert_eq!(Save::parse(&s.text()), s);
        assert_eq!(
            Save::parse("junk\nreached=x\nscore=nope;1\n"),
            Save::default()
        );
        assert!(state_dir().is_none());
    }

    #[test]
    fn a_save_from_before_the_boss_roads_keeps_its_place() {
        // old 6 was Willow Ford, the river: now the Long Haul comes first
        assert_eq!(Save::parse("reached=6\n").reached, 6);
        assert_eq!(Save::parse("reached=7\n").reached, 8);
        assert_eq!(Save::parse("reached=13\n").reached, 15);
        // old 18 was the pond: now the Dragon Wakes comes first
        assert_eq!(Save::parse("reached=18\n").reached, 21);
        assert_eq!(Save::parse("story=2\nreached=7\n").reached, 7);
    }

    #[test]
    fn tables_keep_the_best_and_one_line_a_game() {
        let mut s = Save::default();
        assert_eq!(s.place(BOARD_STORY, 0, 1), None, "nothing for nothing");
        for i in 1..=BOARD_LEN as u32 {
            s.insert(BOARD_STORY, entry(i * 100, i as u64));
        }
        assert_eq!(s.boards[BOARD_STORY][0].score, 800);
        assert_eq!(
            s.place(BOARD_STORY, 50, 99),
            None,
            "too low for a full table"
        );
        assert_eq!(s.insert(BOARD_STORY, entry(450, 99)), Some(4));
        assert_eq!(s.boards[BOARD_STORY].len(), BOARD_LEN);
        assert_eq!(s.boards[BOARD_STORY].last().unwrap().score, 200);
        // the same game again: one line, the better score
        assert_eq!(s.place(BOARD_STORY, 300, 99), None);
        assert_eq!(s.insert(BOARD_STORY, entry(950, 99)), Some(0));
        assert_eq!(
            s.boards[BOARD_STORY].iter().filter(|e| e.run == 99).count(),
            1
        );
        assert_eq!(clean("a;b=c\nd", 10), "abcd");
    }
}
