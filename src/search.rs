use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use chrono::Utc;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sled::Db;
use tokio::select;

use crate::{
    profile::{ProfileConfig, StandardEpisode},
    series::SeriesConfig,
    sink::Sink,
    source::{SearchResult, Source},
};

fn default_source_sink() -> String {
    "default".to_string()
}

#[derive(Deserialize, Clone)]
pub struct SearchConfig {
    /// how many days old can a torrent be to be considered
    /// can be overridden in `SeriesConfig`
    pub max_days_old: u64,
    /// minimum seeders required to consider a torrent
    /// usually 1
    pub min_seeders: u64,
    /// how many minutes between search scans
    pub search_minutes: u64,
    /// how many minutes between scans of the sink (i.e. transmission)
    /// for completed torrents.
    pub completion_check_minutes: u64,
    /// name of source to search, defaulting to `default`
    #[serde(default = "default_source_sink")]
    pub source: String,
    /// name of sink to fetch with, defaulting to `default`
    #[serde(default = "default_source_sink")]
    pub sink: String,
    #[serde(default)]
    pub profiles: IndexMap<String, ProfileConfig>,
    #[serde(default)]
    pub series: IndexMap<String, SeriesConfig>,
    // path prefix patches
    #[serde(default)]
    pub path_patch: IndexMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PullState {
    Downloading,
    Finished,
}

impl Default for PullState {
    fn default() -> Self {
        PullState::Finished
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullEntry {
    result: ParsedSearchResult,
    torrent_id: Option<i64>,
    // only None in legacy
    torrent_hash: Option<String>,
    // default for legacy
    #[serde(default)]
    state: PullState,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParsedSearchResult {
    result: SearchResult,
    parsed: StandardEpisode,
    profile: String,
    relocate: Option<String>,
    relocate_season: bool,
}

impl ParsedSearchResult {
    fn id(&self) -> String {
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

pub async fn wipe_nonexistant(db: &Db) {
    for (key, value) in db.scan_prefix("torrent-").map(|x| x.unwrap()) {
        let key = std::str::from_utf8(&key[..]).unwrap();
        let value = std::str::from_utf8(&value[..]).unwrap();
        let pull_entry: PullEntry =
            serde_json::from_str(value).expect("failed to parse pull entry");
        if !matches!(pull_entry.state, PullState::Finished) {
            continue;
        }
        let relocate = if let Some(x) = pull_entry.result.relocate_dir() {
            x
        } else {
            continue;
        };
        if !relocate.exists() {
            info!("{} doesn't exist, deleting {}", relocate.display(), key);
            db.remove(key).unwrap();
        }
    }
}

pub struct Searcher<I: Source, O: Sink> {
    source: I,
    sink: O,
    db: Db,
    config: SearchConfig,
}

impl<I: Source, O: Sink> Searcher<I, O> {
    pub fn new(db: Db, source: I, sink: O, config: SearchConfig) -> Result<Self> {
        Ok(Self {
            source,
            sink,
            db,
            config,
        })
    }

    pub async fn run(mut self) {
        let mut scan_interval = tokio::time::interval(Duration::from_secs(
            self.config.completion_check_minutes * 60,
        ));
        let mut search_interval =
            tokio::time::interval(Duration::from_secs(self.config.search_minutes * 60));
        loop {
            select! {
                _ = scan_interval.tick() => {
                    if let Err(e) = self.scan_completed().await {
                        error!("failed to run scan: {:?}", e);
                    }
                },
                _ = search_interval.tick() => {
                    if let Err(e) = self.run_iter().await {
                        error!("failed to run search: {:?}", e);
                    }
                },
            }
        }
    }

    async fn scan_completed(&mut self) -> Result<()> {
        debug!("scan starting");
        self.clean().await?;

        let finished = self.sink.finished().await?;
        for torrent in finished {
            let downloading_id = format!("downloading-{}", torrent.id);
            let internal_id = match self.db.get(&downloading_id)? {
                None => continue,
                Some(x) => String::from_utf8_lossy(&x[..]).into_owned(),
            };
            let dbid = format!("torrent-{}", internal_id);
            let mut pull_entry: PullEntry = match self.db.get(&dbid)? {
                None => {
                    error!("missing torrent for id {}", internal_id);
                    continue;
                }
                Some(x) => serde_json::from_str(&*String::from_utf8_lossy(&x[..]))?,
            };
            info!("torrent = {:?}, pe = {:?}", torrent, pull_entry);
            if let Some(relocate) = pull_entry.result.relocate_dir() {
                let mut download_dir = torrent.download_dir.clone();
                for (patch, to) in &self.config.path_patch {
                    if let Some(suffix) = download_dir.strip_prefix(patch) {
                        download_dir = format!("{to}{suffix}");
                        break;
                    }
                }
                let download_dir = Path::new(&*download_dir);
                for file in torrent.files {
                    let new_file = relocate.join(&*file);
                    let old_file = download_dir.join(&*file);
                    if old_file.exists() {
                        tokio::fs::create_dir_all(new_file.parent().unwrap()).await?;
                        tokio::fs::rename(old_file, new_file).await?;
                    }
                }
            }
            pull_entry.state = PullState::Finished;
            self.db
                .insert(&dbid, serde_json::to_string(&pull_entry)?.as_bytes())?;
            self.db.remove(&downloading_id)?;
            self.sink.delete(torrent.id).await?;
        }
        Ok(())
    }

    pub async fn clean(&mut self) -> Result<()> {
        for torrent in self.db.scan_prefix("downloading-") {
            let (key, value) = torrent?;
            let key = std::str::from_utf8(&key[..])?;
            let internal_id = std::str::from_utf8(&value[..])?;
            let torrent_id: i64 = key.strip_prefix("downloading-").unwrap().parse()?;

            let pull_entry: PullEntry = match self.db.get(format!("torrent-{}", internal_id))? {
                None => {
                    error!("missing torrent for id {}", internal_id);
                    self.db.remove(key)?;
                    continue;
                }
                Some(x) => serde_json::from_str(&*String::from_utf8_lossy(&x[..]))?,
            };

            if matches!(pull_entry.state, PullState::Finished) {
                info!(
                    "removing stale download tag for finished torrent: {}",
                    internal_id
                );
                // remove stale download key that we scanned
                self.db.remove(key)?;
                continue;
            }

            match self.sink.check(torrent_id).await? {
                Some(info) => {
                    if let Some(torrent_hash) = &pull_entry.torrent_hash {
                        if torrent_hash != &info.hash {
                            info!("removing id-stale torrent: {}", internal_id);
                            self.db.remove(key)?;
                            self.db.remove(format!("torrent-{}", internal_id))?;
                        }
                    }
                    // otherwise inprogress or finished, and not `clean`s concern
                }
                None => {
                    info!("removing stale torrent: {}", internal_id);
                    self.db.remove(key)?;
                    self.db.remove(format!("torrent-{}", internal_id))?;
                }
            }
        }
        Ok(())
    }

    async fn run_iter(&mut self) -> Result<()> {
        info!("round starting");
        self.clean().await?;

        let mut candidates = vec![];
        for (series_name, series) in self.config.series.iter() {
            let profile = match self.config.profiles.get(&series.profile) {
                Some(x) => x,
                None => {
                    error!("missing/invalid profile for '{}'", series_name);
                    continue;
                }
            };
            let days_old = series
                .max_days_old
                .map(|x| self.config.max_days_old.max(x))
                .unwrap_or(self.config.max_days_old);

            let search = match profile.search_prefix.as_ref() {
                Some(prefix) => format!("{} {}", prefix, series_name),
                None => series_name.to_string(),
            };
            match self.source.search(&*search).await {
                Ok(items) => {
                    for item in items {
                        let since = Utc::now().signed_duration_since(item.date);
                        if since > chrono::Duration::days(days_old as i64)
                            || item.seeders < self.config.min_seeders
                        {
                            continue;
                        }
                        let parsed = match profile.parse_name(&*item.title) {
                            Some(x) => x,
                            None => {
                                warn!("failed to parse title: '{}'", item.title);
                                continue;
                            }
                        };
                        candidates.push(ParsedSearchResult {
                            result: item,
                            parsed,
                            profile: series.profile.to_string(),
                            relocate: series.relocate.clone().or_else(|| {
                                Some(
                                    Path::new(profile.relocate.as_ref()?)
                                        .join(series_name)
                                        .to_string_lossy()
                                        .into_owned(),
                                )
                            }),
                            relocate_season: series.relocate_season,
                        })
                    }
                }
                Err(e) => {
                    error!("failure to search '{}': {:?}", search, e);
                }
            }
        }

        info!("found {} candidates", candidates.len());
        debug!(
            "{:<115} {:<5} {:<7} {:<7} {:<9} {:<9} {:<15}",
            "TITLE", "DATE", "SEEDERS", "LEECHERS", "DOWNLOADS", "SIZE (MB)", "VIEW"
        );
        for candidate in candidates {
            let id = candidate.id();
            debug!(
                "{:<115} {:<5} {:>7} {:>7} {:>9} {: >9.02} {:<15}",
                candidate.result.title,
                candidate.result.date.format("%Y-%m-%d %H:%M:%S"),
                candidate.result.seeders,
                candidate.result.leechers,
                candidate.result.downloads,
                (candidate.result.size as f64) / 1024.0 / 1024.0,
                candidate.result.view_link
            );
            let dbid = format!("torrent-{}", id);
            if self.db.contains_key(&dbid)? {
                continue;
            }
            // TODO: legacy (remove)
            if self
                .db
                .contains_key(&format!("torrent-{}_{}", candidate.profile, id))?
            {
                continue;
            }
            info!(
                "starting download for '{}' from {} ({})",
                id, candidate.result.view_link, candidate.result.date
            );

            let torrent_info = match self.sink.push(&*candidate.result.torrent_link).await {
                Err(e) => {
                    error!("failed to push torrent '{}': {:?}", id, e);
                    continue;
                }
                Ok(Some(out)) => out,
                Ok(None) => {
                    warn!("torrent already present: {}", id);
                    continue;
                }
            };
            let pull_entry = PullEntry {
                result: candidate,
                torrent_id: Some(torrent_info.id),
                torrent_hash: Some(torrent_info.hash),
                state: PullState::Downloading,
            };
            self.db
                .insert(dbid, serde_json::to_string(&pull_entry)?.as_bytes())?;
            self.db
                .insert(format!("downloading-{}", torrent_info.id), id.as_bytes())?;
            self.db.flush_async().await?;
        }
        Ok(())
    }
}
