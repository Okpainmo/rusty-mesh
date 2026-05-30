use crate::core::structs::service_instance::ServiceInstance;
use rand::seq::SliceRandom;
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
}

/// Thread-safe in-memory store for active service instances.
#[derive(Clone, Debug)]
pub struct RegistryStore {
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    ttl_secs: u64,
}

impl RegistryStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn unregister(&self, name: String, version: String, ip: String, port: u16) -> String {
        let key = Self::get_key(&name, &version, &ip, port);
        let mut services = self.services.write().await;
        services.remove(&key);
        info!(
            "Deleted service {}, version {} at {}:{}",
            name, version, ip, port
        );
        key
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
            .filter(|service| service.port == port)
            .filter(|service| {
                Version::parse(&service.version)
                    .map(|version| requirement.matches(&version))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();

        Ok(candidates.choose(&mut rand::thread_rng()).cloned())
    }

    pub async fn list(&self) -> Vec<ServiceInstance> {
        let mut services = self.services.write().await;
        Self::cleanup_locked(&mut services, self.ttl_secs);

        let mut service_list = services.values().cloned().collect::<Vec<_>>();
        service_list.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.version.cmp(&b.version))
                .then_with(|| a.ip.cmp(&b.ip))
                .then_with(|| a.port.cmp(&b.port))
        });
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
            )
            .await
            .expect("service registration should succeed");
        let second_key = registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                3000,
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
    async fn unregister_removes_service() {
        let registry = RegistryStore::new(15);

        registry
            .register(
                "catalog".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                3001,
            )
            .await
            .expect("service registration should succeed");

        registry
            .unregister(
                "catalog".to_string(),
                "2.0.0".to_string(),
                "127.0.0.1".to_string(),
                3001,
            )
            .await;

        let service = registry
            .find("catalog", "^2.0.0", 3001)
            .await
            .expect("lookup should succeed");

        assert!(service.is_none());
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
            )
            .await
            .expect("service registration should succeed");
        registry
            .register(
                "orders".to_string(),
                "1.2.3".to_string(),
                "127.0.0.1".to_string(),
                4000,
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
    async fn rejects_invalid_versions() {
        let registry = RegistryStore::new(15);

        let error = registry
            .register(
                "orders".to_string(),
                "not-semver".to_string(),
                "127.0.0.1".to_string(),
                3000,
            )
            .await
            .expect_err("invalid semver should be rejected");

        assert!(matches!(error, RegistryError::InvalidVersion(_)));
    }
}
