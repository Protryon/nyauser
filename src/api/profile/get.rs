use axum::extract::{Path, State};

use crate::db::Profile;

use super::*;

pub(super) async fn get(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<Option<Profile>>> {
    state
        .database
        .get_profile(&name)
        .map_err(ApiError::Other)
        .map(Json)
}
