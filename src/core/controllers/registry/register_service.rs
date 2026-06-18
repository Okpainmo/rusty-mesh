use crate::AppState;
use crate::core::services::registry::registry_store::RegistryError;
use crate::core::structs::registry_response::{ApiResponse, ServiceRegistrationResponse};
use crate::core::structs::service_registration_request::ServiceRegistrationRequest;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn register_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<ServiceRegistrationRequest>, JsonRejection>,
) -> impl IntoResponse {
    let service_ip = get_service_ip(&headers);
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<ServiceRegistrationResponse>::error(
                    format!("Invalid service registration request body: {}", error),
                    "INVALID_REQUEST_BODY",
                )),
            )
                .into_response();
        }
    };
    let service_name = payload.service_name;
    let service_version = payload.service_version;
    let service_port = payload.service_port;

    let registration_response = ServiceRegistrationResponse {
        service_name: service_name.clone(),
        service_version: service_version.clone(),
        service_ip: service_ip.clone(),
        service_port,
    };

    match state
        .registry
        .register(service_name, service_version, service_ip, service_port)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(
                "Service registered successfully",
                registration_response,
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersion(version)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceRegistrationResponse>::error(
                format!("Invalid service version '{}'.", version),
                "INVALID_SERVICE_VERSION",
            )),
        )
            .into_response(),
        Err(RegistryError::InvalidVersionRequirement(requirement)) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<ServiceRegistrationResponse>::error(
                format!("Invalid service version requirement '{}'.", requirement),
                "INVALID_SERVICE_VERSION_REQUIREMENT",
            )),
        )
            .into_response(),
    }
}

pub(crate) fn get_service_ip(headers: &HeaderMap) -> String {
    let candidate = headers
        .get("x-mesh-advertise-host")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("127.0.0.1");

    normalize_loopback_ip(candidate)
}

fn normalize_loopback_ip(ip: &str) -> String {
    match ip {
        "::1" | "::ffff:127.0.0.1" => "127.0.0.1".to_string(),
        _ => ip.to_string(),
    }
}
