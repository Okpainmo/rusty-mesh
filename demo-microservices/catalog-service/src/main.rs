use anyhow::Result;
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::net::SocketAddr;
use std::time::Duration;

mod registry_client;

use registry_client::{EndpointDetails, MeshRegistryClient};

#[derive(Clone)]
struct ServiceConfig {
    service_name: String,
    service_version: String,
    service_bind_host: String,
    service_advertise_host: String,
    service_port: u16,
    mesh_url: String,
    mesh_token: Option<String>,
    heartbeat_interval_secs: u64,
    external_host: Option<String>,
    external_port: Option<u16>,
    external_scheme: String,
}

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    registry: MeshRegistryClient,
    endpoint: EndpointDetails,
}

#[derive(Serialize)]
struct HealthResponse {
    service: String,
    version: String,
    status: String,
    #[serde(flatten)]
    endpoint: EndpointDetails,
}

#[derive(Serialize)]
struct WelcomeResponse {
    service: String,
    version: String,
    status: String,
    message: String,
    health_url: String,
    feedback_url: String,
    #[serde(flatten)]
    endpoint: EndpointDetails,
}

#[derive(Serialize)]
struct FeedbackResponse {
    service: String,
    message: String,
    #[serde(flatten)]
    endpoint: EndpointDetails,
    data: CatalogFeedbackData,
}

#[derive(Serialize)]
struct CatalogFeedbackData {
    featured_sku: String,
    stock: u16,
    available: bool,
}

#[derive(Serialize)]
struct CallPeerResponse {
    service: String,
    called_service: String,
    #[serde(flatten)]
    endpoint: EndpointDetails,
    peer_response: Value,
}

#[derive(Serialize)]
struct ErrorResponse {
    service: String,
    error: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config("catalog-service");
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        config.service_bind_host, config.service_port
    ))
    .await?;
    let port = listener.local_addr()?.port();

    let registry = MeshRegistryClient::new(
        config.mesh_url.clone(),
        config.mesh_token.clone(),
        config.service_advertise_host.clone(),
        config.service_name.clone(),
        config.service_version.clone(),
        port,
        env::var("HOSTNAME").ok(),
        config.external_host.clone(),
        config.external_port,
        config.external_scheme.clone(),
    );
    let endpoint = register_until_ready(&registry, &config.service_name).await;
    let heartbeat = registry.start_heartbeat(config.heartbeat_interval_secs);

    let state = AppState {
        config: config.clone(),
        registry: registry.clone(),
        endpoint,
    };
    let app = Router::new()
        .route("/", get(welcome))
        .route("/health", get(health))
        .route("/get-catalog-feedback", get(feedback))
        .route("/call-cart-service", get(call_peer))
        .with_state(state);

    println!(
        "{}:{} listening on http://{}:{}",
        config.service_name, config.service_version, config.service_bind_host, port
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    heartbeat.abort();
    if let Err(error) = registry.unregister().await {
        eprintln!("failed to unregister {}: {error:#}", config.service_name);
    }

    Ok(())
}

async fn welcome(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<WelcomeResponse> {
    let service_name = state.config.service_name;

    Json(WelcomeResponse {
        service: service_name.clone(),
        version: state.config.service_version,
        status: "ok".to_string(),
        message: format!("{service_name} is running and registered with Rusty Mesh."),
        health_url: "/health".to_string(),
        feedback_url: "/get-catalog-feedback".to_string(),
        endpoint: state.endpoint,
    })
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: state.config.service_name,
        version: state.config.service_version,
        status: "ok".to_string(),
        endpoint: state.endpoint,
    })
}

async fn feedback(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<FeedbackResponse> {
    Json(FeedbackResponse {
        service: state.config.service_name,
        message: "Catalog service says the featured item is available".to_string(),
        endpoint: state.endpoint,
        data: CatalogFeedbackData {
            featured_sku: "sku-1001".to_string(),
            stock: 42,
            available: true,
        },
    })
}

async fn call_peer(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let called_service = "cart-service";

    match state.registry.discover(called_service).await {
        Ok(peer) => match state
            .registry
            .call_feedback(&peer, "/get-cart-feedback")
            .await
        {
            Ok(peer_response) => (
                StatusCode::OK,
                Json(CallPeerResponse {
                    service: state.config.service_name,
                    called_service: called_service.to_string(),
                    endpoint: state.endpoint,
                    peer_response,
                }),
            )
                .into_response(),
            Err(error) => (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    service: state.config.service_name,
                    error: error.to_string(),
                }),
            )
                .into_response(),
        },
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                service: state.config.service_name,
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn register_until_ready(
    registry: &MeshRegistryClient,
    service_name: &str,
) -> EndpointDetails {
    loop {
        match registry.register().await {
            Ok(endpoint) => break endpoint,
            Err(error) => {
                eprintln!("{service_name} initial registration failed: {error:#}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

fn load_config(default_name: &str) -> ServiceConfig {
    let service_bind_host =
        env::var("SERVICE_BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let service_advertise_host =
        env::var("SERVICE_ADVERTISE_HOST").unwrap_or_else(|_| service_bind_host.clone());

    ServiceConfig {
        service_name: env::var("SERVICE_NAME").unwrap_or_else(|_| default_name.to_string()),
        service_version: env::var("SERVICE_VERSION").unwrap_or_else(|_| "1.0.0".to_string()),
        service_bind_host,
        service_advertise_host,
        service_port: env::var("SERVICE_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        mesh_url: env::var("MESH_URL").unwrap_or_else(|_| "http://127.0.0.1:3080".to_string()),
        mesh_token: env::var("MESH_TOKEN")
            .ok()
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty()),
        heartbeat_interval_secs: env::var("HEARTBEAT_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(5),
        external_host: env::var("SERVICE_EXTERNAL_HOST")
            .ok()
            .map(|host| host.trim().to_string())
            .filter(|host| !host.is_empty()),
        external_port: env::var("SERVICE_EXTERNAL_PORT")
            .ok()
            .and_then(|value| value.parse().ok()),
        external_scheme: env::var("SERVICE_EXTERNAL_SCHEME").unwrap_or_else(|_| "http".to_string()),
    }
}
