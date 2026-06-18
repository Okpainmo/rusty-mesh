# Rusty Mesh

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

Rusty Mesh is a fast in-memory orchestration layer for microservice/distributed systems. It gives
services a focused HTTP control plane for registration, heartbeat-based liveness, semantic version
matching, health checks, and sorted round-robin load balancing across compatible instances.

Built with Rust, Axum, and Tokio, Rusty Mesh is designed for teams that want full control over their
microservices orchestration layer without needing an external or third-party control plane.

> The project's main focus is the mesh/orchestrator service. For easier onboarding, the repository
> also includes a [demo-microservices directory](./demo-microservices) with four demo microservices
> (`order-service - nodejs`, `user-service - rust`, `cart-service - python`, and
> `catalog-service - rust`) that use the mesh and show how engineering teams can integrate
> `rusty-mesh` into microservice or distributed-system builds.

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

- Protect registry routes with a shared mesh token for internal service-to-service access.

- Emit structured JSON logs through `tracing`.

- More...

## Architecture

Rusty Mesh follows a standard Rust service build structure that is intended to be both clean and
maintainable. Below is a breakdown for more context:

```text
src/
  main.rs                  # binary entry point
  lib.rs                   # app state and router assembly
  core/
    router.rs              # versioned route definitions
    controllers/registry/  # HTTP handlers
    services/registry/     # registry domain logic
    structs/               # request/response domain structs
  middlewares/             # logging, mesh-auth, and timeout middleware
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
- `cargo-watch`, optional, for `cargo dev`

## Quick Start

1. Clone the repository:

```bash
git clone https://github.com/okpainmo/rusty-mesh.git
```

2. Copy over the `.env.sample` file to `.env` and update the values as needed:

```bash
cp .env.sample .env
```

> The main `.env` file selects the working/deployment environment with `APP__DEPLOY__ENV`. Depending
> on that value, runtime environment values are loaded from one of the environment-specific files.
> Three environments are supported: `development`, `staging`, and `production`.
>
> In the main `.env`, simply uncomment the preferred environment then leave the rest commented.

3. Copy in the environment-specific `.env` files to the root of the project, and equally update as
   necessary:

```bash
cp .env.development.sample .env.development
cp .env.staging.sample .env.staging
cp .env.production.sample .env.production
```

4. Run the mesh server locally:

```bash
cargo dev
```

> `cargo dev` uses `cargo-watch` to run the server in watch/development mode. If `cargo-watch` is
> not installed, run `cargo install cargo-watch` or use `cargo run`.

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

### Mesh Security

Registry routes are protected by a shared mesh token. Send it as a Bearer token on register,
discover, list, and unregister requests.

For the mesh server itself, set the token through the active environment file:

```bash
APP__SECURITY__MESH_TOKEN=<mesh-token>
```

For demo services, set the matching client-facing token in `demo-microservices/.env`:

```bash
MESH_TOKEN=<mesh-token>
```

The health endpoint remains public so container platforms and load balancers can check liveness
without holding the mesh token.

## Docker

`Rusty Mesh` includes a standalone Docker setup. The mesh service image can be plugged into any
distributed system of choice, then wired into that system's central Compose or orchestration setup.

Build the image:

```bash
docker build -t rusty-mesh .
```

Run the container:

```bash
docker run --rm \
  -p 3080:3080 \
  -e APP__SECURITY__MESH_TOKEN=<add-mesh-token> \
  rusty-mesh
```

The Docker image defaults to:

```text
APP__ENV=production
APP__SERVER__HOST=0.0.0.0
APP__SERVER__PORT=3080
APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=5
APP__REGISTRY__SERVICE_TTL_SECS=15
APP__SECURITY__REQUIRE_MESH_TOKEN=true
```

Override runtime configuration when needed:

```bash
docker run --rm \
  -p 4080:4080 \
  -e APP__SERVER__PORT=4080 \
  -e APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=10 \
  -e APP__REGISTRY__SERVICE_TTL_SECS=30 \
  -e APP__SECURITY__MESH_TOKEN=replace-with-a-strong-token \
  rusty-mesh
```

## Integrating Into A Microservice Project

If you want Rusty Mesh inside an existing microservice workspace, add it as one service in your
parent repository's services directory.

Move into the parent services directory:

```bash
cd <microservice-services-dir>
```

Clone the mesh service with the folder name you want to use in the parent project:

```bash
git clone --single-branch --branch main https://github.com/Okpainmo/rusty-mesh <preferred-mesh-service-name>
cd <preferred-mesh-service-name>
```

Remove the Git history so the mesh service becomes part of the parent project:

```bash
rm -rf .git
```

Keep the runtime files that make Rusty Mesh a service:

```text
src/
config/
Cargo.toml
Cargo.lock
Dockerfile
.dockerignore
.env.sample
.env.development.sample
.env.staging.sample
.env.production.sample
```

Keep `.cargo/config.toml` only if you want the local `cargo dev` watch alias inside the parent
workspace.

Remove repository-only or demo-only files if your parent project already owns those concerns:

```bash
rm -rf \
  .github \
  .husky \
  .codex \
  .vscode \
  demo-microservices \
  CHANGELOG.md \
  CODE_OF_CONDUCT.md \
  CONTRIBUTING.md \
  SECURITY.md \
  commitlint.config.mjs \
  package.json \
  bun.lock \
  prettier.config.mjs
```

You can also remove `README.md` and `LICENSE` if the parent repository already provides project-wide
documentation and licensing. Keep them if the mesh service should remain documented as its own
standalone unit inside the parent repo.

Even inside a broader microservice project, keep Rusty Mesh configured as an independent service.
Set its own `.env` or deployment environment values from the samples, especially
`APP__SECURITY__MESH_TOKEN`. Every internal service that registers with Rusty Mesh should receive
the matching client-side token and use it when calling registry routes:

```http
Authorization: Bearer <shared-mesh-token>
x-mesh-advertise-host: <reachable-service-host>
```

The parent Compose, Kubernetes, or deployment system should start Rusty Mesh before or alongside
services that depend on discovery. `/api/v1/mesh/health` stays public for health checks, while all
`/api/v1/mesh/services...` routes require the shared mesh token.

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

`/health` is public. All `/services` routes require the shared mesh token.

### Register A Service

```bash
curl -X POST \
  -H "authorization: Bearer ${MESH_TOKEN}" \
  -H "x-mesh-advertise-host: 10.0.0.20" \
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
curl -H "authorization: Bearer ${MESH_TOKEN}" \
  http://127.0.0.1:3080/api/v1/mesh/services/orders/%5E1.0.0
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

> **When multiple active instances match, Rusty Mesh sorts the candidates by name, version, IP
> address, and port, then returns them in round-robin order on repeated requests**.

### Find A Service On A Specific Port

The original port-specific lookup remains available when a client needs to target an exact
registered port:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN}" \
  http://127.0.0.1:3080/api/v1/mesh/services/orders/%5E1.0.0/3000
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
curl -H "authorization: Bearer ${MESH_TOKEN}" \
  http://127.0.0.1:3080/api/v1/mesh/services
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
  -H "authorization: Bearer ${MESH_TOKEN}" \
  -H "x-mesh-advertise-host: 10.0.0.20" \
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

Environment files are loaded before configuration is deserialized:

1. `.env`
2. `.env.{APP__DEPLOY__ENV}`, for example `.env.development`

The config loader then resolves application configuration in this order, from lowest to highest
priority:

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
| `APP__SECURITY__REQUIRE_MESH_TOKEN`      | Require token on registry routes    | `true`         |
| `APP__SECURITY__MESH_TOKEN`              | Shared registry access token        | env override   |
| `APP__APP__NAME`                         | Service name in health response     | `mesh_service` |

Example:

```bash
APP__ENV=development \
APP__SECURITY__MESH_TOKEN=replace-with-a-strong-token \
APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=10 \
APP__REGISTRY__SERVICE_TTL_SECS=30 \
cargo run
```

Set `APP__SECURITY__REQUIRE_MESH_TOKEN=false` only for isolated local debugging. Production and
shared environments should keep token enforcement enabled and provide a strong secret through the
active environment file.

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
- Rejecting missing or invalid mesh authentication tokens on registry routes.
- Exercising the HTTP health, register, find, and unregister routes.

Run all tests:

```bash
cargo test
```

## Operational Notes

- Registry state is in memory and is lost when the process restarts.
- There is no multi-node replication.
- Registry routes are protected by a shared mesh token. This is a lightweight service-to-service
  boundary, not per-service identity, RBAC, mTLS, or end-user authorization.
- Keep `APP__SECURITY__REQUIRE_MESH_TOKEN=true` outside isolated local debugging, and set
  `APP__SECURITY__MESH_TOKEN` from the active environment file or deployment secret store.
- The health endpoint is intentionally public for container health checks and load balancers.
- Service advertised-host detection prefers `x-mesh-advertise-host`, falls back to
  `x-forwarded-for`, and finally uses localhost.

These constraints define the first production-facing shape of the service. They keep the registry
straightforward to operate while leaving a clear path toward persistent storage, replication,
service identity, mTLS, and richer mesh routing layers - as the case may be with your needs.

## License

Rusty Mesh is licensed under the [MIT License](LICENSE).
