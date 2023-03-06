use axum::extract::{Path, State};

use crate::db::SeriesStatus;

use super::*;

pub(super) async fn get(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<SeriesStatus>> {
    let Some(series) = state.database.get_series(&name).map_err(ApiError::Other)? else {
        return Err(ApiError::NotFound);
    };
    Ok(Json(
        series.status(&state.database).map_err(ApiError::Other)?,
    ))
}
