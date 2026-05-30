use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tokio::task::JoinHandle;

#[derive(Clone, Debug)]
pub struct MeshRegistryClient {
    http: Client,
    mesh_url: String,
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

impl MeshRegistryClient {
    pub fn new(
        mesh_url: impl Into<String>,
        advertised_host: impl Into<String>,
        service_name: impl Into<String>,
        service_version: impl Into<String>,
        service_port: u16,
    ) -> Self {
        Self {
            http: Client::new(),
            mesh_url: mesh_url.into().trim_end_matches('/').to_string(),
            advertised_host: advertised_host.into(),
            service_name: service_name.into(),
            service_version: service_version.into(),
            service_port,
        }
    }

    pub async fn register(&self) -> Result<()> {
        self.http
            .post(format!("{}/api/v1/mesh/services", self.mesh_url))
            .header("x-forwarded-for", self.advertised_host.as_str())
            .json(&self.registration_body())
            .send()
            .await
            .context("failed to send service registration request")?
            .error_for_status()
            .context("mesh rejected service registration request")?;

        Ok(())
    }

    pub async fn unregister(&self) -> Result<()> {
        self.http
            .delete(format!("{}/api/v1/mesh/services", self.mesh_url))
            .header("x-forwarded-for", self.advertised_host.as_str())
            .json(&self.registration_body())
            .send()
            .await
            .context("failed to send service unregistration request")?
            .error_for_status()
            .context("mesh rejected service unregistration request")?;

        Ok(())
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
}
