//! # Rusty Mesh Server Library
//!
//! Core router setup, state management, and middleware integration for the
//! in-memory mesh service registry.

use crate::core::controllers::health::health::welcome;
use crate::core::router::mesh_routes;
use crate::core::services::registry::registry_store::RegistryStore;
use crate::middlewares::logging_middleware::logging_middleware;
use crate::middlewares::request_timeout_middleware::timeout_middleware;
use crate::utils::load_config::AppConfig;
use axum::{Router, middleware};
use std::sync::Arc;

pub mod core;
pub mod middlewares;
pub mod utils;

/// Global application state shared across all routes and middlewares.
#[derive(Clone, Debug)]
pub struct AppState {
    /// Application configuration loaded from TOML and environment variables.
    pub config: Arc<AppConfig>,
    /// Thread-safe in-memory registry of active service instances.
    pub registry: RegistryStore,
}

/// Creates the main Axum application router.
///
/// Mesh routes are nested under `/api/v1/mesh`, matching the API-versioned
/// routing style used across the Rust services.
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/", axum::routing::get(welcome))
        .nest("/api/v1/mesh", mesh_routes(&state))
        .layer(middleware::from_fn(logging_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            timeout_middleware,
        ))
        .with_state(state)
}
