use crate::core::structs::service_instance::{ExternalServiceEndpoint, ServiceInstance};
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("invalid semantic version '{0}'")]
    InvalidVersion(String),
    #[error("invalid semantic version requirement '{0}'")]
    InvalidVersionRequirement(String),
    #[error("service instance is not registered")]
    ServiceNotRegistered,
}

/// Thread-safe in-memory store for active service instances.
#[derive(Clone, Debug)]
pub struct RegistryStore {
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    round_robin_cursors: Arc<RwLock<HashMap<String, usize>>>,
    ttl_secs: u64,
}

impl RegistryStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            round_robin_cursors: Arc::new(RwLock::new(HashMap::new())),
            ttl_secs,
        }
    }

    pub fn get_key(name: &str, version: &str, ip: &str, port: u16) -> String {
        format!("{}{}{}{}", name, version, ip, port)
    }

    pub async fn register(
        &self,
        name: String,
        version: String,
        ip: String,
        port: u16,
        external_endpoint: Option<ExternalServiceEndpoint>,
    ) -> Result<String, RegistryError> {
        Version::parse(&version).map_err(|_| RegistryError::InvalidVersion(version.clone()))?;

        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let key = Self::get_key(&name, &version, &ip, port);
        let timestamp = current_unix_timestamp_secs();
        let was_registered = services.contains_key(&key);

        services.insert(
            key.clone(),
            ServiceInstance {
                name: name.clone(),
                version: version.clone(),
                ip: ip.clone(),
                port,
                timestamp,
                external_ip: external_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.ip.clone()),
                external_port: external_endpoint.as_ref().map(|endpoint| endpoint.port),
                external_scheme: external_endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.scheme.clone()),
            },
        );

        if was_registered {
            info!(
                "Updated service {}, version {} at {}:{}",
                name, version, ip, port
            );
        } else {
            info!(
                "Added service {}, version {} at {}:{}",
                name, version, ip, port
            );
        }

        Ok(key)
    }

    pub async fn heartbeat(
        &self,
        name: String,
        version: String,
        ip: String,
        port: u16,
    ) -> Result<ServiceInstance, RegistryError> {
        Version::parse(&version).map_err(|_| RegistryError::InvalidVersion(version.clone()))?;

        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let key = Self::get_key(&name, &version, &ip, port);
        if let Some(service) = services.get_mut(&key) {
            service.timestamp = current_unix_timestamp_secs();
            info!(
                "Refreshed service {}, version {} at {}:{} (Internal)",
                name, version, ip, port
            );
            return Ok(service.clone());
        }

        // Fallback: search for a service that matches the name/version AND either internal OR external identity
        let found_key = services.iter_mut().find_map(|(k, service)| {
            let matches_name = service.name == name && service.version == version;
            let matches_internal = service.ip == ip && service.port == port;
            let matches_external =
                service.external_ip.as_ref() == Some(&ip) && service.external_port == Some(port);

            if matches_name && (matches_internal || matches_external) {
                Some(k.clone())
            } else {
                None
            }
        });

        if let Some(key) = found_key {
            let service = services.get_mut(&key).unwrap();
            service.timestamp = current_unix_timestamp_secs();
            info!(
                "Refreshed service {}, version {} at {}:{} (via Identity Match)",
                name, version, ip, port
            );
            Ok(service.clone())
        } else {
            Err(RegistryError::ServiceNotRegistered)
        }
    }

    pub async fn unregister(
        &self,
        name: String,
        version: String,
        ip: String,
        port: u16,
    ) -> Result<ServiceInstance, RegistryError> {
        Version::parse(&version).map_err(|_| RegistryError::InvalidVersion(version.clone()))?;

        let key = Self::get_key(&name, &version, &ip, port);
        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let Some(service) = services.remove(&key) else {
            return Err(RegistryError::ServiceNotRegistered);
        };

        info!(
            "Deleted service {}, version {} at {}:{}",
            service.name, service.version, service.ip, service.port
        );

        Ok(service)
    }

    pub async fn find(
        &self,
        name: &str,
        version_requirement: &str,
        port: u16,
    ) -> Result<Option<ServiceInstance>, RegistryError> {
        let requirement = VersionReq::parse(version_requirement).map_err(|_| {
            RegistryError::InvalidVersionRequirement(version_requirement.to_string())
        })?;

        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let candidates = services
            .values()
            .filter(|service| service.name == name)
            .filter(|service| service.lookup_port() == port)
            .filter(|service| {
                Version::parse(&service.version)
                    .map(|version| requirement.matches(&version))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();

        let cursor_key = format!("{}:{}:{}", name, version_requirement, port);
        Ok(self.select_round_robin(cursor_key, candidates).await)
    }

    pub async fn find_balanced(
        &self,
        name: &str,
        version_requirement: &str,
    ) -> Result<Option<ServiceInstance>, RegistryError> {
        let requirement = VersionReq::parse(version_requirement).map_err(|_| {
            RegistryError::InvalidVersionRequirement(version_requirement.to_string())
        })?;

        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let candidates = services
            .values()
            .filter(|service| service.name == name)
            .filter(|service| {
                Version::parse(&service.version)
                    .map(|version| requirement.matches(&version))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();

        let cursor_key = format!("{}:{}", name, version_requirement);
        Ok(self.select_round_robin(cursor_key, candidates).await)
    }

    pub async fn list(&self) -> Vec<ServiceInstance> {
        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let mut service_list = services.values().cloned().collect::<Vec<_>>();
        Self::sort_instances(&mut service_list);
        service_list
    }

    fn cleanup_locked(services: &mut HashMap<String, ServiceInstance>, ttl_secs: u64) {
        let now = current_unix_timestamp_secs();
        services.retain(|key, service| {
            let keep = service.timestamp.saturating_add(ttl_secs) >= now;
            if !keep {
                info!("Removed expired service {}", key);
            }
            keep
        });
    }

    async fn select_round_robin(
        &self,
        cursor_key: String,
        mut candidates: Vec<ServiceInstance>,
    ) -> Option<ServiceInstance> {
        if candidates.is_empty() {
            return None;
        }

        Self::sort_instances(&mut candidates);

        let mut cursors = self.round_robin_cursors.write().await;
        let cursor = cursors.entry(cursor_key).or_insert(0);
        let selected_index = *cursor % candidates.len();
        *cursor = cursor.saturating_add(1) % candidates.len();

        candidates.get(selected_index).cloned()
    }

    fn sort_instances(services: &mut [ServiceInstance]) {
        services.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.version.cmp(&b.version))
                .then_with(|| a.ip.cmp(&b.ip))
                .then_with(|| a.port.cmp(&b.port))
        });
    }
}

impl ServiceInstance {
    fn lookup_port(&self) -> u16 {
        self.external_port.unwrap_or(self.port)
    }
}

fn current_unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_refreshes_and_finds_matching_service() {
        let registry = RegistryStore::new(15);

        let first_key = registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
                None,
            )
            .await
            .expect("service registration should succeed");
        let second_key = registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
                None,
            )
            .await
            .expect("service refresh should succeed");

        assert_eq!(first_key, second_key);

        let service = registry
            .find("orders", "^1.0.0", 3000)
            .await
            .expect("lookup should succeed")
            .expect("service should be found");

        assert_eq!(service.name, "orders");
        assert_eq!(service.version, "1.2.3");
    }

    #[tokio::test]
    async fn service_external_response_uses_external_endpoint_when_available() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "orders-1".to_string(),
                30304,
                Some(ExternalServiceEndpoint {
                    ip: "127.0.0.1".to_string(),
                    port: 32770,
                    scheme: "https".to_string(),
                }),
            )
            .await
            .expect("service registration should succeed");

        let service = registry
            .list()
            .await
            .pop()
            .expect("service should be registered")
            .external_response();

        assert_eq!(service.name, "orders");
        assert_eq!(service.version, "1.2.3");
        assert_eq!(service.ip, "127.0.0.1");
        assert_eq!(service.port, 32770);
        assert_eq!(service.internal_ip, "orders-1");
        assert_eq!(service.internal_port, 30304);
        assert_eq!(service.url, "https://127.0.0.1:32770");
    }

    #[tokio::test]
    async fn heartbeat_refreshes_registered_service() {
        let registry = RegistryStore::new(15);

        let register_key = registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
                None,
            )
            .await
            .expect("service registration should succeed");

        let refreshed_service = registry
            .heartbeat(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
            )
            .await
            .expect("heartbeat should refresh registered service");

        assert_eq!(
            register_key,
            RegistryStore::get_key(
                &refreshed_service.name,
                &refreshed_service.version,
                &refreshed_service.ip,
                refreshed_service.port
            )
        );
    }

    #[tokio::test]
    async fn heartbeat_returns_refreshed_service_with_external_endpoint() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "orders-1".to_string(),
                30304,
                Some(ExternalServiceEndpoint {
                    ip: "127.0.0.1".to_string(),
                    port: 32770,
                    scheme: "http".to_string(),
                }),
            )
            .await
            .expect("service registration should succeed");

        let refreshed_service = registry
            .heartbeat(
                "orders".to_string(),
                "1.2.3".to_string(),
                "orders-1".to_string(),
                30304,
            )
            .await
            .expect("heartbeat should refresh registered service");

        assert_eq!(refreshed_service.ip, "orders-1");
        assert_eq!(refreshed_service.port, 30304);
        assert_eq!(refreshed_service.external_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(refreshed_service.external_port, Some(32770));
    }

    #[tokio::test]
    async fn heartbeat_rejects_unknown_service() {
        let registry = RegistryStore::new(15);

        let error = registry
            .heartbeat(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
            )
            .await
            .expect_err("unknown service heartbeat should fail");

        assert!(matches!(error, RegistryError::ServiceNotRegistered));
    }

    #[tokio::test]
    async fn unregister_removes_service() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "catalog".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                3001,
                None,
            )
            .await
            .expect("service registration should succeed");

        let removed_service = registry
            .unregister(
                "catalog".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                3001,
            )
            .await
            .expect("registered service should be removed");

        assert_eq!(removed_service.name, "catalog");
        assert_eq!(removed_service.version, "2.0.0");
        assert_eq!(removed_service.ip, "127.0.0.1");
        assert_eq!(removed_service.port, 3001);

        let service = registry
            .find("catalog", "^2.0.0", 3001)
            .await
            .expect("lookup should succeed");

        assert!(service.is_none());
    }

    #[tokio::test]
    async fn unregister_rejects_unknown_service() {
        let registry = RegistryStore::new(15);

        let error = registry
            .unregister(
                "catalog".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                3001,
            )
            .await
            .expect_err("unknown service unregister should fail");

        assert!(matches!(error, RegistryError::ServiceNotRegistered));
    }

    #[tokio::test]
    async fn expired_services_are_cleaned_up() {
        let registry = RegistryStore::new(1);

        registry
            .register(
                "payments".to_string(),
                "1.0.0".to_string(),
                "127.0.0.1".to_string(),
                3002,
                None,
            )
            .await
            .expect("service registration should succeed");

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let service = registry
            .find("payments", "^1.0.0", 3002)
            .await
            .expect("lookup should succeed");

        assert!(service.is_none());
        assert!(registry.list().await.is_empty());
    }

    #[tokio::test]
    async fn find_filters_by_port() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
                None,
            )
            .await
            .expect("service registration should succeed");
        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                4000,
                None,
            )
            .await
            .expect("service registration should succeed");

        let service = registry
            .find("orders", "^1.0.0", 4000)
            .await
            .expect("lookup should succeed")
            .expect("service should be found");

        assert_eq!(service.port, 4000);
    }

    #[tokio::test]
    async fn find_filters_by_external_port_when_available() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "orders-1".to_string(),
                30304,
                Some(ExternalServiceEndpoint {
                    ip: "127.0.0.1".to_string(),
                    port: 32770,
                    scheme: "http".to_string(),
                }),
            )
            .await
            .expect("service registration should succeed");

        let internal_port_lookup = registry
            .find("orders", "^1.0.0", 30304)
            .await
            .expect("lookup should succeed");

        assert!(internal_port_lookup.is_none());

        let service = registry
            .find("orders", "^1.0.0", 32770)
            .await
            .expect("lookup should succeed")
            .expect("service should be found");

        assert_eq!(service.ip, "orders-1");
        assert_eq!(service.port, 30304);
        assert_eq!(service.external_port, Some(32770));
    }

    #[tokio::test]
    async fn find_balanced_cycles_through_sorted_matching_services() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "10.0.0.3".to_string(),
                3003,
                None,
            )
            .await
            .expect("service registration should succeed");
        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "10.0.0.1".to_string(),
                3001,
                None,
            )
            .await
            .expect("service registration should succeed");
        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "10.0.0.2".to_string(),
                3002,
                None,
            )
            .await
            .expect("service registration should succeed");

        let first = registry
            .find_balanced("orders", "^1.0.0")
            .await
            .expect("lookup should succeed")
            .expect("service should be found");
        let second = registry
            .find_balanced("orders", "^1.0.0")
            .await
            .expect("lookup should succeed")
            .expect("service should be found");
        let third = registry
            .find_balanced("orders", "^1.0.0")
            .await
            .expect("lookup should succeed")
            .expect("service should be found");
        let fourth = registry
            .find_balanced("orders", "^1.0.0")
            .await
            .expect("lookup should succeed")
            .expect("service should be found");

        assert_eq!(first.ip, "10.0.0.1");
        assert_eq!(second.ip, "10.0.0.2");
        assert_eq!(third.ip, "10.0.0.3");
        assert_eq!(fourth.ip, "10.0.0.1");
    }

    #[tokio::test]
    async fn find_balanced_filters_by_semver_requirement() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "10.0.0.1".to_string(),
                3001,
                None,
            )
            .await
            .expect("service registration should succeed");
        registry
            .register(
                "orders".to_string(),
                "2.0.0".to_string(),
                "10.0.0.2".to_string(),
                3002,
                None,
            )
            .await
            .expect("service registration should succeed");

        let service = registry
            .find_balanced("orders", "^2.0.0")
            .await
            .expect("lookup should succeed")
            .expect("service should be found");

        assert_eq!(service.version, "2.0.0");
    }

    #[tokio::test]
    async fn rejects_invalid_versions() {
        let registry = RegistryStore::new(15);

        let error = registry
            .register(
                "orders".to_string(),
                "not-semver".to_string(),
                "127.0.0.1".to_string(),
                3000,
                None,
            )
            .await
            .expect_err("invalid semver should be rejected");

        assert!(matches!(error, RegistryError::InvalidVersion(_)));
    }

    #[tokio::test]
    async fn heartbeat_fallback_matches_external_identity() {
        let registry = RegistryStore::new(15);

        // Register with an external endpoint
        registry
            .register(
                "catalog".to_string(),
                "1.0.0".to_string(),
                "internal-ip".to_string(),
                8080,
                Some(ExternalServiceEndpoint {
                    ip: "external-ip".to_string(),
                    port: 30000,
                    scheme: "http".to_string(),
                }),
            )
            .await
            .expect("registration should succeed");

        // Heartbeat using INTERNAL identity (should work)
        registry
            .heartbeat(
                "catalog".to_string(),
                "1.0.0".to_string(),
                "internal-ip".to_string(),
                8080,
            )
            .await
            .expect("heartbeat via internal identity should succeed");

        // Heartbeat using EXTERNAL identity (should work via fallback)
        registry
            .heartbeat(
                "catalog".to_string(),
                "1.0.0".to_string(),
                "external-ip".to_string(),
                30000,
            )
            .await
            .expect("heartbeat via external identity should succeed");

        // Heartbeat with WRONG port (should fail)
        registry
            .heartbeat(
                "catalog".to_string(),
                "1.0.0".to_string(),
                "external-ip".to_string(),
                9999,
            )
            .await
            .expect_err("heartbeat with wrong port should fail");
    }

    #[tokio::test]
    async fn heartbeat_fallback_respects_service_name_and_version() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "service-a".to_string(),
                "1.0.0".to_string(),
                "127.0.0.1".to_string(),
                8080,
                None,
            )
            .await
            .expect("registration should succeed");

        // Heartbeat with correct IP/Port but WRONG name
        registry
            .heartbeat(
                "service-b".to_string(),
                "1.0.0".to_string(),
                "127.0.0.1".to_string(),
                8080,
            )
            .await
            .expect_err("heartbeat with wrong name should fail");

        // Heartbeat with correct IP/Port but WRONG version
        registry
            .heartbeat(
                "service-a".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                8080,
            )
            .await
            .expect_err("heartbeat with wrong version should fail");
    }
}
