use crate::AppState;
use crate::core::services::registry::registry_store::RegistryError;
use crate::core::structs::registry_response::{ApiResponse, ServiceRegistrationResponse};
use crate::core::structs::service_instance::ExternalServiceEndpoint;
use crate::core::structs::service_registration_request::ServiceRegistrationRequest;
use crate::utils::docker_port_mapping::resolve_external_endpoint;
use crate::utils::load_config::ExternalEndpointResolution;
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
    let service_ip = payload
        .service_ip
        .clone()
        .unwrap_or_else(|| get_service_ip(&headers));
    let explicit_external_endpoint = match explicit_external_endpoint(&payload) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<ServiceRegistrationResponse>::error(
                    error,
                    "INVALID_EXTERNAL_ENDPOINT",
                )),
            )
                .into_response();
        }
    };
    let service_name = payload.service_name;
    let service_version = payload.service_version;
    let service_port = payload.service_port;
    let container_id = get_container_id(&headers);
    let external_endpoint = resolve_registration_external_endpoint(
        explicit_external_endpoint,
        container_id,
        service_port,
        &state.config.registry.external_endpoint_resolution,
        state.config.registry.public_host.clone(),
    )
    .await;

    let registration_response = service_registration_response(
        &service_name,
        &service_version,
        &service_ip,
        service_port,
        external_endpoint.as_ref(),
    );

    match state
        .registry
        .register(
            service_name,
            service_version,
            service_ip,
            service_port,
            external_endpoint,
        )
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
        Err(RegistryError::ServiceNotRegistered) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ServiceRegistrationResponse>::error(
                "Service instance is not registered.",
                "SERVICE_NOT_REGISTERED",
            )),
        )
            .into_response(),
    }
}

pub(crate) fn service_registration_response(
    service_name: &str,
    service_version: &str,
    service_ip: &str,
    service_port: u16,
    external_endpoint: Option<&ExternalServiceEndpoint>,
) -> ServiceRegistrationResponse {
    let ip = external_endpoint
        .map(|endpoint| endpoint.ip.clone())
        .unwrap_or_else(|| service_ip.to_string());
    let port = external_endpoint
        .map(|endpoint| endpoint.port)
        .unwrap_or(service_port);
    let scheme = external_endpoint
        .map(|endpoint| endpoint.scheme.as_str())
        .unwrap_or("http");

    ServiceRegistrationResponse {
        service_name: service_name.to_string(),
        service_version: service_version.to_string(),
        ip: ip.clone(),
        port,
        internal_ip: service_ip.to_string(),
        internal_port: service_port,
        url: format!("{}://{}:{}", scheme, ip, port),
    }
}

async fn resolve_registration_external_endpoint(
    explicit_external_endpoint: Option<ExternalServiceEndpoint>,
    container_id: Option<String>,
    service_port: u16,
    resolution: &ExternalEndpointResolution,
    public_host: Option<String>,
) -> Option<ExternalServiceEndpoint> {
    if explicit_external_endpoint.is_some() {
        return explicit_external_endpoint;
    }

    match resolution {
        ExternalEndpointResolution::None => None,
        ExternalEndpointResolution::Docker => {
            resolve_external_endpoint(container_id, service_port, public_host).await
        }
    }
}

fn explicit_external_endpoint(
    payload: &ServiceRegistrationRequest,
) -> Result<Option<ExternalServiceEndpoint>, String> {
    let host = payload
        .external_host
        .as_deref()
        .map(str::trim)
        .filter(|host| !host.is_empty());
    let port = payload.external_port;
    let scheme = payload
        .external_scheme
        .as_deref()
        .map(str::trim)
        .filter(|scheme| !scheme.is_empty());

    if host.is_none() && port.is_none() && scheme.is_none() {
        return Ok(None);
    }

    let Some(host) = host else {
        return Err("external_host is required when registering an external endpoint.".to_string());
    };
    let Some(port) = port else {
        return Err("external_port is required when registering an external endpoint.".to_string());
    };

    let scheme = scheme.unwrap_or("http").to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https") {
        return Err("external_scheme must be either 'http' or 'https'.".to_string());
    }

    Ok(Some(ExternalServiceEndpoint {
        ip: host.to_string(),
        port,
        scheme,
    }))
}

pub(crate) fn get_container_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-mesh-container-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
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
