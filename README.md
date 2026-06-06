# Rusty Mesh

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

Rusty Mesh is a fast in-memory orchestration layer for microservice/distributed systems. It gives
services a focused HTTP control plane for registration, heartbeat-based liveness, semantic version
matching, health checks, and sorted round-robin load balancing across compatible instances.

Built with Rust, Axum, and Tokio, Rusty Mesh is designed for teams that want service discovery they
can run locally, ship in containers, and reason about without pulling in an external database on day
one. The API and internal structure leave a clear path toward persistent or distributed backends as
deployment needs grow.

> The project's main focus is the mesh(orchestrator) service. But for easy onboarding, included, is
> a [demo-microservices directory](./demo-microservices) with four(4) demo
> microservices(`order-service - nodejs`, `user-service - rust`, `cart-service - python`, and
> `catalog-service - rust`) that use the orchestrator, and would help to clearly guide engineering
> teams with integrating `rusty-mesh` into their microservices/distributed systems builds.

## Core Capabilities

- Register service instances by name, semantic version, IP address, and port.

- Refresh an existing registration by calling the register endpoint again.

- Unregister service instances explicitly.

- Discover a compatible service instance using semantic-version requirements.

- Select compatible service instances with sorted round-robin load balancing.

- Automatically remove expired service instances using a configured heartbeat interval and TTL.

- List currently active service instances.

- Run as a standalone Docker container without the repository `compose.yaml`.

- Load configuration from TOML files and `APP__` environment variable overrides.

- Emit structured JSON logs through `tracing`.

## Architecture

Rusty Mesh follows the same application shape used by the sibling Rust services in this workspace:

```text
src/
  main.rs                  # binary entry point
  lib.rs                   # app state and router assembly
  core/
    router.rs              # versioned route definitions
    controllers/registry/  # HTTP handlers
    services/registry/     # registry domain logic
    structs/               # request/response domain structs
  middlewares/             # logging and timeout middleware
  utils/                   # environment and config loading
```

Routes are mounted under:

```text
/api/v1/mesh
```

The registry itself is an in-memory `HashMap` wrapped in async-safe shared state. Each service
instance stores:

- `name`
- `version`
- `ip`
- `port`
- `timestamp`

Expired entries are removed during register, find, and list operations. Load-balanced discovery
sorts matching instances by name, version, IP address, and port, then returns the next instance
using a round-robin cursor for that service and version requirement.

Rusty Mesh treats service registration as a heartbeat-driven contract. Registered services should
refresh their registration before `registry.service_ttl_secs` elapses. Startup validation enforces
that `registry.heartbeat_interval_secs` is lower than `registry.service_ttl_secs`, so the configured
policy always leaves room for missed or delayed heartbeats before an instance is considered stale.

## Requirements

- Rust `1.85+`
- Cargo
- Docker, optional, for containerized runs

## Quick Start

Run the service locally:

```bash
APP__ENV=development cargo run
```

By default, development mode starts the server at:

```text
http://127.0.0.1:3080
```

Check health:

```bash
curl http://127.0.0.1:3080/api/v1/mesh/health
```

Expected response:

```json
{
  "response_message": "Mesh service is healthy",
  "response": {
    "status": "ok",
    "service": "mesh_service",
    "registry_policy": {
      "heartbeat_interval_secs": 5,
      "service_ttl_secs": 15
    }
  },
  "error": null
}
```

## Docker

Rusty Mesh includes a standalone Docker setup. The repository `compose.yaml` is intentionally not
required for running the mesh service by itself.

Build the image:

```bash
docker build -t rusty-mesh .
```

Run the container:

```bash
docker run --rm -p 3080:3080 rusty-mesh
```

The Docker image defaults to:

```text
APP__ENV=production
APP__SERVER__HOST=0.0.0.0
APP__SERVER__PORT=3080
APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=5
APP__REGISTRY__SERVICE_TTL_SECS=15
```

Override runtime configuration when needed:

```bash
docker run --rm \
  -p 4080:4080 \
  -e APP__SERVER__PORT=4080 \
  -e APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=10 \
  -e APP__REGISTRY__SERVICE_TTL_SECS=30 \
  rusty-mesh
```

## API Reference

All endpoints are nested under:

```text
/api/v1/mesh
```

| Method | Path                                                        | Purpose                                                    |
| ------ | ----------------------------------------------------------- | ---------------------------------------------------------- |
| GET    | `/health`                                                   | Check service health                                       |
| GET    | `/services`                                                 | List active service instances                              |
| POST   | `/services`                                                 | Register or refresh an instance                            |
| DELETE | `/services`                                                 | Unregister an instance                                     |
| GET    | `/services/{service_name}/{service_version}`                | Find a compatible instance with round-robin load balancing |
| GET    | `/services/{service_name}/{service_version}/{service_port}` | Find a compatible instance on a specific port              |

### Register A Service

```bash
curl -X POST \
  -H "x-forwarded-for: 10.0.0.20" \
  -H "content-type: application/json" \
  -d '{
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_port": 3000
  }' \
  http://127.0.0.1:3080/api/v1/mesh/services
```

Response:

```json
{
  "response_message": "Service registered successfully",
  "response": {
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_ip": "10.0.0.20",
    "service_port": 3000
  },
  "error": null
}
```

Calling the same endpoint again refreshes the service timestamp.

Production service clients should call this endpoint on the configured heartbeat interval. With the
default policy, each service instance should refresh every `5` seconds and expires if it has not
refreshed within `15` seconds.

### Find A Service With Load Balancing

Semantic-version requirements are supported through the `semver` crate. Because characters such as
`^` are special in URLs, encode them in request paths.

```bash
curl http://127.0.0.1:3080/api/v1/mesh/services/orders/%5E1.0.0
```

Response:

```json
{
  "response_message": "Service found successfully",
  "response": {
    "name": "orders",
    "version": "1.2.3",
    "ip": "10.0.0.20",
    "port": 3000,
    "timestamp": 1790745600
  },
  "error": null
}
```

When multiple active instances match, Rusty Mesh sorts the candidates by name, version, IP address,
and port, then returns them in round-robin order on repeated requests.

### Find A Service On A Specific Port

The original port-specific lookup remains available when a client needs to target an exact
registered port:

```bash
curl http://127.0.0.1:3080/api/v1/mesh/services/orders/%5E1.0.0/3000
```

If no compatible service is found:

```json
{
  "response_message": "No matching service found.",
  "response": null,
  "error": {
    "message": "No matching service found.",
    "code": "SERVICE_NOT_FOUND"
  }
}
```

### List Services

```bash
curl http://127.0.0.1:3080/api/v1/mesh/services
```

Response:

```json
{
  "response_message": "Services listed successfully",
  "response": {
    "services": [
      {
        "name": "orders",
        "version": "1.2.3",
        "ip": "10.0.0.20",
        "port": 3000,
        "timestamp": 1790745600
      }
    ]
  },
  "error": null
}
```

### Unregister A Service

```bash
curl -X DELETE \
  -H "x-forwarded-for: 10.0.0.20" \
  -H "content-type: application/json" \
  -d '{
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_port": 3000
  }' \
  http://127.0.0.1:3080/api/v1/mesh/services
```

Response:

```json
{
  "response_message": "Service unregistered successfully",
  "response": {
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_ip": "10.0.0.20",
    "service_port": 3000
  },
  "error": null
}
```

## Service Discovery Semantics

Rusty Mesh accepts exact semantic versions during registration:

```text
1.2.3
```

Discovery accepts semantic-version requirements:

```text
1.2.3
^1.0.0
~1.2.0
>=1.0.0,<2.0.0
```

When multiple instances match a load-balanced discovery request, Rusty Mesh sorts the candidates and
returns them in round-robin order. This gives callers a predictable service-side balancing pattern
across compatible instances.

## Configuration

Configuration is loaded in this order, from lowest to highest priority:

1. `config/base.toml`
2. `config/{APP__ENV}.toml`
3. `config/local.toml`, if present
4. `APP__` environment variables

Default values live in [config/base.toml](config/base.toml).

| Variable                                 | Purpose                             | Default        |
| ---------------------------------------- | ----------------------------------- | -------------- |
| `APP__ENV`                               | Selects the config environment      | required       |
| `APP__SERVER__HOST`                      | Server bind host                    | `127.0.0.1`    |
| `APP__SERVER__PORT`                      | Server bind port                    | `3080`         |
| `APP__SERVER__REQUEST_TIMEOUT_SECS`      | Request timeout in seconds          | `60`           |
| `APP__REGISTRY__HEARTBEAT_INTERVAL_SECS` | Expected service heartbeat interval | `5`            |
| `APP__REGISTRY__SERVICE_TTL_SECS`        | Service registration TTL            | `15`           |
| `APP__APP__NAME`                         | Service name in health response     | `mesh_service` |

Example:

```bash
APP__ENV=development \
APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=10 \
APP__REGISTRY__SERVICE_TTL_SECS=30 \
cargo run
```

The heartbeat interval must be lower than the TTL. For example, a `10` second heartbeat with a `30`
second TTL gives each service roughly three heartbeat opportunities before it is removed from
discovery.

## Development

Format the code:

```bash
cargo fmt
```

Run tests:

```bash
cargo test
```

Run a formatter check:

```bash
cargo fmt --check
```

## Testing

The test suite covers:

- Registering a service.
- Refreshing an existing service registration.
- Finding a compatible semantic version.
- Unregistering a service.
- Cleaning up expired services.
- Rejecting invalid service versions.
- Exercising the HTTP health, register, find, and unregister routes.

Run all tests:

```bash
cargo test
```

## Operational Notes

- Registry state is in memory and is lost when the process restarts.
- There is no multi-node replication.
- There is no authentication or authorization layer yet.
- Service IP detection currently prefers `x-forwarded-for` and falls back to localhost.

These constraints define the first production-facing shape of the service. They keep the registry
straightforward to operate while leaving a clear path toward persistent storage, replication,
authentication, and richer mesh routing layers.

## License

Rusty Mesh is licensed under the [MIT License](LICENSE).
