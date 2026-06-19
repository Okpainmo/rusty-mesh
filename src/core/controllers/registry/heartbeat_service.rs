use crate::AppState;
use crate::core::controllers::registry::register_service::{
    get_service_ip, service_registration_response,
};
use crate::core::services::registry::registry_store::RegistryError;
use crate::core::structs::registry_response::{ApiResponse, ServiceRegistrationResponse};
use crate::core::structs::service_instance::{ExternalServiceEndpoint, ServiceInstance};
use crate::core::structs::service_registration_request::ServiceRegistrationRequest;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn heartbeat_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<ServiceRegistrationRequest>, JsonRejection>,
) -> impl IntoResponse {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<ServiceRegistrationResponse>::error(
                    format!("Invalid service heartbeat request body: {}", error),
                    "INVALID_REQUEST_BODY",
                )),
            )
                .into_response();
        }
    };
    let service_ip = payload
        .service_ip
        .clone()
        .unwrap_or_else(|| get_service_ip(&headers));
    let service_name = payload.service_name;
    let service_version = payload.service_version;
    let service_port = payload.service_port;

    match state
        .registry
        .heartbeat(service_name, service_version, service_ip, service_port)
        .await
    {
        Ok(service) => {
            let heartbeat_response = heartbeat_response(&service);

            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    "Service heartbeat refreshed successfully",
                    heartbeat_response,
                )),
            )
                .into_response()
        }
        Err(RegistryError::ServiceNotRegistered) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceRegistrationResponse>::error(
                "Service instance is not registered.",
                "SERVICE_NOT_REGISTERED",
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

fn heartbeat_response(service: &ServiceInstance) -> ServiceRegistrationResponse {
    let external_endpoint =
        service
            .external_ip
            .as_ref()
            .zip(service.external_port)
            .map(|(ip, port)| ExternalServiceEndpoint {
                ip: ip.clone(),
                port,
                scheme: service
                    .external_scheme
                    .clone()
                    .unwrap_or_else(|| "http".to_string()),
            });

    service_registration_response(
        &service.name,
        &service.version,
        &service.ip,
        service.port,
        external_endpoint.as_ref(),
    )
}
