//! # Rusty Mesh Server Binary
//!
//! Entry point for the mesh service registry.

use mesh::core::services::registry::registry_store::RegistryStore;
use mesh::utils::load_config::load_config;
use mesh::utils::load_env::load_env;
use mesh::{AppState, create_app};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::fmt::time::SystemTime;

/// Initializes the global tracing subscriber with JSON formatting.
fn initialize_logging() {
    tracing_subscriber::fmt()
        .json()
        .with_timer(SystemTime)
        .with_level(true)
        .init();
}

#[tokio::main]
async fn main() {
    load_env();
    initialize_logging();

    let app_config = load_config();

    let clean_config = match app_config {
        Ok(config) => {
            if let Err(e) = config.validate() {
                error!(
                    "SERVER START-UP ERROR: FAILED TO LOAD SERVER CONFIGURATIONS, {}",
                    e
                );
                std::process::exit(1);
            }

            config
        }
        Err(e) => {
            error!(
                "SERVER START-UP ERROR: FAILED TO LOAD SERVER CONFIGURATIONS, {}",
                e
            );
            std::process::exit(1);
        }
    };

    let registry_config = clean_config.registry.clone();
    let state = AppState {
        config: Arc::new(clean_config),
        registry: RegistryStore::new(registry_config.service_ttl_secs),
    };

    let app = create_app(state.clone());

    let host = state.config.server.host.as_str();
    let port = state.config.server.port;

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid server address");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            print!(
                "
                .................................................
                Service: {}
                Environment: {}
                Registry heartbeat interval: {}s
                Registry TTL: {}s
                .................................................

                Server running on http://{}
                ",
                state.config.app.name,
                state.config.app.environment.as_deref().unwrap_or("unknown"),
                state.config.registry.heartbeat_interval_secs,
                state.config.registry.service_ttl_secs,
                addr
            );
            listener
        }
        Err(e) => {
            error!("SERVER INITIALIZATION ERROR: {}!", e);
            std::process::exit(1);
        }
    };

    let server_result = axum::serve(listener, app.into_make_service()).await;

    match server_result {
        Ok(_) => {
            info!("Graceful server shutdown!");
        }
        Err(e) => {
            error!("SERVER SHUTDOWN ERROR: {}!", e);
        }
    }
}
