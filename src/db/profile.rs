use regex::Match;
use serde::{Deserialize, Serialize};

use crate::regex_wrapper::RegexWrapper;
use anyhow::Result;

use super::{Database, StandardEpisode};

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    /// initial parts of search phrase, of which is followed by space and series name
    pub search_prefix: Option<String>,
    /// torrent name parsing regex
    pub parse_regex: RegexWrapper,
    /// if set, is a default path for series relocation. I.e. `relocate`/<series-name>/Season X/episode1.mp4
    pub relocate: Option<String>,
}

impl Profile {
    pub fn save(&self, db: &Database) -> Result<()> {
        db.db.insert(
            format!("profile-{}", self.name),
            serde_json::to_string(self)?.as_bytes(),
        )?;
        Ok(())
    }

    pub fn parse_name<'s, 't>(&'s self, name: &'t str) -> Option<StandardEpisode> {
        let mut out = StandardEpisode::default();
        out.season = 1;
        let captures = self.parse_regex.captures(name)?;
        for name in self.parse_regex.capture_names().filter_map(|x| x) {
            let value = match captures.name(name).as_ref().map(Match::as_str) {
                Some(x) => x,
                None => continue,
            };
            match name {
                "title" => out.title = value.to_string(),
                "season" => out.season = value.parse().ok()?,
                "episode" => out.episode = value.parse().ok()?,
                "checksum" => {
                    out.checksum = u32::from_le_bytes(hex::decode(value).ok()?.try_into().ok()?)
                }
                name => {
                    out.ext.insert(name.to_string(), value.to_string());
                }
            }
        }
        Some(out)
    }
}

impl Database {
    pub fn delete_profile(&self, name: &str) -> Result<()> {
        self.db.remove(&format!("profile-{name}"))?;
        Ok(())
    }

    pub fn get_profile(&self, name: &str) -> Result<Option<Profile>> {
        self.get_serde("profile", name)
    }

    pub fn list_profile(&self) -> Result<Vec<Profile>> {
        self.list_serde("profile-")
    }
}
