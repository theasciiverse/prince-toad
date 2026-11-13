//! Progress and settings, as plain `key=value` lines in
//! `$XDG_STATE_HOME/prince-toad/save.txt` (~/.local/state/...).
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Save {
    /// The furthest story crossing unlocked (0 = the first).
    pub reached: usize,
    pub best_story: u32,
    pub best_endless: u32,
    pub finished: bool,
    pub muted: bool,
}

impl Save {
    pub fn parse(text: &str) -> Save {
        let mut s = Save::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "reached" => s.reached = v.parse().unwrap_or(0),
                "best_story" => s.best_story = v.parse().unwrap_or(0),
                "best_endless" => s.best_endless = v.parse().unwrap_or(0),
                "finished" => s.finished = v == "1",
                "muted" => s.muted = v == "1",
                _ => {}
            }
        }
        s
    }

    pub fn text(&self) -> String {
        format!(
            "reached={}\nbest_story={}\nbest_endless={}\nfinished={}\nmuted={}\n",
            self.reached, self.best_story, self.best_endless, self.finished as u8, self.muted as u8
        )
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

    #[test]
    fn round_trip_and_no_folder_in_tests() {
        let s = Save {
            reached: 7,
            best_story: 1234,
            best_endless: 99,
            finished: true,
            muted: true,
        };
        assert_eq!(Save::parse(&s.text()), s);
        assert_eq!(Save::parse("junk\nreached=x\n"), Save::default());
        assert!(state_dir().is_none());
    }
}
