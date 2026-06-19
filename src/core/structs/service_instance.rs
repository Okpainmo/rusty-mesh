use serde::{Deserialize, Serialize};

/// Registered service instance held by the in-memory mesh registry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceInstance {
    pub name: String,
    pub version: String,
    pub ip: String,
    pub port: u16,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_scheme: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalServiceEndpoint {
    pub ip: String,
    pub port: u16,
    pub scheme: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceEndpointResponse {
    pub name: String,
    pub version: String,
    pub ip: String,
    pub port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub timestamp: u64,
    pub url: String,
}

impl ServiceInstance {
    pub fn external_response(&self) -> ServiceEndpointResponse {
        let ip = self.external_ip.clone().unwrap_or_else(|| self.ip.clone());
        let port = self.external_port.unwrap_or(self.port);
        let scheme = self
            .external_scheme
            .as_deref()
            .filter(|scheme| !scheme.trim().is_empty())
            .unwrap_or("http");

        ServiceEndpointResponse {
            name: self.name.clone(),
            version: self.version.clone(),
            url: format!("{}://{}:{}", scheme, ip, port),
            ip,
            port,
            internal_ip: self.ip.clone(),
            internal_port: self.port,
            timestamp: self.timestamp,
        }
    }
}
