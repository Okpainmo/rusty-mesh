use axum::{body::to_bytes, response::Response};
use mesh::core::services::registry::registry_store::RegistryStore;
use mesh::utils::load_config::{
    AppConfig, AppSection, ClientIntegrationsSection, RegistrySection, SecuritySection,
    ServerSection,
};
use mesh::{AppState, create_app};
use serde_json::Value;
use std::sync::Arc;

pub const TEST_MESH_TOKEN: &str = "test-mesh-token";

pub fn test_state() -> AppState {
    AppState {
        config: Arc::new(AppConfig {
            app: AppSection {
                name: "mesh_service".to_string(),
                environment: Some("test".to_string()),
            },
            client_integrations: ClientIntegrationsSection {
                allow_logging_middleware: true,
                allow_request_timeout_middleware: true,
            },
            server: ServerSection {
                host: "127.0.0.1".to_string(),
                port: 3080,
                request_timeout_secs: 60,
            },
            registry: RegistrySection {
                heartbeat_interval_secs: 5,
                service_ttl_secs: 15,
            },
            security: SecuritySection {
                require_mesh_token: true,
                mesh_token: Some(TEST_MESH_TOKEN.to_string()),
            },
        }),
        registry: RegistryStore::new(15),
    }
}

pub fn test_app() -> axum::Router {
    create_app(test_state())
}

pub async fn response_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");

    serde_json::from_slice(&body).expect("response body should be json")
}
