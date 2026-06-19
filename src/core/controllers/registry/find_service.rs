use crate::AppState;
use crate::core::services::registry::registry_store::RegistryError;
use crate::core::structs::registry_response::ApiResponse;
use crate::core::structs::service_instance::{ServiceEndpointResponse, ServiceInstance};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn find_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((service_name, service_version, service_port)): Path<(String, String, u16)>,
) -> impl IntoResponse {
    let use_internal_endpoint = wants_internal_endpoint(&headers);

    match state
        .registry
        .find(&service_name, &service_version, service_port)
        .await
    {
        Ok(Some(service)) => service_found_response(service, use_internal_endpoint),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                "No matching service found.",
                "SERVICE_NOT_FOUND",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersionRequirement(requirement)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                format!("Invalid service version requirement '{}'.", requirement),
                "INVALID_SERVICE_VERSION_REQUIREMENT",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersion(version)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                format!("Invalid service version '{}'.", version),
                "INVALID_SERVICE_VERSION",
            )),
        )
            .into_response(),
        Err(RegistryError::ServiceNotRegistered) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                "Service instance is not registered.",
                "SERVICE_NOT_REGISTERED",
            )),
        )
            .into_response(),
    }
}

pub async fn find_balanced_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((service_name, service_version)): Path<(String, String)>,
) -> impl IntoResponse {
    let use_internal_endpoint = wants_internal_endpoint(&headers);

    match state
        .registry
        .find_balanced(&service_name, &service_version)
        .await
    {
        Ok(Some(service)) => service_found_response(service, use_internal_endpoint),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                "No matching service found.",
                "SERVICE_NOT_FOUND",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersionRequirement(requirement)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                format!("Invalid service version requirement '{}'.", requirement),
                "INVALID_SERVICE_VERSION_REQUIREMENT",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersion(version)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                format!("Invalid service version '{}'.", version),
                "INVALID_SERVICE_VERSION",
            )),
        )
            .into_response(),
        Err(RegistryError::ServiceNotRegistered) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceEndpointResponse>::error(
                "Service instance is not registered.",
                "SERVICE_NOT_REGISTERED",
            )),
        )
            .into_response(),
    }
}

fn service_found_response(
    service: ServiceInstance,
    use_internal_endpoint: bool,
) -> axum::response::Response {
    if use_internal_endpoint {
        return (
            StatusCode::OK,
            Json(ApiResponse::success("Service found successfully", service)),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            "Service found successfully",
            service.external_response(),
        )),
    )
        .into_response()
}

fn wants_internal_endpoint(headers: &HeaderMap) -> bool {
    headers
        .get("x-mesh-endpoint-scope")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("internal"))
        .unwrap_or(false)
}
