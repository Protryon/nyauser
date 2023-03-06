use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::source::SearchResult;

use super::Database;
use anyhow::Result;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StandardEpisode {
    pub title: String,
    pub season: u32,
    pub episode: u32,
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
