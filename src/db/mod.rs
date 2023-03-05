use anyhow::Result;
use sled::Db;

mod pull_entry;
pub use pull_entry::*;

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
}
