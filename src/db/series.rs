use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Database, Episode, ParsedSearchResult, PullState};
use anyhow::Result;

fn default_relocate_season() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Series {
    pub name: String,
    /// profile name to search for this series
    pub profile: String,
    /// override for `SearchConfig::max_days_old`
    pub max_days_old: Option<u64>,
    /// if set, overrides `Profile::relocate`/<series-name> default path
    pub relocate: Option<String>,
    /// if true, `Season X` is appended to the relocate path
    #[serde(default = "default_relocate_season")]
    pub relocate_season: bool,
}

impl Series {
    pub fn save(&self, db: &Database) -> Result<()> {
        db.db.insert(
            format!("series-{}", self.name),
            serde_json::to_string(self)?.as_bytes(),
        )?;
        Ok(())
    }

    pub fn status(self, db: &Database) -> Result<SeriesStatus> {
        let mut seasons: BTreeMap<u32, SeasonStatus> = BTreeMap::new();
        let pulls = db.list_pull_entry_series(&self.name)?;
        for pull in pulls {
            seasons
                .entry(pull.result.parsed.season)
                .or_default()
                .episodes
                .insert(
                    pull.result.parsed.episode.clone(),
                    EpisodeStatus {
                        state: pull.state,
                        source: pull.result,
                    },
                );
        }
        Ok(SeriesStatus {
            seasons,
            series: self,
        })
    }
}

impl Database {
    pub fn delete_series(&self, name: &str) -> Result<()> {
        self.db.remove(&format!("series-{name}"))?;
        Ok(())
    }

    pub fn get_series(&self, name: &str) -> Result<Option<Series>> {
        self.get_serde("series", name)
    }

    pub fn list_series(&self) -> Result<Vec<Series>> {
        self.list_serde("series-")
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SeriesStatus {
    #[serde(flatten)]
    pub series: Series,
    pub seasons: BTreeMap<u32, SeasonStatus>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SeasonStatus {
    pub episodes: BTreeMap<Episode, EpisodeStatus>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EpisodeStatus {
    pub state: PullState,
    pub source: ParsedSearchResult,
}
