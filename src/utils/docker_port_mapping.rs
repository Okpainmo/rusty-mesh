use crate::core::structs::service_instance::ExternalServiceEndpoint;
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use tracing::warn;

const DOCKER_SOCKET_PATH: &str = "/var/run/docker.sock";

#[derive(Debug, Deserialize)]
struct DockerContainerInspect {
    #[serde(rename = "NetworkSettings")]
    network_settings: DockerNetworkSettings,
}

#[derive(Debug, Deserialize)]
struct DockerNetworkSettings {
    #[serde(rename = "Ports")]
    ports: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DockerPortBinding {
    #[serde(rename = "HostIp")]
    host_ip: String,
    #[serde(rename = "HostPort")]
    host_port: String,
}

pub async fn resolve_external_endpoint(
    container_id: Option<String>,
    internal_port: u16,
    public_host: Option<String>,
) -> Option<ExternalServiceEndpoint> {
    let container_id = container_id?.trim().to_string();
    if container_id.is_empty() {
        return None;
    }

    let inspected_container_id = container_id.clone();
    tokio::task::spawn_blocking(move || {
        inspect_published_port(
            &inspected_container_id,
            internal_port,
            public_host.as_deref(),
        )
    })
    .await
    .ok()
    .and_then(|result| match result {
        Ok(endpoint) => Some(endpoint),
        Err(error) => {
            warn!(
                "failed to resolve Docker-published endpoint for container {} port {}: {error:#}",
                container_id, internal_port
            );
            None
        }
    })
}

fn inspect_published_port(
    container_id: &str,
    internal_port: u16,
    public_host: Option<&str>,
) -> Result<ExternalServiceEndpoint> {
    let mut stream = UnixStream::connect(DOCKER_SOCKET_PATH)
        .with_context(|| format!("failed to connect to {}", DOCKER_SOCKET_PATH))?;
    let path = format!("/containers/{}/json", container_id);
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n",
        path
    );

    stream
        .write_all(request.as_bytes())
        .context("failed to send Docker inspect request")?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("failed to read Docker inspect response")?;

    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("Docker inspect response did not include a body"))?;
    let body = if headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        decode_chunked_body(body).context("failed to decode chunked Docker inspect response")?
    } else {
        body.to_string()
    };

    let inspect = serde_json::from_str::<DockerContainerInspect>(&body)
        .context("failed to decode Docker inspect response")?;
    let bindings = inspect
        .network_settings
        .ports
        .get(&format!("{}/tcp", internal_port))
        .and_then(|value| {
            serde_json::from_value::<Option<Vec<DockerPortBinding>>>(value.clone()).ok()
        })
        .flatten()
        .ok_or_else(|| anyhow!("container port {} is not published", internal_port))?;

    let binding = bindings
        .first()
        .ok_or_else(|| anyhow!("container port {} has no host bindings", internal_port))?;
    let port = binding
        .host_port
        .parse::<u16>()
        .context("published Docker host port was not a valid u16")?;
    let ip = normalize_public_host(&binding.host_ip, public_host);

    Ok(ExternalServiceEndpoint {
        ip,
        port,
        scheme: "http".to_string(),
    })
}

fn decode_chunked_body(body: &str) -> Result<String> {
    let mut decoded = String::new();
    let mut rest = body;

    loop {
        let (size_line, after_size) = rest
            .split_once("\r\n")
            .ok_or_else(|| anyhow!("chunked body was missing chunk size terminator"))?;
        let size = usize::from_str_radix(size_line.trim(), 16)
            .context("chunked body contained invalid chunk size")?;
        if size == 0 {
            break;
        }
        if after_size.len() < size + 2 {
            return Err(anyhow!("chunked body ended before the declared chunk size"));
        }

        decoded.push_str(&after_size[..size]);
        rest = &after_size[size + 2..];
    }

    Ok(decoded)
}

fn normalize_public_host(host_ip: &str, public_host: Option<&str>) -> String {
    if let Some(public_host) = public_host.map(str::trim).filter(|host| !host.is_empty()) {
        return public_host.to_string();
    }

    match host_ip.trim() {
        "" | "0.0.0.0" | "::" => "127.0.0.1".to_string(),
        value => value.to_string(),
    }
}
