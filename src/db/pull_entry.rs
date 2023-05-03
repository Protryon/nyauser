use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::source::SearchResult;

use super::Database;
use anyhow::Result;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PullEntry {
    pub result: ParsedSearchResult,
    pub torrent_id: Option<i64>,
    pub torrent_hash: String,
    pub state: PullState,
}

impl PullEntry {
    pub fn key(&self) -> String {
        self.result.key()
    }

    /// includes an interior save
    pub fn clear_torrent_id(&mut self, db: &Database) -> Result<()> {
        if let Some(torrent_id) = self.torrent_id.take() {
            db.db.remove(format!("downloading-{}", torrent_id))?;
        }
        self.save(db)?;
        Ok(())
    }

    pub fn save(&self, db: &Database) -> Result<()> {
        let key = self.key();
        db.db.insert(
            format!("torrent-{key}"),
            serde_json::to_string(self)?.as_bytes(),
        )?;
        if let Some(torrent_id) = self.torrent_id {
            db.db
                .insert(format!("downloading-{}", torrent_id), key.as_bytes())?;
        }
        Ok(())
    }

    pub fn delete(mut self, db: &Database) -> Result<()> {
        if let Some(torrent_id) = self.torrent_id.take() {
            db.db.remove(format!("downloading-{}", torrent_id))?;
        }
        db.db.remove(format!("torrent-{}", self.key()))?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PullState {
    Downloading,
    Finished,
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

impl Database {
    pub fn exists_pull_entry(&self, key: &str) -> Result<bool> {
        Ok(self.db.contains_key(format!("torrent-{key}"))?)
    }

    pub fn get_pull_entry(&self, key: &str) -> Result<Option<PullEntry>> {
        self.get_serde("torrent", key)
    }

    pub fn get_pull_entry_from_torrent_id(&self, id: i64) -> Result<Option<PullEntry>> {
        let downloading_id = format!("downloading-{}", id);
        let internal_id = match self.db.get(&downloading_id)? {
            None => return Ok(None),
            Some(x) => String::from_utf8(x.to_vec())?,
        };
        self.get_pull_entry(&internal_id)
    }

    pub fn list_pull_entry_series(&self, name: &str) -> Result<Vec<PullEntry>> {
        self.list_serde(&format!("torrent-{name}_S"))
    }

    pub fn list_pull_entry(&self) -> Result<Vec<PullEntry>> {
        self.list_serde("torrent-")
    }

    pub fn list_pull_entry_downloading(&self) -> Result<Vec<PullEntry>> {
        let mut out = vec![];
        for entry in self.db.scan_prefix("downloading-") {
            let (_, value) = entry?;
            let key = std::str::from_utf8(&value[..])?;
            let Some(pull_entry) = self.get_pull_entry(&key)? else {
                bail!("dangling `downloading` key");
            };
            out.push(pull_entry);
        }
        Ok(out)
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
