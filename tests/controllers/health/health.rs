use crate::common::{response_json, test_app};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn root_returns_welcome_message() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["response_message"], "Welcome to Rusty Mesh");
    assert_eq!(body["response"]["status"], "ok");
    assert_eq!(body["response"]["service"], "mesh_service");
    assert_eq!(body["response"]["health_url"], "/api/v1/mesh/health");
    assert_eq!(body["response"]["registry_url"], "/api/v1/mesh/services");
    assert_eq!(body["error"], Value::Null);
}

#[tokio::test]
async fn mesh_base_returns_welcome_message() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/mesh")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["response_message"], "Welcome to Rusty Mesh");
    assert_eq!(body["response"]["status"], "ok");
    assert_eq!(body["response"]["service"], "mesh_service");
    assert_eq!(body["error"], Value::Null);
}

#[tokio::test]
async fn health_returns_service_status() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/mesh/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["response_message"], "Mesh service is healthy");
    assert_eq!(body["response"]["status"], "ok");
    assert_eq!(body["response"]["service"], "mesh_service");
    assert_eq!(
        body["response"]["registry_policy"]["heartbeat_interval_secs"],
        5
    );
    assert_eq!(body["response"]["registry_policy"]["service_ttl_secs"], 15);
    assert_eq!(body["error"], Value::Null);
}
