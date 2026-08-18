use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::typing::SessionRecord;

const HISTORY_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mode_index: usize,
    pub time_index: usize,
    pub word_index: usize,
    pub punctuation: bool,
    pub numbers: bool,
    #[serde(default)]
    pub extended_words: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode_index: 0,
            time_index: 1,
            word_index: 1,
            punctuation: false,
            numbers: false,
            extended_words: false,
        }
    }
}

pub struct Storage {
    history_path: PathBuf,
    settings_path: PathBuf,
}

impl Storage {
    pub fn new() -> io::Result<Self> {
        let project_dirs = ProjectDirs::from("dev", "TerminalTyper", "TerminalTyper")
            .ok_or_else(|| io::Error::other("local data directory is unavailable"))?;
        let data_dir = project_dirs.data_local_dir();
        fs::create_dir_all(data_dir)?;
        Ok(Self {
            history_path: data_dir.join("history.json"),
            settings_path: data_dir.join("settings.json"),
        })
    }

    pub fn load_history(&self) -> io::Result<Vec<SessionRecord>> {
        match fs::read_to_string(&self.history_path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    pub fn save_history(&self, history: &[SessionRecord]) -> io::Result<()> {
        let start = history.len().saturating_sub(HISTORY_LIMIT);
        let encoded = serde_json::to_string_pretty(&history[start..]).map_err(io::Error::other)?;
        write_atomic(&self.history_path, &encoded)
    }

    pub fn load_settings(&self) -> Option<Settings> {
        let contents = fs::read_to_string(&self.settings_path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        let encoded = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
        write_atomic(&self.settings_path, &encoded)
    }
}

/// Write via a sibling temp file so a crash mid-write cannot truncate the real file.
fn write_atomic(path: &std::path::Path, contents: &str) -> io::Result<()> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, contents)?;
    fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::typing::PracticeMode;

    #[test]
    fn history_round_trips_through_json() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "terminal-typer-storage-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("temporary directory should be created");
        let storage = Storage {
            history_path: root.join("history.json"),
            settings_path: root.join("settings.json"),
        };
        let expected = vec![SessionRecord {
            finished_at: 1_800_000_000,
            mode: PracticeMode::Words,
            amount: 25,
            wpm: 72.5,
            raw_wpm: 75.0,
            accuracy: 96.7,
            errors: 2,
            elapsed_ms: 12_400,
            correct_chars: 75,
            wpm_samples: vec![70.0, 74.0, 73.0],
            consistency: 91.2,
        }];

        storage
            .save_history(&expected)
            .expect("history should be written");
        let actual = storage.load_history().expect("history should be read");

        assert_eq!(actual, expected);
        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }
}
