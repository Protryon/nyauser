use std::io::BufReader;

use chrono::{DateTime, FixedOffset};
use rss::Channel;
use serde::{Deserialize, Serialize};

use anyhow::Result;

use super::{SearchResult, Source};

fn default_url() -> String {
    "https://nyaa.si/?page=rss&c=0_0&f=0&q=".to_string()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NyaaConfig {
    #[serde(default = "default_url")]
    url: String,
}

pub struct NyaaClient {
    client: reqwest::Client,
    config: NyaaConfig,
}

#[derive(Debug, Clone)]
pub struct NyaaResult {
    title: String,
    torrent_link: String,
    view_link: String,
    date: DateTime<FixedOffset>,
    seeders: u64,
    leechers: u64,
    downloads: u64,
    /// Anime - English-translated
    category: String,
    size: u64,
    trusted: bool,
}

impl Into<SearchResult> for NyaaResult {
    fn into(self) -> SearchResult {
        SearchResult {
            title: self.title,
            torrent_link: self.torrent_link,
            view_link: self.view_link,
            date: self.date,
            seeders: self.seeders,
            leechers: self.leechers,
            downloads: self.downloads,
            size: self.size,
        }
    }
}

fn size_parse(input: &str) -> Option<u64> {
    let input_prefix = input.splitn(2, ' ').next()?;
    let input_prefix = input_prefix.parse::<f64>().ok()?;
    let out = if input.ends_with(" GiB") || input.ends_with(" GB") {
        Some(input_prefix * 1024.0 * 1024.0 * 1024.0)
    } else if input.ends_with(" MiB") || input.ends_with(" MB") {
        Some(input_prefix * 1024.0 * 1024.0)
    } else if input.ends_with(" KiB") || input.ends_with(" KB") {
        Some(input_prefix * 1024.0)
    } else if input.ends_with(" B") {
        Some(input_prefix)
    } else {
        None
    };

    out.map(|x| x as u64)
}

impl NyaaClient {
    pub fn new(config: NyaaConfig) -> Self {
        Self {
            client: reqwest::ClientBuilder::default()
                .build()
                .expect("failed to make client"),
            config,
        }
    }

    pub async fn query(&self, query: &str) -> Result<Vec<NyaaResult>> {
        let response = self
            .client
            .get(format!("{}{}", self.config.url, urlencoding::encode(query)))
            .send()
            .await?;
        if !response.status().is_success() {
            bail!("bad http status code for nyaa: {}", response.status());
        }
        let body = response.text().await?;
        let rss = Channel::read_from(BufReader::new(body.as_bytes()))?;
        let mut out = vec![];
        for mut item in rss.into_items() {
            let mut nyaa = item
                .extensions
                .remove("nyaa")
                .ok_or_else(|| anyhow!("missing nyaa ext"))?;
            out.push(NyaaResult {
                title: item.title.unwrap_or_default(),
                torrent_link: item.link.unwrap_or_default(),
                view_link: item.guid.map(|x| x.value).unwrap_or_default(),
                date: item
                    .pub_date
                    .and_then(|x| DateTime::parse_from_str(&*x, "%a, %d %b %Y %H:%M:%S %z").ok())
                    .ok_or_else(|| anyhow!("no date"))?,
                seeders: nyaa
                    .remove("seeders")
                    .and_then(|x| x.into_iter().next())
                    .and_then(|x| x.value)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or_default(),
                leechers: nyaa
                    .remove("leechers")
                    .and_then(|x| x.into_iter().next())
                    .and_then(|x| x.value)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or_default(),
                downloads: nyaa
                    .remove("downloads")
                    .and_then(|x| x.into_iter().next())
                    .and_then(|x| x.value)
                    .and_then(|x| x.parse().ok())
                    .unwrap_or_default(),
                category: nyaa
                    .remove("category")
                    .and_then(|x| x.into_iter().next())
                    .and_then(|x| x.value)
                    .unwrap_or_default(),
                size: size_parse(
                    &*nyaa
                        .remove("size")
                        .and_then(|x| x.into_iter().next())
                        .and_then(|x| x.value)
                        .unwrap_or_default(),
                )
                .unwrap_or_default(),
                trusted: nyaa
                    .remove("category")
                    .and_then(|x| x.into_iter().next())
                    .and_then(|x| x.value)
                    .as_deref()
                    == Some("Yes"),
            })
        }
        Ok(out)
    }
}

#[async_trait::async_trait]
impl Source for NyaaClient {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        Ok(self
            .query(query)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
