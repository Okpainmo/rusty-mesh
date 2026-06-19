use crate::core::structs::service_instance::ServiceEndpointResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub response_message: String,
    pub response: Option<T>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub message: String,
    pub code: String,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(message: impl Into<String>, response: T) -> Self {
        Self {
            response_message: message.into(),
            response: Some(response),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        let message = message.into();

        Self {
            response_message: message.clone(),
            response: None,
            error: Some(ApiError {
                message,
                code: code.into(),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServiceRegistrationResponse {
    pub service_name: String,
    pub service_version: String,
    pub ip: String,
    pub port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub registry_policy: RegistryPolicyResponse,
}

#[derive(Debug, Serialize)]
pub struct WelcomeResponse {
    pub service: String,
    pub status: String,
    pub message: String,
    pub health_url: String,
    pub registry_url: String,
}

#[derive(Debug, Serialize)]
pub struct RegistryPolicyResponse {
    pub heartbeat_interval_secs: u64,
    pub service_ttl_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct ServicesResponse {
    #[serde(rename = "services-count")]
    pub services_count: usize,
    pub services: Vec<ServiceEndpointResponse>,
}
