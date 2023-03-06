mod nyaa;
pub use nyaa::{NyaaClient, NyaaConfig};

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub torrent_link: String,
    pub view_link: String,
    pub date: DateTime<FixedOffset>,
    pub seeders: u64,
    pub leechers: u64,
    pub downloads: u64,
    pub size: u64,
}

#[async_trait::async_trait]
pub trait Source: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
}

#[async_trait::async_trait]
impl Source for Box<dyn Source + Send + Sync> {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        Source::search(&**self, query).await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SourceConfig {
    Nyaa(NyaaConfig),
}
