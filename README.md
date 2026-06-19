# Rusty Mesh

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

Rusty Mesh is a fast in-memory orchestration layer for microservice/distributed systems. It provides
a focused HTTP control plane for service registration, heartbeat-based liveness, semantic version
matching, health checks, and sorted round-robin load balancing across compatible instances.

Built with Rust, Axum, and Tokio, Rusty Mesh is designed for teams that want full control over their
microservices orchestration layer without needing an external or third-party control plane.

> The project's main focus is the mesh/orchestrator layer. But for easier onboarding, the repository
> also includes a [demo-microservices directory](./demo-microservices) with four demo microservices
> (`order-service - nodejs`, `user-service - rust`, `cart-service - python`, and
> `catalog-service - rust`) that use the mesh and show how engineering teams can integrate
> `rusty-mesh` into their microservice builds.

## Table Of Contents

- [Core Capabilities](#core-capabilities)
- [Architecture](#architecture)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
  - [Environment Selection](#environment-selection)
  - [Mesh Security](#mesh-security)
- [External Endpoint Resolution](#external-endpoint-resolution)
  - [Explicit External Registration](#explicit-external-registration)
  - [Docker Resolver](#docker-resolver)
  - [Docker Socket Security](#docker-socket-security)
- [Docker](#docker)
- [Integrating Into A Microservice Project](#integrating-into-a-microservice-project)
- [API Reference](#api-reference)
  - [Postman Collections](#postman-collections)
  - [Register A Service](#register-a-service)
  - [Find A Service With Load Balancing](#find-a-service-with-load-balancing)
  - [Find A Service On A Specific External Port](#find-a-service-on-a-specific-external-port)
  - [List Services](#list-services)
  - [Unregister A Service](#unregister-a-service)
- [Service Discovery Semantics](#service-discovery-semantics)
- [Configuration](#configuration)
- [Development](#development)
- [Testing](#testing)
- [Operational Notes](#operational-notes)
- [License](#license)

## Core Capabilities

- Register service instances by name, semantic version, IP address, and port.

- Refresh service leases through the dedicated heartbeat endpoint or by re-registering.

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
- optional external endpoint metadata: `external_ip`, `external_port`, and `external_scheme`

Expired entries are removed during register, heartbeat, unregister, find, and list operations.
**Load-balanced discovery sorts matching instances by name, version, registered internal IP address,
and registered internal port, then returns the next instance using a round-robin cursor for that
service and version requirement**.

Rusty Mesh treats service registration as a heartbeat-driven contract. Registered services should
refresh their lease before `registry.service_ttl_secs` elapses. Startup validation enforces that
`registry.heartbeat_interval_secs` is lower than `registry.service_ttl_secs`, so the configured
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

### Environment Selection

> The `.env` file exists solely to select the working/deployment environment with
> `APP__DEPLOY__ENV`. Depending on that value, runtime environment values are loaded from one of the
> environment-specific files. Three environments are supported: `development`, `staging`, and
> `production`.
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

## External Endpoint Resolution

By default, Rusty Mesh only needs the internal endpoint a service registers with. It can also store
an externally reachable endpoint for operator-facing discovery responses. **This matters when
multiple instances of the same service share one stable internal port while the deployment platform
assigns dynamic host ports for each replica, or when host-side access to each service instance is
required.** Continue reading to [learn more in the docker resolver section](#docker-resolver).

External endpoint resolution is explicit and controlled by:

```bash
APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=<mode>
```

Available modes:

| Mode     | Behavior                                                                                        | Production posture                        |
| -------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `none`   | Do not inspect Docker or any platform API. Use explicit external endpoint fields when provided. | Default and safest baseline               |
| `docker` | Inspect the registering Docker container to resolve the host port mapped to its internal port.  | Opt-in; requires a trusted control-plane deployment |

The default is `none`.

Resolution priority is:

1. Use explicit external endpoint fields from the registration request.

2. If no explicit endpoint fields are supplied and the resolver mode is `docker`, inspect Docker.

3. On discovery response, if no external endpoint can be resolved, fall back to the internal
   registered endpoint.

### Explicit External Registration

In `none` mode, Rusty Mesh does not inspect Docker or any platform API. This is the production-safe
baseline when the deployment platform, gateway, sidecar, or service config already knows the
externally reachable host and port.

The service sends those values during registration:

```json
{
  "service_name": "cart-service",
  "service_version": "1.0.0",
  "service_port": 30302,
  "external_host": "cart.example.com",
  "external_port": 443,
  "external_scheme": "https"
}
```

`external_host` and `external_port` must be provided together. `external_scheme` is optional and
defaults to `http`; when provided, it must be `http` or `https`.

If a service does not provide an explicit external endpoint, public discovery falls back to the
registered internal endpoint.

### Docker Resolver

The Docker resolver exists for dynamic-port deployments where each scaled instance shares the same
internal container port, but Docker assigns a different host port. This is common in local Compose
stacks and simple Docker-based demos:

```yaml
ports:
  - "30302"
```

In that setup, the service knows it is listening on `30302` inside the container, but it does not
automatically know which random host port Docker assigned. Docker Engine is the source of truth for
that mapping. Rusty Mesh can ask Docker only when this mode is explicitly enabled:

```bash
APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=docker
```

The Docker resolver flow is:

1. A service registers with its internal service port, for example `30302`.
2. The service client sends its Docker container id in the `x-mesh-container-id` header.
3. Rusty Mesh inspects that container through the Docker Engine socket.
4. It reads `NetworkSettings.Ports["30302/tcp"][0].HostPort` from the Docker inspect response.
5. It stores that host port as the service's external port.
6. Public list and discovery responses return `ip`, `port`, and `url` using the external endpoint,
   plus `internal_ip` and `internal_port` for the in-network endpoint.

For example, if Docker maps `cart-service-1:30302` to host port `63858`, the public registry
response becomes:

```json
{
  "name": "cart-service",
  "version": "1.0.0",
  "ip": "127.0.0.1",
  "port": 63858,
  "internal_ip": "cart-service-1",
  "internal_port": 30302,
  "timestamp": 1790745600,
  "url": "http://127.0.0.1:63858"
}
```

Internal service-to-service discovery can still request the Docker-network endpoint by sending:

```http
x-mesh-endpoint-scope: internal
```

That keeps peer calls inside the Docker network while letting operators curl the externally
published service URL from the host.

### Docker Socket Security

Mounting `/var/run/docker.sock` into a container is powerful. Even when mounted read-only at the
filesystem level, the Docker Engine API can expose sensitive container, network, image, and runtime
metadata. Treat Docker socket access as privileged infrastructure access, not as a harmless
read-only helper.

Rusty Mesh uses the socket only when `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=docker`. In that
mode it inspects the registering container and resolves the host port mapped to the registered
internal port.

Using the Docker resolver in production can be valid when Rusty Mesh is deployed as a trusted
control-plane component with strong network isolation, strict mesh-token handling, patched hosts,
centralized monitoring, and restricted operator access. In that model, Docker inspection is an
intentional control-plane privilege that lets Rusty Mesh resolve runtime endpoint metadata for the
services it manages.

Keep these guardrails in place when enabling it:

- Enable `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=docker` only for deployments that explicitly
  trust Rusty Mesh with Docker inspection.

- Do not log or return raw Docker inspect payloads.

- Keep registry routes private and protected by a strong `APP__SECURITY__MESH_TOKEN`.

- Treat `x-mesh-container-id` as privileged input from trusted service clients.

- Restrict the Docker host, firewall rules, service network, and operator access so the mesh server
  cannot be reached from untrusted networks.

You might prefer one of these alternatives when Rusty Mesh should not hold Docker-level privilege:

- Keep `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=none` and have services register their
  externally reachable host and port directly when the deployment platform provides them.

- Put Rusty Mesh behind the same orchestrator or service-discovery layer as the services and use
  internal endpoints only.

- Use a restricted sidecar or proxy that exposes only the small inspect data Rusty Mesh needs.

- In Kubernetes(if Rusty Mesh ever applies), use pod/service metadata from the Kubernetes API
  instead of Docker socket access.

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
APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=none
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

If you want Rusty Mesh inside an existing microservice workspace, simply add it as a standalone
service in your parent repository's microservices directory.

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
| GET    | `/`                                                         | Show a short mesh welcome/status message                   |
| GET    | `/health`                                                   | Check service health                                       |
| GET    | `/services`                                                 | List active service instances                              |
| POST   | `/services`                                                 | Register or refresh an instance                            |
| POST   | `/services/heartbeat`                                       | Update instance lease TTL                                  |
| DELETE | `/services`                                                 | Unregister an instance                                     |
| GET    | `/services/{service_name}/{service_version}`                | Find a compatible instance with round-robin load balancing |
| GET    | `/services/{service_name}/{service_version}/{service_port}` | Find a compatible instance on a specific port              |

`/health` is public. All `/services` routes require the shared mesh token.

## Postman Collections

Postman collections are available in the [postman](postman) directory. Import them separately when
you want to exercise the mesh API or the demo services without rebuilding requests by hand.

| Collection           | File                                                                                                     | Folders                            |
| -------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `mesh-core`          | [postman/mesh-core.postman_collection.json](postman/mesh-core.postman_collection.json)                   | `health`, `registry`               |
| `demo-microservices` | [postman/demo-microservices.postman_collection.json](postman/demo-microservices.postman_collection.json) | `cart`, `catalog`, `order`, `user` |

The `mesh-core` collection contains:

- `health`: welcome and health-check endpoints.
- `registry`: registration, heartbeat, list, discovery, internal-scope discovery, and unregistration
  endpoints.

Before running `mesh-core` requests, update these collection variables for your environment:

| Variable                      | Purpose                                     | Default                             |
| ----------------------------- | ------------------------------------------- | ----------------------------------- |
| `mesh_core_base_url`          | Rusty Mesh API base URL                     | `http://127.0.0.1:3080/api/v1/mesh` |
| `mesh_token`                  | Shared token for protected registry routes  | `local-demo-mesh-token`             |
| `service_name`                | Service name used in registry examples      | `orders`                            |
| `service_version`             | Exact service version used for registration | `1.2.3`                             |
| `service_version_requirement` | Encoded semver requirement for discovery    | `%5E1.0.0`                          |
| `internal_host`               | Internal service host/IP                    | `10.0.0.20`                         |
| `internal_port`               | Internal service port                       | `3000`                              |
| `external_host`               | Public service host/IP                      | `orders.example.com`                |
| `external_port`               | Public service port                         | `443`                               |
| `external_scheme`             | Public service URL scheme                   | `https`                             |
| `container_id`                | Optional Docker container id for resolution | empty                               |

The `demo-microservices` collection contains:

- `cart`: welcome, health, cart feedback, and order-service call endpoints.
- `catalog`: welcome, health, catalog feedback, and cart-service call endpoints.
- `order`: welcome, health, order feedback, and user-service call endpoints.
- `user`: welcome, health, user feedback, and catalog-service call endpoints.

Before running `demo-microservices` requests, update these collection variables:

| Variable                   | Purpose                  | Default                  |
| -------------------------- | ------------------------ | ------------------------ |
| `cart_service_base_url`    | Cart demo service URL    | `http://127.0.0.1:30302` |
| `catalog_service_base_url` | Catalog demo service URL | `http://127.0.0.1:30303` |
| `order_service_base_url`   | Order demo service URL   | `http://127.0.0.1:30304` |
| `user_service_base_url`    | User demo service URL    | `http://127.0.0.1:30301` |

### Register A Service

```bash
curl -X POST \
  -H "authorization: Bearer ${MESH_TOKEN}" \
  -H "x-mesh-advertise-host: 10.0.0.20" \
  -H "content-type: application/json" \
  -d '{
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_ip": "10.0.0.20",
    "service_port": 3000,
    "external_host": "orders.example.com",
    "external_port": 443,
    "external_scheme": "https"
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
    "ip": "orders.example.com",
    "port": 443,
    "internal_ip": "10.0.0.20",
    "internal_port": 3000,
    "url": "https://orders.example.com:443"
  },
  "error": null
}
```

The external fields are optional. Use them when the service already knows its externally reachable
endpoint. If `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=docker` is enabled and no explicit
external endpoint is supplied, Rusty Mesh tries to resolve the external port from Docker instead.

Calling the same endpoint again refreshes the service timestamp. Alternatively, you can use the
explicit heartbeat endpoint:

```bash
curl -X POST \
  -H "authorization: Bearer ${MESH_TOKEN}" \
  -H "content-type: application/json" \
  -d '{
    "service_name": "orders",
    "service_version": "1.2.3",
    "service_ip": "10.0.0.20",
    "service_port": 3000
  }' \
  http://127.0.0.1:3080/api/v1/mesh/services/heartbeat
```

Response:

```json
{
  "response_message": "Service heartbeat refreshed successfully",
  "response": {
    "service_name": "orders",
    "service_version": "1.2.3",
    "ip": "orders.example.com",
    "port": 443,
    "internal_ip": "10.0.0.20",
    "internal_port": 3000,
    "url": "https://orders.example.com:443"
  },
  "error": null
}
```

> **Heartbeat Identity Matching**: The heartbeat endpoint supports flexible lookup. It will match
> the instance by its **Internal Identity** (name, version, IP, and port) or by its **External
> Identity** (matching against the `external_host` and `external_port` used during registration).
> The heartbeat response returns the refreshed instance with the same external `ip`, `port`, and
> `url` fields used by registration and public discovery, plus `internal_ip` and `internal_port` for
> in-network calls.

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
    "ip": "orders.example.com",
    "port": 443,
    "internal_ip": "10.0.0.20",
    "internal_port": 3000,
    "timestamp": 1790745600,
    "url": "https://orders.example.com:443"
  },
  "error": null
}
```

For service-to-service calls that should stay on the private network, send
`x-mesh-endpoint-scope: internal`. In that mode, discovery returns the raw registered instance, so
`ip` and `port` are the internal registered endpoint and any resolved external endpoint remains in
the optional `external_ip`, `external_port`, and `external_scheme` fields.

> **When multiple active instances match, Rusty Mesh sorts the candidates by name, version, IP
> address, and port using the registered internal endpoint, then returns them in round-robin order
> on repeated requests**.

### Find A Service On A Specific External Port

The port-specific lookup targets the external port when a service has an external endpoint. This is
important when multiple replicas share the same internal port but publish to different host ports.
If a service has no external endpoint, Rusty Mesh falls back to matching its internal registered
port.

```bash
curl -H "authorization: Bearer ${MESH_TOKEN}" \
  http://127.0.0.1:3080/api/v1/mesh/services/orders/%5E1.0.0/443
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
        "ip": "orders.example.com",
        "port": 443,
        "internal_ip": "10.0.0.20",
        "internal_port": 3000,
        "timestamp": 1790745600,
        "url": "https://orders.example.com:443"
      }
    ],
    "services-count": 1
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
    "ip": "orders.example.com",
    "port": 443,
    "internal_ip": "10.0.0.20",
    "internal_port": 3000,
    "url": "https://orders.example.com:443"
  },
  "error": null
}
```

If the service instance is not registered, Rusty Mesh returns HTTP `404`:

```json
{
  "response_message": "Service instance is not registered.",
  "response": null,
  "error": {
    "message": "Service instance is not registered.",
    "code": "SERVICE_NOT_REGISTERED"
  }
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

All environment variables files are loaded before configuration is deserialized:

1. `.env` - in which the current working environment is selected -
   [as earlier described](#environment-selection).

2. `.env.{APP__DEPLOY__ENV}`, for example `.env.development`

The config loader then resolves application configuration in this order, from lowest to highest
priority:

1. `config/base.toml`
2. `config/{APP__ENV}.toml`
3. `config/local.toml`, if present
4. `APP__` environment variables

Default values live in [config/base.toml](config/base.toml).

| Variable                                      | Purpose                                         | Default        |
| --------------------------------------------- | ----------------------------------------------- | -------------- |
| `APP__ENV`                                    | Selects the config environment                  | required       |
| `APP__SERVER__HOST`                           | Server bind host                                | `127.0.0.1`    |
| `APP__SERVER__PORT`                           | Server bind port                                | `3080`         |
| `APP__SERVER__REQUEST_TIMEOUT_SECS`           | Request timeout in seconds                      | `60`           |
| `APP__REGISTRY__HEARTBEAT_INTERVAL_SECS`      | Expected service heartbeat interval             | `5`            |
| `APP__REGISTRY__SERVICE_TTL_SECS`             | Service registration TTL                        | `15`           |
| `APP__REGISTRY__PUBLIC_HOST`                  | Host used when Docker publishes to wildcard IPs | empty          |
| `APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION` | External endpoint resolver: `none` or `docker`  | `none`         |
| `APP__SECURITY__REQUIRE_MESH_TOKEN`           | Require token on registry routes                | `true`         |
| `APP__SECURITY__MESH_TOKEN`                   | Shared registry access token                    | env override   |
| `APP__APP__NAME`                              | Service name in health response                 | `mesh_service` |

Example:

```bash
APP__ENV=development \
APP__SECURITY__MESH_TOKEN=replace-with-a-strong-token \
APP__REGISTRY__HEARTBEAT_INTERVAL_SECS=10 \
APP__REGISTRY__SERVICE_TTL_SECS=30 \
APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=none \
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
- Refreshing an existing service lease through heartbeat.
- Finding a compatible semantic version.
- Unregistering a service.
- Returning `404` when unregistering a service instance that does not exist.
- Cleaning up expired services.
- Rejecting invalid service versions.
- Rejecting missing or invalid mesh authentication tokens on registry routes.
- Exercising the HTTP health, register, heartbeat, find, and unregister routes.

Run all tests:

```bash
cargo test
```

## Operational Notes

- **Registry state is in-memory** and is lost when the process restarts. This design is intentional
  as it keeps the system simple, focused, highly performant, and flexible enough to permit
  customized scaling/extending. A reasonable area(in terms of extending the current system) that
  comes to mind is adding data persistence via a queue(background jobs), which will be triggered
  after every successful service registration/de-registration.

- **There is no multi-node replication**. As it stands, deploying multiple instances of Rusty Mesh
  remains to be handled as you see fit. A very good way to utilize Rusty Mesh, will be to treat it's
  deployment as `pods` - similar to Kubernetes. With each pod consisting of a Rusty Mesh copy, and
  `podlets` - other non-mesh services. Of course, this does not prevent individual service scaling
  for cases where only specific `podlets` should be added for load balancing.

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
