use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServiceRegistrationRequest {
    pub service_name: String,
    pub service_version: String,
    pub service_ip: Option<String>,
    pub service_port: u16,
    pub external_host: Option<String>,
    pub external_port: Option<u16>,
    pub external_scheme: Option<String>,
}
