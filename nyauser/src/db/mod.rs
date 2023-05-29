use anyhow::Result;
use serde::de::DeserializeOwned;
use sled::Db;

mod pull_entry;
pub use pull_entry::*;

mod profile;
pub use profile::*;

mod series;
pub use series::*;

pub struct Database {
    db: Db,
}

impl Database {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn flush(&self) -> Result<()> {
        self.db.flush_async().await?;
        Ok(())
    }

    fn get_serde<T: DeserializeOwned>(&self, prefix: &str, key: &str) -> Result<Option<T>> {
        let Some(raw) = self.db.get(format!("{prefix}-{key}"))? else {
            return Ok(None);
        };
        let utf = String::from_utf8(raw.to_vec())?;
        Ok(Some(serde_json::from_str(&utf)?))
    }

    fn list_serde<T: DeserializeOwned>(&self, prefix: &str) -> Result<Vec<T>> {
        let mut out = vec![];
        for entry in self.db.scan_prefix(prefix) {
            let (_, value) = entry?;
            let value = std::str::from_utf8(&value[..])?;
            let pull_entry: T = serde_json::from_str(value)?;
            out.push(pull_entry);
        }
        Ok(out)
    }
}
