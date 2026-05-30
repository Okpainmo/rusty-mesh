#[path = "common/mod.rs"]
mod common;

#[path = "controllers/health/health.rs"]
mod health;

#[path = "controllers/registry/register_find_unregister_service.rs"]
mod register_find_unregister_service;

#[path = "controllers/registry/register_rejects_invalid_request_body.rs"]
mod register_rejects_invalid_request_body;

#[path = "controllers/registry/register_rejects_invalid_version.rs"]
mod register_rejects_invalid_version;
