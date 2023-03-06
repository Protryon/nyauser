mod transmission;
use serde::{Deserialize, Serialize};
pub use transmission::*;

use anyhow::Result;

#[derive(Debug)]
pub struct FinishedTorrent {
    pub id: i64,
    pub download_dir: String,
    pub files: Vec<String>,
}

pub enum TorrentStatus {
    Finished,
    InProgress,
}

pub struct TorrentInfo {
    pub id: i64,
    pub hash: String,
    pub status: TorrentStatus,
}

#[async_trait::async_trait]
pub trait Sink: Send + Sync {
    /// Ok(None) -> already present
    async fn push(&mut self, torrent_url: &str) -> Result<Option<TorrentInfo>>;

    async fn check(&mut self, id: i64) -> Result<Option<TorrentInfo>>;

    async fn finished(&mut self) -> Result<Vec<FinishedTorrent>>;

    async fn delete(&mut self, id: i64) -> Result<()>;
}

#[async_trait::async_trait]
impl Sink for Box<dyn Sink + Send + Sync> {
    async fn push(&mut self, torrent_url: &str) -> Result<Option<TorrentInfo>> {
        Sink::push(&mut **self, torrent_url).await
    }

    async fn check(&mut self, id: i64) -> Result<Option<TorrentInfo>> {
        Sink::check(&mut **self, id).await
    }

    async fn finished(&mut self) -> Result<Vec<FinishedTorrent>> {
        Sink::finished(&mut **self).await
    }

    async fn delete(&mut self, id: i64) -> Result<()> {
        Sink::delete(&mut **self, id).await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SinkConfig {
    Transmission(TransmissionConfig),
}
