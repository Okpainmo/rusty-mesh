# Rusty Mesh Demo Microservices

This folder contains four small services that show Rusty Mesh in action across multiple runtimes:
Rust, Node, and Python. Together they demonstrate registration, heartbeat refresh, load-balanced
discovery, inter-service calls, and shutdown unregistration.

Each microservice:

- binds to the configured service port, or a dynamic OS-assigned port when `SERVICE_PORT=0`
- registers the actual assigned port with the registry after the server starts
- refreshes its registered lease on the dedicated heartbeat endpoint
- unregisters during graceful shutdown where the runtime supports it

## Services

| Service           | Runtime        | Endpoints                                                        |
| ----------------- | -------------- | ---------------------------------------------------------------- |
| `user-service`    | Rust + Axum    | `GET /`, `/health`, `/get-user-feedback`, `/call-catalog-service` |
| `catalog-service` | Rust + Axum    | `GET /`, `/health`, `/get-catalog-feedback`, `/call-cart-service` |
| `order-service`   | Node + Express | `GET /`, `/health`, `/get-order-feedback`, `/call-user-service`  |
| `cart-service`    | Python/FastAPI | `GET /`, `/health`, `/get-cart-feedback`, `/call-order-service`  |

Manual service runs use dynamic ports by default. The Compose stack uses stable internal ports by
service type and publishes each instance to a random host port. Each `/health` response includes
both endpoint views:

```json
{
  "service": "user-service",
  "version": "1.0.0",
  "status": "ok",
  "ip": "127.0.0.1",
  "port": 32771,
  "internal_ip": "user-service-1",
  "internal_port": 30301,
  "url": "http://127.0.0.1:32771"
}
```

## Shared Configuration

Each service understands these environment variables:

| Variable                  | Default                 | Purpose                              |
| ------------------------- | ----------------------- | ------------------------------------ |
| `MESH_URL`                | `http://127.0.0.1:3080` | Rusty Mesh base URL                  |
| `MESH_TOKEN`              | unset                   | Shared token for protected mesh API  |
| `MESH_PUBLIC_HOST`        | `127.0.0.1`             | Host used for published demo URLs    |
| `SERVICE_NAME`            | service-specific        | Service name registered with mesh    |
| `SERVICE_VERSION`         | `1.0.0`                 | Service semantic version             |
| `SERVICE_BIND_HOST`       | `127.0.0.1`             | Host/interface the service binds to  |
| `SERVICE_ADVERTISE_HOST`  | `SERVICE_BIND_HOST`     | Hostname/IP sent to Rusty Mesh       |
| `SERVICE_PORT`            | `0`                     | Service port; `0` asks the OS for one |
| `SERVICE_EXTERNAL_HOST`   | unset                   | Optional externally reachable host   |
| `SERVICE_EXTERNAL_PORT`   | unset                   | Optional externally reachable port   |
| `SERVICE_EXTERNAL_SCHEME` | `http`                  | Optional external URL scheme         |
| `HEARTBEAT_INTERVAL_SECS` | `5`                     | Registered lease refresh interval    |

Each service is self-contained. The Rust, Node, and Python registry clients live inside the service
folders that use them, so each service can be built and shipped independently.

## Inter-Service Communication

Each service exposes one explicit feedback endpoint for other services. Each service also exposes
one explicit call endpoint that discovers another service through Rusty Mesh and calls that
service's feedback endpoint. The discovery step uses Rusty Mesh's load-balanced service lookup.

The demo call chain is:

| Caller            | Call endpoint          | Peer feedback endpoint  |
| ----------------- | ---------------------- | ----------------------- |
| `user-service`    | `/call-catalog-service` | `/get-catalog-feedback` |
| `catalog-service` | `/call-cart-service`    | `/get-cart-feedback`    |
| `cart-service`    | `/call-order-service`   | `/get-order-feedback`   |
| `order-service`   | `/call-user-service`    | `/get-user-feedback`    |

The responses use hard-coded dummy data only. There is no database, authentication, or authorization
gate in this demo.

## Registry Contract

Registration:

```http
POST /api/v1/mesh/services
```

Heartbeat refresh:

```http
POST /api/v1/mesh/services/heartbeat
```

Unregistration:

```http
DELETE /api/v1/mesh/services
```

Request body:

```json
{
  "service_name": "catalog-service",
  "service_version": "1.0.0",
  "service_ip": "127.0.0.1",
  "service_port": 45783
}
```

All registry requests send the shared mesh token as:

```http
Authorization: Bearer <MESH_TOKEN>
```

Registration, heartbeat, and unregistration also send the reachable service host through:

```http
x-mesh-advertise-host: <SERVICE_ADVERTISE_HOST>
```

> **Flexible Heartbeat Identity Match**: The heartbeat endpoint matches instances by their **Internal Identity**
> (name, version, IP, port) OR their **External Identity** (mapped host/port). The heartbeat response
> returns the refreshed instance with the published external `ip`, `port`, and `url`, plus
> `internal_ip` and `internal_port` for Docker-network calls.

Load-balanced discovery:

```http
GET /api/v1/mesh/services/{service_name}/{service_version}
```

Exact-port discovery:

```http
GET /api/v1/mesh/services/{service_name}/{service_version}/{service_port}
```

List and discovery responses return the externally reachable endpoint by default:

```json
{
  "name": "user-service",
  "version": "1.0.0",
  "ip": "127.0.0.1",
  "port": 32773,
  "internal_ip": "user-service-1",
  "internal_port": 30301,
  "timestamp": 1781804063,
  "url": "http://127.0.0.1:32773"
}
```

The demo services send `x-mesh-endpoint-scope: internal` on peer discovery calls, so
service-to-service traffic still uses Docker DNS names and stable internal ports.

## Run The Demo

The fastest way to run the complete demo is Docker Compose. The Compose stack already includes
Rusty Mesh, so you do not need to start the mesh service separately.

```bash
cp .env.sample .env
COMPOSE_BAKE=true docker compose up -d --build
```

The first build still downloads and compiles dependencies. Later builds are faster because the
Dockerfiles cache Rust, Node, and Python dependency downloads separately from application source
changes.

The demo Compose stack explicitly sets
`APP__REGISTRY__EXTERNAL_ENDPOINT_RESOLUTION=docker` for Rusty Mesh. That opt-in mode lets Rusty
Mesh inspect each registering container and resolve the random host port mapped to that service's
stable internal port. Outside this demo, keep the resolver as `none` unless the deployment is
intentionally allowing Docker inspection.

The compose setup starts:

- Rusty Mesh
- two `user-service` instances
- two `catalog-service` instances
- two `order-service` instances
- two `cart-service` instances

Each service instance uses a unique Docker DNS name as its advertised host, such as
`user-service-1` or `cart-service-2`. Instances of the same service type share the same internal
container port:

| Service           | Internal port |
| ----------------- | ------------- |
| `user-service`    | `30301`       |
| `cart-service`    | `30302`       |
| `catalog-service` | `30303`       |
| `order-service`   | `30304`       |

List everything registered with the mesh:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://127.0.0.1:3080/api/v1/mesh/services
```

## Call Services From The Host

The demo services publish their stable internal ports to random host ports, so duplicate instances
do not collide. Ask Docker Compose which host port was assigned, then call that port.

```bash
docker compose port user-service-1 30301
```

Example output:

```text
0.0.0.0:32771
```

Call that service instance from the host:

```bash
curl http://127.0.0.1:32771/health
```

Use these internal ports when asking Compose for a host mapping:

```bash
docker compose port user-service-1 30301
docker compose port user-service-2 30301
docker compose port cart-service-1 30302
docker compose port cart-service-2 30302
docker compose port catalog-service-1 30303
docker compose port catalog-service-2 30303
docker compose port order-service-1 30304
docker compose port order-service-2 30304
```

## Call Services Inside Docker

Inside the Compose network, call services through the Docker DNS host and internal port registered
in Rusty Mesh.

First, discover a load-balanced service instance from inside the Docker network:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  -H "x-mesh-endpoint-scope: internal" \
  http://rusty-mesh:3080/api/v1/mesh/services/user-service/%5E1.0.0
```

The internal endpoint-scope header makes the response use the selected instance's Docker DNS name
and internal port. Use those values in the next curl. For example, if discovery returns
`user-service-1` and port `30301`:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  http://user-service-1:30301/health
```

You can call the demo peer endpoint the same way:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  http://user-service-1:30301/call-catalog-service
```

To see every registered service instance and choose a specific one:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  -H "x-mesh-endpoint-scope: internal" \
  http://rusty-mesh:3080/api/v1/mesh/services
```

For ad hoc extra capacity, add another service entry to `compose.yaml` with the same build context
and a unique `SERVICE_ADVERTISE_HOST`, or run that service's Docker image separately with the same
mesh network and a distinct advertised host.

## Run Individual Services Manually

Use this mode only when you want to run one demo service directly from its own folder instead of
running the full Compose stack. In manual mode, start Rusty Mesh first from the repository root:

```bash
APP__DEPLOY__ENV=development \
APP__SECURITY__MESH_TOKEN=local-demo-mesh-token \
cargo run
```

Then start any demo service in another terminal. Make sure the service uses the same mesh token:

```bash
export MESH_URL=http://127.0.0.1:3080
export MESH_TOKEN=local-demo-mesh-token
export SERVICE_BIND_HOST=127.0.0.1
export SERVICE_ADVERTISE_HOST=127.0.0.1
```

### Rust: user-service

```bash
cd demo-microservices/user-service
cargo run
```

Build and run it individually with Docker:

```bash
docker build -t demo-user-service .
docker run --rm --network host \
  -e MESH_URL=http://127.0.0.1:3080 \
  -e MESH_TOKEN=local-demo-mesh-token \
  -e SERVICE_BIND_HOST=127.0.0.1 \
  -e SERVICE_ADVERTISE_HOST=127.0.0.1 \
  demo-user-service
```

### Rust: catalog-service

```bash
cd demo-microservices/catalog-service
cargo run
```

Build and run it individually with Docker:

```bash
docker build -t demo-catalog-service .
docker run --rm --network host \
  -e MESH_URL=http://127.0.0.1:3080 \
  -e MESH_TOKEN=local-demo-mesh-token \
  -e SERVICE_BIND_HOST=127.0.0.1 \
  -e SERVICE_ADVERTISE_HOST=127.0.0.1 \
  demo-catalog-service
```

### Node: order-service

```bash
cd demo-microservices/order-service
npm install
npm start
```

Node `18+` is required because the service uses the built-in `fetch` API.

Build and run it individually with Docker:

```bash
docker build -t demo-order-service .
docker run --rm --network host \
  -e MESH_URL=http://127.0.0.1:3080 \
  -e MESH_TOKEN=local-demo-mesh-token \
  -e SERVICE_BIND_HOST=127.0.0.1 \
  -e SERVICE_ADVERTISE_HOST=127.0.0.1 \
  demo-order-service
```

### Python: cart-service

```bash
cd demo-microservices/cart-service
python -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
python main.py
```

Build and run it individually with Docker:

```bash
docker build -t demo-cart-service .
docker run --rm --network host \
  -e MESH_URL=http://127.0.0.1:3080 \
  -e MESH_TOKEN=local-demo-mesh-token \
  -e SERVICE_BIND_HOST=127.0.0.1 \
  -e SERVICE_ADVERTISE_HOST=127.0.0.1 \
  demo-cart-service
```

## Verify Registration

List registered services:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://127.0.0.1:3080/api/v1/mesh/services
```

Find a specific service using the public port from the list response. For manually run services
without an external endpoint, this is the same dynamic port printed in that service's startup log:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://127.0.0.1:3080/api/v1/mesh/services/user-service/%5E1.0.0/<dynamic-port>
```

Stop a service with `Ctrl+C`, then list services again. The service should unregister during
shutdown. If a process is killed abruptly, Rusty Mesh removes it after the configured TTL once it
stops heartbeating.
