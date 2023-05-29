use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Episode, ParsedSearchResult, PullState};

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
