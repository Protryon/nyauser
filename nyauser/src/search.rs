use std::{path::Path, sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use indexmap::IndexMap;
use nyauser_types::{ParsedSearchResult, PullEntry, PullState};
use serde::Deserialize;
use tokio::{select, sync::Notify};

use crate::{db::Database, sink::Sink, source::Source};

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
    // path prefix patches
    #[serde(default)]
    pub path_patch: IndexMap<String, String>,
    pub relocate: Option<String>,
}

pub fn wipe_nonexistant(db: &Database) -> Result<()> {
    for pull_entry in db.list_pull_entry()? {
        if !matches!(pull_entry.state, PullState::Finished) {
            continue;
        }
        let relocate = if let Some(x) = pull_entry.result.relocate_dir() {
            x
        } else {
            continue;
        };
        let should_delete = if pull_entry.files.is_empty() {
            !relocate.exists()
        } else {
            pull_entry
                .files
                .iter()
                .all(|file| !relocate.join(file).exists())
        };
        if should_delete {
            info!(
                "{} doesn't exist, deleting {}",
                relocate.display(),
                pull_entry.key()
            );
            db.delete_pull(pull_entry)?;
        }
    }
    Ok(())
}

pub struct Searcher<I: Source, O: Sink> {
    source: I,
    sink: O,
    db: Arc<Database>,
    config: SearchConfig,
    search: Arc<Notify>,
    scan: Arc<Notify>,
}

impl<I: Source, O: Sink> Searcher<I, O> {
    pub fn new(db: Arc<Database>, source: I, sink: O, config: SearchConfig) -> Result<Self> {
        Ok(Self {
            source,
            sink,
            db,
            config,
            search: Arc::new(Notify::new()),
            scan: Arc::new(Notify::new()),
        })
    }

    pub fn search(&self) -> &Arc<Notify> {
        &self.search
    }

    pub fn scan(&self) -> &Arc<Notify> {
        &self.scan
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
                _ = self.scan.notified() => {
                    if let Err(e) = self.scan_completed().await {
                        error!("failed to run scan: {:?}", e);
                    }
                },
                _ = search_interval.tick() => {
                    if let Err(e) = self.run_iter().await {
                        error!("failed to run search: {:?}", e);
                    }
                },
                _ = self.search.notified() => {
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
            let Some(mut pull_entry) = self.db.get_pull_entry_from_torrent_id(torrent.id)? else {
                continue;
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
                    pull_entry.files.push(file);
                }
            } else {
                pull_entry.files.extend(torrent.files);
            }
            pull_entry.state = PullState::Finished;
            self.db.clear_torrent_id(&mut pull_entry)?;
            self.sink.delete(torrent.id).await?;
        }
        Ok(())
    }

    pub async fn clean(&mut self) -> Result<()> {
        for pull_entry in self.db.list_pull_entry_downloading()? {
            let Some(torrent_id) = pull_entry.torrent_id else {
                warn!(
                    "torrent_id missing in downloading-indexed pull_entry: {}",
                    pull_entry.key()
                );
                continue;
            };

            match self.sink.check(torrent_id).await? {
                Some(info) => {
                    if pull_entry.torrent_hash != info.hash {
                        info!("removing id-stale torrent: {}", pull_entry.key());
                        self.db.delete_pull(pull_entry)?;
                    }
                    // otherwise inprogress or finished, and not `clean`s concern
                }
                None => {
                    info!("removing stale torrent: {}", pull_entry.key());
                    self.db.delete_pull(pull_entry)?;
                }
            }
        }
        Ok(())
    }

    async fn run_iter(&mut self) -> Result<()> {
        info!("round starting");
        self.clean().await?;

        let mut candidates = vec![];
        for series in self.db.list_series()? {
            info!("Searching for {}", series.name);
            let profile = match self.db.get_profile(&series.profile)? {
                Some(x) => x,
                None => {
                    error!("missing/invalid profile for '{}'", series.name);
                    continue;
                }
            };
            let days_old = series
                .max_days_old
                .map(|x| self.config.max_days_old.max(x))
                .unwrap_or(self.config.max_days_old);

            let search = match profile.search_prefix.as_ref() {
                Some(prefix) => format!("{} {}", prefix, series.name),
                None => series.name.clone(),
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
                        // Order for relocate:
                        // series > profile > search
                        let relocate = if series.relocate.is_some() {
                            series.relocate.clone()
                        } else {
                            let mut base = if profile.relocate.is_some() {
                                profile.relocate.clone()
                            } else {
                                self.config.relocate.clone()
                            };
                            if let Some(base) = base.as_mut() {
                                if !base.ends_with('/') {
                                    base.push('/');
                                }
                                base.push_str(&series.name);
                            }
                            base
                        };
                        candidates.push(ParsedSearchResult {
                            result: item,
                            parsed,
                            profile: series.profile.to_string(),
                            relocate,
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
            let id = candidate.key();
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
            if self.db.exists_pull_entry(&id)? {
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
                torrent_hash: torrent_info.hash,
                state: PullState::Downloading,
                files: vec![],
            };
            self.db.save_pull(&pull_entry)?;
            self.db.flush().await?;
        }
        Ok(())
    }
}
