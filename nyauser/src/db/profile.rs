use nyauser_types::Profile;

use anyhow::Result;

use super::Database;

impl Database {
    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        self.db.insert(
            format!("profile-{}", profile.name),
            serde_json::to_string(profile)?.as_bytes(),
        )?;
        Ok(())
    }

    pub fn delete_profile(&self, name: &str) -> Result<()> {
        self.db.remove(&format!("profile-{name}"))?;
        Ok(())
    }

    pub fn get_profile(&self, name: &str) -> Result<Option<Profile>> {
        self.get_serde("profile", name)
    }

    pub fn list_profile(&self) -> Result<Vec<Profile>> {
        self.list_serde("profile-")
    }
}
