use crate::AppState;
use crate::core::structs::registry_response::ApiResponse;
use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

const MESH_TOKEN_HEADER: &str = "x-mesh-token";

pub async fn mesh_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.security.require_mesh_token {
        return next.run(request).await;
    }

    let expected_token = state
        .config
        .security
        .mesh_token
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();

    if expected_token.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "Mesh token is not configured.",
                "MESH_TOKEN_NOT_CONFIGURED",
            )),
        )
            .into_response();
    }

    match request_mesh_token(&request) {
        Some(token) if constant_time_eq(token.as_bytes(), expected_token.as_bytes()) => {
            next.run(request).await
        }
        Some(_) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error(
                "Invalid mesh authentication token.",
                "INVALID_MESH_TOKEN",
            )),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error(
                "Mesh authentication token is required.",
                "MESH_TOKEN_REQUIRED",
            )),
        )
            .into_response(),
    }
}

fn request_mesh_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer_token)
        .or_else(|| {
            request
                .headers()
                .get(MESH_TOKEN_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

fn bearer_token(value: &str) -> Option<&str> {
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();

    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        diff |= usize::from(left_byte ^ right_byte);
    }

    diff == 0
}
