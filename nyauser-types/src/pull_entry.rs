use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PullEntryFilter {
    pub profile: Option<String>,
    pub title_contains: Option<String>,
    pub title_is: Option<String>,
    pub season_is: Option<u32>,
    pub episode_is: Option<Episode>,
    pub state: Option<PullState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub torrent_link: String,
    pub view_link: String,
    pub date: DateTime<FixedOffset>,
    pub seeders: u64,
    pub leechers: u64,
    pub downloads: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Episode {
    Standard(u32),
    Special(String),
}

impl PartialOrd for Episode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Episode::Standard(e1), Episode::Standard(e2)) => e1.partial_cmp(e2),
            (Episode::Standard(_), Episode::Special(_)) => Some(Ordering::Less),
            (Episode::Special(_), Episode::Standard(_)) => Some(Ordering::Greater),
            (Episode::Special(e1), Episode::Special(e2)) => e1.partial_cmp(e2),
        }
    }
}

impl Ord for Episode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl FromStr for Episode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.parse().ok() {
            Ok(Episode::Standard(s))
        } else {
            Ok(Episode::Special(s.to_string()))
        }
    }
}

impl Default for Episode {
    fn default() -> Self {
        Episode::Standard(0)
    }
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Episode::Standard(x) => write!(f, "{}", x),
            Episode::Special(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StandardEpisode {
    pub title: String,
    pub season: u32,
    pub episode: Episode,
    pub checksum: u32,
    pub ext: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct PullEntryNamed {
    pub id: String,
    #[serde(flatten)]
    pub pull_entry: PullEntry,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullEntry {
    pub result: ParsedSearchResult,
    pub torrent_id: Option<i64>,
    pub torrent_hash: String,
    pub state: PullState,
    #[serde(default)]
    pub files: Vec<String>,
}

impl PullEntry {
    pub fn key(&self) -> String {
        self.result.key()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PullState {
    Downloading,
    Finished,
}

impl fmt::Display for PullState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PullState::Downloading => write!(f, "downloading"),
            PullState::Finished => write!(f, "finished"),
        }
    }
}

impl FromStr for PullState {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "downloading" => Ok(PullState::Downloading),
            "finished" => Ok(PullState::Finished),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedSearchResult {
    pub result: SearchResult,
    pub parsed: StandardEpisode,
    pub profile: String,
    pub relocate: Option<String>,
    pub relocate_season: bool,
}

impl ParsedSearchResult {
    pub fn key(&self) -> String {
        format!(
            "{}_S{:02}E{:02}",
            self.parsed.title, self.parsed.season, self.parsed.episode
        )
    }

    pub fn relocate_dir(&self) -> Option<PathBuf> {
        let mut relocate = Path::new(self.relocate.as_ref()?).to_owned();
        if self.relocate_season {
            relocate = relocate.join(format!("Season {}", self.parsed.season));
        }
        Some(relocate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_order() {
        assert!(Episode::Standard(5) < Episode::Standard(10));
        assert!(Episode::Standard(15) < Episode::Special("test".to_string()));
        assert!(Episode::Special("2".to_string()) < Episode::Special("20".to_string()));
    }
}
