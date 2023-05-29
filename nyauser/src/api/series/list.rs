
use axum::extract::State;

use super::*;

pub(super) async fn list(
    _: Auth,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Series>>> {
    state.database.list_series().map_err(ApiError::Other).map(Json)
}
