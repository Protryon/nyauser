use axum::extract::{Path, State};

use crate::db::Profile;

use super::*;

pub(super) async fn update(
    _: Auth,
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<Profile>,
) -> ApiResult<()> {
    if body.name != name {
        return Err(ApiError::BadRequest(format!(
            "path name did not match name in body '{}' != {}",
            name, body.name
        )));
    }
    body.save(&state.database).map_err(ApiError::Other)?;
    Ok(())
}
