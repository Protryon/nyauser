use nyauser_types::PullEntry;

use super::Database;
use anyhow::Result;

impl Database {
    /// includes an interior save
    pub fn clear_torrent_id(&self, pull: &mut PullEntry) -> Result<()> {
        if let Some(torrent_id) = pull.torrent_id.take() {
            self.db.remove(format!("downloading-{}", torrent_id))?;
        }
        self.save_pull(pull)?;
        Ok(())
    }

    pub fn save_pull(&self, pull: &PullEntry) -> Result<()> {
        let key = pull.key();
        self.db.insert(
            format!("torrent-{key}"),
            serde_json::to_string(pull)?.as_bytes(),
        )?;
        if let Some(torrent_id) = pull.torrent_id {
            self.db
                .insert(format!("downloading-{}", torrent_id), key.as_bytes())?;
        }
        Ok(())
    }

    pub fn delete_pull(&self, mut pull: PullEntry) -> Result<()> {
        if let Some(torrent_id) = pull.torrent_id.take() {
            self.db.remove(format!("downloading-{}", torrent_id))?;
        }
        self.db.remove(format!("torrent-{}", pull.key()))?;
        Ok(())
    }
}

impl Database {
    pub fn exists_pull_entry(&self, key: &str) -> Result<bool> {
        Ok(self.db.contains_key(format!("torrent-{key}"))?)
    }

    pub fn get_pull_entry(&self, key: &str) -> Result<Option<PullEntry>> {
        self.get_serde("torrent", key)
    }

    pub fn get_pull_entry_from_torrent_id(&self, id: i64) -> Result<Option<PullEntry>> {
        let downloading_id = format!("downloading-{}", id);
        let internal_id = match self.db.get(&downloading_id)? {
            None => return Ok(None),
            Some(x) => String::from_utf8(x.to_vec())?,
        };
        self.get_pull_entry(&internal_id)
    }

    pub fn list_pull_entry_series(&self, name: &str) -> Result<Vec<PullEntry>> {
        self.list_serde(&format!("torrent-{name}_S"))
    }

    pub fn list_pull_entry(&self) -> Result<Vec<PullEntry>> {
        self.list_serde("torrent-")
    }

    pub fn list_pull_entry_downloading(&self) -> Result<Vec<PullEntry>> {
        let mut out = vec![];
        for entry in self.db.scan_prefix("downloading-") {
            let (_, value) = entry?;
            let key = std::str::from_utf8(&value[..])?;
            let Some(pull_entry) = self.get_pull_entry(&key)? else {
                bail!("dangling `downloading` key");
            };
            out.push(pull_entry);
        }
        Ok(out)
    }
}
