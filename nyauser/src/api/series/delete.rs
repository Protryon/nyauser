use axum::extract::{Path, State};

use super::*;

pub(super) async fn delete(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    state
        .database
        .delete_series(&name)
        .map_err(ApiError::Other)?;
    Ok(())
}
