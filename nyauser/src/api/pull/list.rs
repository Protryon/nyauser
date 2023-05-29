use axum::extract::{State, Query};

use super::*;

pub(super) async fn list(
    _: Auth,
    State(state): State<AppState>,
    Query(filter): Query<PullEntryFilter>,
) -> ApiResult<Json<Vec<PullEntryNamed>>> {
    let raw: Vec<PullEntryNamed> = state
        .database
        .list_pull_entry_downloading()
        .map_err(ApiError::Other)?
        .into_iter()
        .map(|pull_entry| PullEntryNamed {
            id: pull_entry.key(),
            pull_entry,
        })
        .filter(|entry| {
            if let Some(profile) = &filter.profile {
                if &entry.pull_entry.result.profile != profile {
                    return false;
                }
            }
            if let Some(title_contains) = &filter.title_contains {
                if !entry.pull_entry.result.parsed.title.contains(title_contains) {
                    return false;
                }
            }
            if let Some(title_is) = &filter.title_is {
                if &entry.pull_entry.result.parsed.title != title_is {
                    return false;
                }
            }
            if let Some(season_is) = filter.season_is {
                if entry.pull_entry.result.parsed.season != season_is {
                    return false;
                }
            }
            if let Some(episode_is) = &filter.episode_is {
                if &entry.pull_entry.result.parsed.episode != episode_is {
                    return false;
                }
            }
            if let Some(state) = filter.state {
                if entry.pull_entry.state != state {
                    return false;
                }
            }
            true
        })
        .collect();
    
    Ok(Json(raw))
}
