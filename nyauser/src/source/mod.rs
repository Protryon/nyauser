mod nyaa;
pub use nyaa::{NyaaClient, NyaaConfig};

use anyhow::Result;
use nyauser_types::SearchResult;
use serde::{Deserialize, Serialize};

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
