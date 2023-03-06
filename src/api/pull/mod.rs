use crate::db::PullEntry;

use super::*;

mod delete;
mod list;

#[derive(Serialize, Deserialize)]
pub struct PullEntryNamed {
    pub id: String,
    #[serde(flatten)]
    pub pull_entry: PullEntry,
}

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(list::list))
        .route("/:name", routing::delete(delete::delete))
}
