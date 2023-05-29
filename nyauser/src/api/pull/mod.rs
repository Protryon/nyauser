use super::*;

mod delete;
mod list;

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(list::list))
        .route("/:name", routing::delete(delete::delete))
}
