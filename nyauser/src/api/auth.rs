use axum::extract::FromRequestParts;
use axum_auth::AuthBasic;
use http::request::Parts;

use crate::config::CONFIG;

use super::*;

pub struct Auth {
    _p: (),
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> ApiResult<Self> {
        let AuthBasic((username, Some(password))) = AuthBasic::from_request_parts(req, state)
            .await
            .map_err(|e| ApiError::Arbitrary(e.into_response()))?
        else {
            return Err(ApiError::Unauthorized(format!("missing password")));
        };
        if username != CONFIG.rpc_username || password != CONFIG.rpc_password {
            return Err(ApiError::Unauthorized(format!("invalid credentials")));
        }
        Ok(Self { _p: () })
    }
}
