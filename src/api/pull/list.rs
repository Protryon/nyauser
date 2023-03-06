use axum::extract::State;

use super::*;

pub(super) async fn list(
    _: Auth,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PullEntryNamed>>> {
    state
        .database
        .list_pull_entry_downloading()
        .map_err(ApiError::Other)
        .map(|x| {
            x.into_iter()
                .map(|pull_entry| PullEntryNamed {
                    id: pull_entry.key(),
                    pull_entry,
                })
                .collect()
        })
        .map(Json)
}
