use crate::AppState;
use crate::core::controllers::health::health::health_check;
use crate::core::controllers::registry::find_service::find_service;
use crate::core::controllers::registry::list_services::list_services;
use crate::core::controllers::registry::register_service::register_service;
use crate::core::controllers::registry::unregister_service::unregister_service;
use axum::{Router, routing::get};

pub fn mesh_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/services",
            get(list_services)
                .post(register_service)
                .delete(unregister_service),
        )
        .route(
            "/services/{service_name}/{service_version}/{service_port}",
            get(find_service),
        )
}
