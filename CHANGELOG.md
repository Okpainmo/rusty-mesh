# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

---

## [1.0.0] - 2026-06-20

### Added

- Added the first production-facing Rusty Mesh control plane with Axum routes under `/api/v1/mesh`.
- Added public health and welcome endpoints.
- Added protected service registry endpoints for registration, heartbeat refresh, discovery,
  listing, and unregistration.
- Added semantic-version service discovery with sorted round-robin load balancing.
- Added exact-port discovery that uses external ports when an external endpoint exists, with
  internal-port fallback when no external endpoint is registered.
- Added heartbeat-based in-memory service TTL cleanup.
- Added dual endpoint responses with public `ip`, `port`, and `url` fields plus `internal_ip` and
  `internal_port` for in-network calls.
- Added optional explicit external endpoint registration through `external_host`, `external_port`,
  and `external_scheme`.
- Added opt-in Docker endpoint resolution for dynamic host-port deployments.
- Added `x-mesh-endpoint-scope: internal` support for private service-to-service discovery.
- Added shared mesh-token protection for registry routes through Bearer auth and the `x-mesh-token`
  header.
- Added TOML, `.env`, environment-specific `.env.{environment}`, and `APP__` environment based
  configuration loading.
- Added startup configuration validation for server settings, registry heartbeat/TTL settings, and
  required mesh-token configuration.
- Added structured JSON logging, request timeout middleware, and logging middleware.
- Added standalone Docker support for the mesh service.
- Added demo microservices for Rust, Node, and Python runtimes, including Compose orchestration,
  heartbeat refresh, shutdown unregistration, and peer discovery calls.
- Added Postman collections for the mesh-core API and demo microservices.
- Added README onboarding, API reference, Docker resolver guidance, demo instructions, and security
  notes.

### Changed

- Standardized registry API responses around `response_message`, `response`, and `error`.
- Standardized registration and heartbeat responses to return only `ip`/`port` and
  `internal_ip`/`internal_port`, removing duplicate service endpoint aliases.
- Documented Docker inspection as an intentional trusted control-plane privilege when enabled in
  production.

### Fixed

- Fixed heartbeat responses so they return the refreshed stored external endpoint instead of falling
  back to the internal endpoint.
- Fixed unregister behavior so missing service instances return `404` with `SERVICE_NOT_REGISTERED`.
- Fixed demo and README examples so Docker-network calls request internal discovery scope where
  needed.

### Security

- Documented Docker socket access risks and production guardrails for the Docker resolver.
- Kept Docker endpoint resolution disabled by default through
  `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=none`.
- Kept `/health` public while protecting all registry routes with the shared mesh token by default.
- Compared mesh tokens without short-circuiting on the first mismatched byte.
