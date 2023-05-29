use super::*;

mod delete;
mod get;
mod list;
mod update;

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(list::list))
        .route("/:name", routing::get(get::get))
        .route("/:name", routing::post(update::update))
        .route("/:name", routing::delete(delete::delete))
}
