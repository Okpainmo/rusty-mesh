use crate::common::{TEST_MESH_TOKEN, response_json, test_app};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn heartbeat_refreshes_registered_service() {
    let app = test_app();
    let body = json!({
        "service_name": "orders",
        "service_version": "1.2.3",
        "service_port": 3000
    })
    .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services")
                .header("authorization", format!("Bearer {}", TEST_MESH_TOKEN))
                .header("x-forwarded-for", "10.0.0.20")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services/heartbeat")
                .header("authorization", format!("Bearer {}", TEST_MESH_TOKEN))
                .header("x-forwarded-for", "10.0.0.20")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["response_message"],
        "Service heartbeat refreshed successfully"
    );
    assert_eq!(body["response"]["service_name"], "orders");
    assert_eq!(body["response"]["service_version"], "1.2.3");
    assert_eq!(body["response"]["ip"], "10.0.0.20");
    assert_eq!(body["response"]["port"], 3000);
    assert_eq!(body["response"]["internal_ip"], "10.0.0.20");
    assert_eq!(body["response"]["internal_port"], 3000);
    let response = body["response"]
        .as_object()
        .expect("response should be an object");
    assert!(!response.contains_key("service_ip"));
    assert!(!response.contains_key("service_port"));
    assert_eq!(body["error"], Value::Null);
}

#[tokio::test]
async fn heartbeat_response_uses_registered_external_endpoint() {
    let app = test_app();
    let registration_body = json!({
        "service_name": "orders",
        "service_version": "1.2.3",
        "service_ip": "orders-1",
        "service_port": 30304,
        "external_host": "127.0.0.1",
        "external_port": 32770,
        "external_scheme": "http"
    })
    .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services")
                .header("authorization", format!("Bearer {}", TEST_MESH_TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(registration_body))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services/heartbeat")
                .header("authorization", format!("Bearer {}", TEST_MESH_TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "service_name": "orders",
                        "service_version": "1.2.3",
                        "service_ip": "orders-1",
                        "service_port": 30304
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["response"]["ip"], "127.0.0.1");
    assert_eq!(body["response"]["port"], 32770);
    assert_eq!(body["response"]["internal_ip"], "orders-1");
    assert_eq!(body["response"]["internal_port"], 30304);
    assert_eq!(body["response"]["url"], "http://127.0.0.1:32770");
    let response = body["response"]
        .as_object()
        .expect("response should be an object");
    assert!(!response.contains_key("service_ip"));
    assert!(!response.contains_key("service_port"));
    assert_eq!(body["error"], Value::Null);
}

#[tokio::test]
async fn heartbeat_rejects_unknown_service() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services/heartbeat")
                .header("authorization", format!("Bearer {}", TEST_MESH_TOKEN))
                .header("x-forwarded-for", "10.0.0.20")
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

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(
        body["response_message"],
        "Service instance is not registered."
    );
    assert_eq!(body["response"], Value::Null);
    assert_eq!(
        body["error"]["message"],
        "Service instance is not registered."
    );
    assert_eq!(body["error"]["code"], "SERVICE_NOT_REGISTERED");
}
