use crate::AppState;
use crate::core::controllers::health::health::{health_check, welcome};
use crate::core::controllers::registry::find_service::{find_balanced_service, find_service};
use crate::core::controllers::registry::heartbeat_service::heartbeat_service;
use crate::core::controllers::registry::list_services::list_services;
use crate::core::controllers::registry::register_service::register_service;
use crate::core::controllers::registry::unregister_service::unregister_service;
use crate::middlewares::mesh_auth_middleware::mesh_auth_middleware;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn mesh_routes(state: &AppState) -> Router<AppState> {
    let protected_registry_routes = Router::new()
        .route(
            "/services",
            get(list_services)
                .post(register_service)
                .delete(unregister_service),
        )
        .route("/services/heartbeat", post(heartbeat_service))
        .route(
            "/services/{service_name}/{service_version}/{service_port}",
            get(find_service),
        )
        .route(
            "/services/{service_name}/{service_version}",
            get(find_balanced_service),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mesh_auth_middleware,
        ));

    Router::new()
        .route("/", get(welcome))
        .route("/health", get(health_check))
        .merge(protected_registry_routes)
}
