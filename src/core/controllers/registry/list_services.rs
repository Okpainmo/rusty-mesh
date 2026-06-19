use crate::AppState;
use crate::core::structs::registry_response::{ApiResponse, ServicesResponse};
use axum::{Json, extract::State};

pub async fn list_services(State(state): State<AppState>) -> Json<ApiResponse<ServicesResponse>> {
    let services = state
        .registry
        .list()
        .await
        .into_iter()
        .map(|service| service.external_response())
        .collect::<Vec<_>>();
    let services_count = services.len();

    Json(ApiResponse::success(
        "Services listed successfully",
        ServicesResponse {
            services_count,
            services,
        },
    ))
}
