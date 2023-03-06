use axum::extract::{Path, State};

use super::*;

pub(super) async fn delete(
    _: Auth,
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    state
        .database
        .get_pull_entry(&key)
        .map_err(ApiError::Other)?
        .ok_or(ApiError::NotFound)?
        .delete(&state.database)
        .map_err(ApiError::Other)?;
    Ok(())
}
