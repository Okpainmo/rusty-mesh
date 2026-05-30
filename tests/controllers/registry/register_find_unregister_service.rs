use crate::common::{response_json, test_app};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn register_find_and_unregister_service() {
    let app = test_app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mesh/services")
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

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["response_message"], "Service registered successfully");
    assert_eq!(body["response"]["service_name"], "orders");
    assert_eq!(body["response"]["service_version"], "1.2.3");
    assert_eq!(body["response"]["service_ip"], "10.0.0.20");
    assert_eq!(body["response"]["service_port"], 3000);
    assert_eq!(body["error"], Value::Null);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/mesh/services/orders/%5E1.0.0/3000")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["response_message"], "Service found successfully");
    assert_eq!(body["response"]["name"], "orders");
    assert_eq!(body["response"]["version"], "1.2.3");
    assert_eq!(body["response"]["ip"], "10.0.0.20");
    assert_eq!(body["response"]["port"], 3000);
    assert_eq!(body["error"], Value::Null);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/mesh/services")
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["response_message"],
        "Service unregistered successfully"
    );
    assert_eq!(body["response"]["service_name"], "orders");
    assert_eq!(body["response"]["service_version"], "1.2.3");
    assert_eq!(body["response"]["service_ip"], "10.0.0.20");
    assert_eq!(body["response"]["service_port"], 3000);
    assert_eq!(body["error"], Value::Null);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/mesh/services/orders/%5E1.0.0/3000")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["response_message"], "No matching service found.");
    assert_eq!(body["response"], Value::Null);
    assert_eq!(body["error"]["message"], "No matching service found.");
    assert_eq!(body["error"]["code"], "SERVICE_NOT_FOUND");
}
