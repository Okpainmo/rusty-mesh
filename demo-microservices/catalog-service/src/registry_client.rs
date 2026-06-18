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
}

#[derive(Debug, Serialize)]
struct ServiceRegistrationRequest<'a> {
    service_name: &'a str,
    service_version: &'a str,
    service_port: u16,
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
        }
    }

    pub async fn register(&self) -> Result<()> {
        self.authorized(
            self.http
                .post(format!("{}/api/v1/mesh/services", self.mesh_url)),
        )
        .header("x-mesh-advertise-host", self.advertised_host.as_str())
        .json(&self.registration_body())
        .send()
        .await
        .context("failed to send service registration request")?
        .error_for_status()
        .context("mesh rejected service registration request")?;

        Ok(())
    }

    pub async fn unregister(&self) -> Result<()> {
        self.authorized(
            self.http
                .delete(format!("{}/api/v1/mesh/services", self.mesh_url)),
        )
        .header("x-mesh-advertise-host", self.advertised_host.as_str())
        .json(&self.registration_body())
        .send()
        .await
        .context("failed to send service unregistration request")?
        .error_for_status()
        .context("mesh rejected service unregistration request")?;

        Ok(())
    }

    pub async fn discover(&self, service_name: &str) -> Result<ServiceInstance> {
        let version_requirement = "^1.0.0";
        let encoded_requirement = encode_version_requirement(version_requirement);

        let response = self
            .authorized(self.http.get(format!(
                "{}/api/v1/mesh/services/{}/{}",
                self.mesh_url, service_name, encoded_requirement
            )))
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

                if let Err(error) = client.register().await {
                    eprintln!(
                        "{}:{} heartbeat registration failed: {error:#}",
                        client.service_name, client.service_version
                    );
                }
            }
        })
    }

    fn registration_body(&self) -> ServiceRegistrationRequest<'_> {
        ServiceRegistrationRequest {
            service_name: &self.service_name,
            service_version: &self.service_version,
            service_port: self.service_port,
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
