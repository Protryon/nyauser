use serde::Deserialize;

fn serde_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct SeriesConfig {
    /// profile name to search for this series
    pub profile: String,
    /// override for `SearchConfig::max_days_old`
    pub max_days_old: Option<u64>,
    /// if set, overrides `ProfileConfig::relocate`/<series-name> default path
    pub relocate: Option<String>,
    /// if true, `Season X` is appended to the relocate path
    #[serde(default = "serde_true")]
    pub relocate_season: bool,
}
