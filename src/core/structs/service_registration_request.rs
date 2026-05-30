use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServiceRegistrationRequest {
    pub service_name: String,
    pub service_version: String,
    pub service_port: u16,
}
