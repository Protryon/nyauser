use core::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::response::Redirect;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    *,
};
use reqwest::Url;

use crate::api::auth::Auth;
use crate::db::Database;
use serde::{Deserialize, Serialize};

use crate::config::CONFIG;

use self::logger::LoggerLayer;
use anyhow::Result;

mod auth;

mod logger;
mod profile;
mod pull;
mod series;

#[derive(Serialize, Deserialize)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    Found(Url),
    Unauthorized(String),
    BadRequest(String),
    GeneralFailure(String),
    NotFound,
    Arbitrary(Response),
    Other(anyhow::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self::Other(anyhow::Error::from(error))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Found(destination) => {
                Redirect::temporary(destination.as_str()).into_response()
            }
            ApiError::Unauthorized(message) => {
                (StatusCode::UNAUTHORIZED, Json(ErrorBody { message })).into_response()
            }
            ApiError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, Json(ErrorBody { message })).into_response()
            }
            ApiError::GeneralFailure(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { message }),
            )
                .into_response(),
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    message: "not found".to_string(),
                }),
            )
                .into_response(),
            ApiError::Arbitrary(r) => r,
            ApiError::Other(e) => {
                error!("internal error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
}

async fn health() {}

fn route(state: AppState) -> Router {
    let api = Router::<AppState>::new()
        .nest("/series", series::route())
        .nest("/profile", profile::route())
        .nest("/pull", pull::route())
        .route("/health", routing::get(health))
        .with_state(state);

    Router::new().nest("/api/v1", api).layer(LoggerLayer)
}

pub fn spawn_api_server(state: AppState) {
    tokio::spawn(async move {
        async fn run(state: AppState) -> Result<()> {
            let server = axum::Server::bind(&CONFIG.bind);
            server
                .serve(route(state).into_make_service_with_connect_info::<SocketAddr>())
                .await?;
            Ok(())
        }
        loop {
            if let Err(e) = run(state.clone()).await {
                error!("failed to start api server: {:?}", e);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
