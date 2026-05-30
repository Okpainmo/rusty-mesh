use serde::{Deserialize, Serialize};

/// Registered service instance held by the in-memory mesh registry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceInstance {
    pub name: String,
    pub version: String,
    pub ip: String,
    pub port: u16,
    pub timestamp: u64,
}
