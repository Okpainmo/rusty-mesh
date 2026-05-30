use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tracing::info;

pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let started_at = Instant::now();

    let response = next.run(request).await;
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = status.as_u16(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "request completed"
    );

    response.into_response()
}
