use crate::AppState;
use crate::core::structs::registry_response::{
    ApiResponse, HealthResponse, RegistryPolicyResponse,
};
use axum::{Json, extract::State};

pub async fn health_check(State(state): State<AppState>) -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(
        "Mesh service is healthy",
        HealthResponse {
            status: "ok".to_string(),
            service: state.config.app.name.clone(),
            registry_policy: RegistryPolicyResponse {
                heartbeat_interval_secs: state.config.registry.heartbeat_interval_secs,
                service_ttl_secs: state.config.registry.service_ttl_secs,
            },
        },
    ))
}
