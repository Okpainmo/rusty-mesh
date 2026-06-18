use crate::common::{TEST_MESH_TOKEN, response_json, test_app};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn registry_routes_reject_missing_mesh_token() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/mesh/services")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(response).await;
    assert_eq!(body["response"], Value::Null);
    assert_eq!(body["error"]["code"], "MESH_TOKEN_REQUIRED");
}

#[tokio::test]
async fn registry_routes_reject_invalid_mesh_token() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/mesh/services")
                .header("authorization", "Bearer wrong-token")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(response).await;
    assert_eq!(body["response"], Value::Null);
    assert_eq!(body["error"]["code"], "INVALID_MESH_TOKEN");
}

#[tokio::test]
async fn registry_routes_accept_x_mesh_token_header() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services")
                .header("x-mesh-token", TEST_MESH_TOKEN)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "service_name": "orders",
                        "service_version": "1.2.3",
                        "service_port": 3000
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_route_does_not_require_mesh_token() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/mesh/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
}
