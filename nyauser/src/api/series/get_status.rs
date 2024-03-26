use axum::extract::{Path, State};

use super::*;

pub(super) async fn get_status(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<SeriesStatus>> {
    let Some(series) = state.database.get_series(&name).map_err(ApiError::Other)? else {
        return Err(ApiError::NotFound);
    };
    Ok(Json(
        state
            .database
            .series_status(series)
            .map_err(ApiError::Other)?,
    ))
}
