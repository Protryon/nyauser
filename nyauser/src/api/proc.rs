use axum::extract::State;

use crate::search::wipe_nonexistant;

use super::*;

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/scan", routing::get(scan))
        .route("/search", routing::get(search))
        .route("/wipe_deleted", routing::get(wipe_deleted))
}

async fn scan(_auth: Auth, State(state): State<AppState>) {
    state.scan.notify_one();
}

async fn search(_auth: Auth, State(state): State<AppState>) {
    state.search.notify_one();
}

async fn wipe_deleted(_auth: Auth, State(state): State<AppState>) -> ApiResult<()> {
    wipe_nonexistant(&state.database).map_err(ApiError::Other)
}
