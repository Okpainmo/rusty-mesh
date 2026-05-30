use crate::AppState;
use crate::core::controllers::registry::register_service::get_service_ip;
use crate::core::structs::registry_response::{ApiResponse, ServiceRegistrationResponse};
use crate::core::structs::service_registration_request::ServiceRegistrationRequest;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn unregister_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<ServiceRegistrationRequest>, JsonRejection>,
) -> impl IntoResponse {
    let service_ip = get_service_ip(&headers);
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<ServiceRegistrationResponse>::error(
                    format!("Invalid service unregistration request body: {}", error),
                    "INVALID_REQUEST_BODY",
                )),
            )
                .into_response();
        }
    };
    let service_name = payload.service_name;
    let service_version = payload.service_version;
    let service_port = payload.service_port;

    let registration_response = ServiceRegistrationResponse {
        service_name: service_name.clone(),
        service_version: service_version.clone(),
        service_ip: service_ip.clone(),
        service_port,
    };

    state
        .registry
        .unregister(service_name, service_version, service_ip, service_port)
        .await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            "Service unregistered successfully",
            registration_response,
        )),
    )
        .into_response()
}
