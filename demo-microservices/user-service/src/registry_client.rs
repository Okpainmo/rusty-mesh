use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinHandle;

#[derive(Clone, Debug)]
pub struct MeshRegistryClient {
    http: Client,
    mesh_url: String,
    mesh_token: Option<String>,
    advertised_host: String,
    service_name: String,
    service_version: String,
    service_port: u16,
    container_id: Option<String>,
    external_host: Option<String>,
    external_port: Option<u16>,
    external_scheme: String,
}

#[derive(Debug, Serialize)]
struct ServiceRegistrationRequest<'a> {
    service_name: &'a str,
    service_version: &'a str,
    service_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_host: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_scheme: Option<&'a str>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EndpointDetails {
    pub ip: String,
    pub port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceInstance {
    pub name: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
struct MeshResponse<T> {
    response: Option<T>,
}

impl MeshRegistryClient {
    pub fn new(
        mesh_url: impl Into<String>,
        mesh_token: Option<String>,
        advertised_host: impl Into<String>,
        service_name: impl Into<String>,
        service_version: impl Into<String>,
        service_port: u16,
        container_id: Option<String>,
        external_host: Option<String>,
        external_port: Option<u16>,
        external_scheme: impl Into<String>,
    ) -> Self {
        Self {
            http: Client::new(),
            mesh_url: mesh_url.into().trim_end_matches('/').to_string(),
            mesh_token: mesh_token
                .map(|token| token.trim().to_string())
                .filter(|token| !token.is_empty()),
            advertised_host: advertised_host.into(),
            service_name: service_name.into(),
            service_version: service_version.into(),
            service_port,
            container_id: container_id
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty()),
            external_host: external_host
                .map(|host| host.trim().to_string())
                .filter(|host| !host.is_empty()),
            external_port,
            external_scheme: external_scheme.into(),
        }
    }

    pub async fn register(&self) -> Result<EndpointDetails> {
        self.send_registry_request(
            self.http
                .post(format!("{}/api/v1/mesh/services", self.mesh_url)),
            "registration",
        )
        .await
        .and_then(|response| response.context("mesh service registration response was empty"))
    }

    pub async fn heartbeat(&self) -> Result<()> {
        self.send_registry_request(
            self.http
                .post(format!("{}/api/v1/mesh/services/heartbeat", self.mesh_url)),
            "heartbeat",
        )
        .await
        .map(|_| ())
    }

    pub async fn unregister(&self) -> Result<()> {
        self.send_registry_request(
            self.http
                .delete(format!("{}/api/v1/mesh/services", self.mesh_url)),
            "unregistration",
        )
        .await
        .map(|_| ())
    }

    async fn send_registry_request(
        &self,
        request: reqwest::RequestBuilder,
        action: &str,
    ) -> Result<Option<EndpointDetails>> {
        let mut request = self
            .authorized(request)
            .header("x-mesh-advertise-host", self.advertised_host.as_str());
        if let Some(container_id) = self.container_id.as_deref() {
            request = request.header("x-mesh-container-id", container_id);
        }

        let response = request
            .json(&self.registration_body())
            .send()
            .await
            .with_context(|| format!("failed to send service {action} request"))?
            .error_for_status()
            .with_context(|| format!("mesh rejected service {action} request"))?
            .json::<MeshResponse<EndpointDetails>>()
            .await
            .with_context(|| format!("failed to decode service {action} response"))?
            .response;

        Ok(response)
    }

    pub async fn discover(&self, service_name: &str) -> Result<ServiceInstance> {
        let version_requirement = "^1.0.0";
        let encoded_requirement = encode_version_requirement(version_requirement);

        let response = self
            .authorized(self.http.get(format!(
                "{}/api/v1/mesh/services/{}/{}",
                self.mesh_url, service_name, encoded_requirement
            )))
            .header("x-mesh-endpoint-scope", "internal")
            .send()
            .await
            .context("failed to request mesh service discovery")?
            .error_for_status()
            .context("mesh rejected service discovery request")?
            .json::<MeshResponse<ServiceInstance>>()
            .await
            .context("failed to decode mesh service discovery response")?;

        response
            .response
            .context("mesh service discovery response was empty")
    }

    pub async fn call_feedback(&self, service: &ServiceInstance, path: &str) -> Result<Value> {
        self.http
            .get(format!("http://{}:{}{}", service.ip, service.port, path))
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to call feedback endpoint '{}' for {} at {}:{}",
                    path, service.name, service.ip, service.port
                )
            })?
            .error_for_status()
            .context("peer feedback endpoint returned an error")?
            .json::<Value>()
            .await
            .context("failed to decode peer feedback response")
    }

    pub fn start_heartbeat(&self, heartbeat_interval_secs: u64) -> JoinHandle<()> {
        let client = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(heartbeat_interval_secs));

            loop {
                interval.tick().await;

                if let Err(error) = client.heartbeat().await {
                    eprintln!(
                        "{}:{} heartbeat failed: {error:#}",
                        client.service_name, client.service_version
                    );
                }
            }
        })
    }

    fn registration_body(&self) -> ServiceRegistrationRequest<'_> {
        let has_external_endpoint = self.external_host.is_some() && self.external_port.is_some();

        ServiceRegistrationRequest {
            service_name: &self.service_name,
            service_version: &self.service_version,
            service_port: self.service_port,
            external_host: if has_external_endpoint {
                self.external_host.as_deref()
            } else {
                None
            },
            external_port: if has_external_endpoint {
                self.external_port
            } else {
                None
            },
            external_scheme: if has_external_endpoint {
                Some(self.external_scheme.as_str())
            } else {
                None
            },
        }
    }

    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.mesh_token.as_deref() {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }
}

fn encode_version_requirement(requirement: &str) -> String {
    requirement
        .replace('^', "%5E")
        .replace('>', "%3E")
        .replace('<', "%3C")
        .replace('=', "%3D")
        .replace(' ', "%20")
}
