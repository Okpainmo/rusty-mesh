use anyhow::Result;
use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::env;
use std::net::SocketAddr;
use std::time::Duration;

mod registry_client;

use registry_client::MeshRegistryClient;

#[derive(Clone)]
struct ServiceConfig {
    service_name: String,
    service_version: String,
    service_bind_host: String,
    service_advertise_host: String,
    mesh_url: String,
    heartbeat_interval_secs: u64,
}

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    port: u16,
}

#[derive(Serialize)]
struct HealthResponse {
    service: String,
    version: String,
    status: String,
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config("user-service");
    let listener = tokio::net::TcpListener::bind(format!("{}:0", config.service_bind_host)).await?;
    let port = listener.local_addr()?.port();

    let registry = MeshRegistryClient::new(
        config.mesh_url.clone(),
        config.service_advertise_host.clone(),
        config.service_name.clone(),
        config.service_version.clone(),
        port,
    );
    register_until_ready(&registry, &config.service_name).await;
    let heartbeat = registry.start_heartbeat(config.heartbeat_interval_secs);

    let state = AppState {
        config: config.clone(),
        port,
    };
    let app = Router::new()
        .route("/health", get(health))
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

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: state.config.service_name,
        version: state.config.service_version,
        status: "ok".to_string(),
        port: state.port,
    })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn register_until_ready(registry: &MeshRegistryClient, service_name: &str) {
    loop {
        match registry.register().await {
            Ok(_) => break,
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
        mesh_url: env::var("MESH_URL").unwrap_or_else(|_| "http://127.0.0.1:3080".to_string()),
        heartbeat_interval_secs: env::var("HEARTBEAT_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(5),
    }
}
