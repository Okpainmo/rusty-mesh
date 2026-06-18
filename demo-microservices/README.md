# Rusty Mesh Demo Microservices

This folder contains four small services that show Rusty Mesh in action across multiple runtimes:
Rust, Node, and Python. Together they demonstrate registration, heartbeat refresh, load-balanced
discovery, inter-service calls, and shutdown unregistration.

Each microservice:

- binds to a dynamic OS-assigned port
- registers the actual assigned port with the registry after the server starts
- refreshes the registration on a heartbeat interval
- unregisters during graceful shutdown where the runtime supports it

## Services

| Service           | Runtime        | Endpoints                                                        |
| ----------------- | -------------- | ---------------------------------------------------------------- |
| `user-service`    | Rust + Axum    | `GET /health`, `/get-user-feedback`, `/call-catalog-service`     |
| `catalog-service` | Rust + Axum    | `GET /health`, `/get-catalog-feedback`, `/call-cart-service`     |
| `order-service`   | Node + Express | `GET /health`, `/get-order-feedback`, `/call-user-service`       |
| `cart-service`    | Python/FastAPI | `GET /health`, `/get-cart-feedback`, `/call-order-service`       |

All services use dynamic ports by default. Each `/health` response includes the actual assigned
port:

```json
{
  "service": "user-service",
  "version": "1.0.0",
  "status": "ok",
  "port": 45783
}
```

## Shared Configuration

Each service understands these environment variables:

| Variable                  | Default                 | Purpose                              |
| ------------------------- | ----------------------- | ------------------------------------ |
| `MESH_URL`                | `http://127.0.0.1:3080` | Rusty Mesh base URL                  |
| `MESH_TOKEN`              | unset                   | Shared token for protected mesh API  |
| `SERVICE_NAME`            | service-specific        | Service name registered with mesh    |
| `SERVICE_VERSION`         | `1.0.0`                 | Service semantic version             |
| `SERVICE_BIND_HOST`       | `127.0.0.1`             | Host/interface the service binds to  |
| `SERVICE_ADVERTISE_HOST`  | `SERVICE_BIND_HOST`     | Hostname/IP sent to Rusty Mesh       |
| `SERVICE_PORT`            | `0`                     | `0` asks the OS for a free port      |
| `HEARTBEAT_INTERVAL_SECS` | `5`                     | Registration refresh interval        |

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

Registration and heartbeat refresh:

```http
POST /api/v1/mesh/services
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
  "service_port": 45783
}
```

All registry requests send the shared mesh token as:

```http
Authorization: Bearer <MESH_TOKEN>
```

Registration and unregistration also send the reachable service host through:

```http
x-mesh-advertise-host: <SERVICE_ADVERTISE_HOST>
```

Load-balanced discovery:

```http
GET /api/v1/mesh/services/{service_name}/{service_version}
```

Exact-port discovery:

```http
GET /api/v1/mesh/services/{service_name}/{service_version}/{service_port}
```

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

The compose setup starts:

- Rusty Mesh
- two `user-service` instances
- two `catalog-service` instances
- two `order-service` instances
- two `cart-service` instances

Each service instance uses a dynamic internal port and a unique Docker DNS name as its advertised
host, such as `user-service-1` or `cart-service-2`.

List everything registered with the mesh:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://127.0.0.1:3080/api/v1/mesh/services
```

## Call Services Inside Docker

The demo services are intentionally not published to host ports. Call them from inside the Compose
network after discovering their registered host and port from Rusty Mesh.

First, discover a load-balanced service instance from inside the Docker network:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://rusty-mesh:3080/api/v1/mesh/services/user-service/%5E1.0.0
```

The response includes the selected instance's `ip` and `port`. Use those values in the next curl.
For example, if discovery returns `user-service-1` and port `45783`:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  http://user-service-1:45783/health
```

You can call the demo peer endpoint the same way:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  http://user-service-1:45783/call-catalog-service
```

To see every registered service instance and choose a specific one:

```bash
docker run --rm --network rusty-mesh-demo-net curlimages/curl:8.8.0 -sS \
  -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
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

Find a specific service using the dynamic port from either its startup log or the list response:

```bash
curl -H "authorization: Bearer ${MESH_TOKEN:-local-demo-mesh-token}" \
  http://127.0.0.1:3080/api/v1/mesh/services/user-service/%5E1.0.0/<dynamic-port>
```

Stop a service with `Ctrl+C`, then list services again. The service should unregister during
shutdown. If a process is killed abruptly, Rusty Mesh removes it after the configured TTL once it
stops heartbeating.
