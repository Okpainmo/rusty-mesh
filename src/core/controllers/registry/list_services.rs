use crate::AppState;
use crate::core::structs::registry_response::{ApiResponse, ServicesResponse};
use axum::{Json, extract::State};

pub async fn list_services(State(state): State<AppState>) -> Json<ApiResponse<ServicesResponse>> {
    Json(ApiResponse::success(
        "Services listed successfully",
        ServicesResponse {
            services: state.registry.list().await,
        },
    ))
}
