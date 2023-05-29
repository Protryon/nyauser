use serde::{Deserialize, Serialize};
use transmission_rpc::{
    types::{BasicAuth, RpcResponse, TorrentAddArgs, TorrentAddedOrDuplicate, TorrentGetField},
    *,
};

use crate::sink::Sink;
use anyhow::Result;

use super::{FinishedTorrent, TorrentInfo, TorrentStatus};

#[derive(Clone, Serialize, Deserialize)]
pub struct TransmissionConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

pub struct TransmissionClient {
    client: TransClient,
}

impl TransmissionClient {
    pub fn new(config: TransmissionConfig) -> Self {
        Self {
            client: TransClient::with_auth(
                &config.url,
                BasicAuth {
                    user: config.username,
                    password: config.password,
                },
            ),
        }
    }
}

impl TorrentInfo {
    fn from(torrent: &RpcResponse<TorrentAddedOrDuplicate>) -> Option<Self> {
        match &torrent.arguments {
            TorrentAddedOrDuplicate::TorrentAdded(torrent) => Some(Self {
                id: torrent.id?,
                hash: torrent.hash_string.clone()?,
                status: TorrentStatus::InProgress,
            }),
            TorrentAddedOrDuplicate::TorrentDuplicate(_) => None,
        }
    }
}

#[async_trait::async_trait]
impl Sink for TransmissionClient {
    async fn push(&mut self, torrent_url: &str) -> Result<Option<TorrentInfo>> {
        let pushed = self
            .client
            .torrent_add(TorrentAddArgs {
                filename: Some(torrent_url.to_string()),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("failed to add torrent: {:?}", e))?;
        if let Some(info) = TorrentInfo::from(&pushed) {
            return Ok(Some(info));
        }
        if pushed.result == "success" {
            return Ok(None);
        }
        Err(anyhow!("failed to get torrent id: {:?}", pushed))
    }

    async fn check(&mut self, id: i64) -> Result<Option<TorrentInfo>> {
        let torrent = self
            .client
            .torrent_get(
                Some(vec![
                    TorrentGetField::Id,
                    TorrentGetField::IsFinished,
                    TorrentGetField::PercentDone,
                    TorrentGetField::HashString,
                ]),
                Some(vec![types::Id::Id(id)]),
            )
            .await
            .map_err(|e| anyhow!("failed to get torrent: {:?}", e))?;
        let torrent = torrent.arguments.torrents.into_iter().next();
        Ok(match torrent {
            None => None,
            Some(torrent) => Some(TorrentInfo {
                id,
                hash: torrent.hash_string.unwrap_or_default(),
                status: if torrent.is_finished.unwrap_or(false) || torrent.percent_done == Some(1.0)
                {
                    TorrentStatus::Finished
                } else {
                    TorrentStatus::InProgress
                },
            }),
        })
    }

    async fn finished(&mut self) -> Result<Vec<FinishedTorrent>> {
        let torrents = self
            .client
            .torrent_get(
                Some(vec![
                    TorrentGetField::Id,
                    TorrentGetField::IsFinished,
                    TorrentGetField::PercentDone,
                    TorrentGetField::DownloadDir,
                    TorrentGetField::Files,
                ]),
                None,
            )
            .await
            .map_err(|e| anyhow!("failed to get torrent: {:?}", e))?;

        Ok(torrents
            .arguments
            .torrents
            .into_iter()
            .filter(|x| x.is_finished == Some(true) || x.percent_done == Some(1.0))
            .filter_map(|x| {
                Some(FinishedTorrent {
                    id: x.id?,
                    download_dir: x.download_dir?,
                    files: x.files?.into_iter().map(|x| x.name).collect(),
                })
            })
            .collect())
    }

    async fn delete(&mut self, id: i64) -> Result<()> {
        self.client
            .torrent_remove(vec![types::Id::Id(id)], true)
            .await
            .map_err(|e| anyhow!("failed to delete torrent: {:?}", e))?;
        Ok(())
    }
}
