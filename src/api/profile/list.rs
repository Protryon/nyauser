use axum::extract::State;

use crate::db::Profile;

use super::*;

pub(super) async fn list(_: Auth, State(state): State<AppState>) -> ApiResult<Json<Vec<Profile>>> {
    state
        .database
        .list_profile()
        .map_err(ApiError::Other)
        .map(Json)
}
