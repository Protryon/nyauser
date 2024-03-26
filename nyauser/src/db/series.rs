use std::collections::BTreeMap;

use nyauser_types::{EpisodeStatus, SeasonStatus, Series, SeriesStatus};

use super::Database;
use anyhow::Result;

impl Database {
    pub fn save_series(&self, series: &Series) -> Result<()> {
        self.db.insert(
            format!("series-{}", series.name),
            serde_json::to_string(series)?.as_bytes(),
        )?;
        Ok(())
    }

    pub fn series_status(&self, series: Series) -> Result<SeriesStatus> {
        let mut seasons: BTreeMap<u32, SeasonStatus> = BTreeMap::new();
        let pulls = self.list_pull_entry_series(&series.name)?;
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
        Ok(SeriesStatus { seasons, series })
    }

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
