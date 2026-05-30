use crate::AppState;
use crate::core::structs::registry_response::ApiResponse;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;

pub async fn timeout_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let timeout_secs = state.config.server.request_timeout_secs;

    match tokio::time::timeout(Duration::from_secs(timeout_secs), next.run(request)).await {
        Ok(response) => response,
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            axum::Json(ApiResponse::<()>::error(
                "Request timed out",
                "REQUEST_TIMEOUT",
            )),
        )
            .into_response(),
    }
}
