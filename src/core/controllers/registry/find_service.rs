use crate::AppState;
use crate::core::services::registry::registry_store::RegistryError;
use crate::core::structs::registry_response::ApiResponse;
use crate::core::structs::service_instance::ServiceInstance;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

pub async fn find_service(
    State(state): State<AppState>,
    Path((service_name, service_version, service_port)): Path<(String, String, u16)>,
) -> impl IntoResponse {
    match state
        .registry
        .find(&service_name, &service_version, service_port)
        .await
    {
        Ok(Some(service)) => (
            StatusCode::OK,
            Json(ApiResponse::success("Service found successfully", service)),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceInstance>::error(
                "No matching service found.",
                "SERVICE_NOT_FOUND",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersionRequirement(requirement)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceInstance>::error(
                format!("Invalid service version requirement '{}'.", requirement),
                "INVALID_SERVICE_VERSION_REQUIREMENT",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersion(version)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceInstance>::error(
                format!("Invalid service version '{}'.", version),
                "INVALID_SERVICE_VERSION",
            )),
        )
            .into_response(),
    }
}

pub async fn find_balanced_service(
    State(state): State<AppState>,
    Path((service_name, service_version)): Path<(String, String)>,
) -> impl IntoResponse {
    match state
        .registry
        .find_balanced(&service_name, &service_version)
        .await
    {
        Ok(Some(service)) => (
            StatusCode::OK,
            Json(ApiResponse::success("Service found successfully", service)),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceInstance>::error(
                "No matching service found.",
                "SERVICE_NOT_FOUND",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersionRequirement(requirement)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceInstance>::error(
                format!("Invalid service version requirement '{}'.", requirement),
                "INVALID_SERVICE_VERSION_REQUIREMENT",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersion(version)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceInstance>::error(
                format!("Invalid service version '{}'.", version),
                "INVALID_SERVICE_VERSION",
            )),
        )
            .into_response(),
    }
}
