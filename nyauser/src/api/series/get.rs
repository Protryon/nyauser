use axum::extract::{Path, State};

use super::*;

pub(super) async fn get(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<Series>> {
    state
        .database
        .get_series(&name)
        .map_err(ApiError::Other)?
        .map(Json)
        .ok_or(ApiError::NotFound)
}
