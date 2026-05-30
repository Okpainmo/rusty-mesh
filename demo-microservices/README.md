# Rusty Mesh Demo Microservices

This folder contains four tiny services that demonstrate how real microservices can integrate with
Rusty Mesh for registration, heartbeat refresh, discovery, and shutdown unregistration.

The implementation mirrors the lifecycle pattern from the referenced Node microservices workspace:

- bind to a dynamic OS-assigned port
- register the actual assigned port with the registry after the server starts
- refresh the registration on a heartbeat interval
- unregister during graceful shutdown where the runtime supports it

## Services

| Service           | Runtime        | Endpoint      |
| ----------------- | -------------- | ------------- |
| `user-service`    | Rust + Axum    | `GET /health` |
| `catalog-service` | Rust + Axum    | `GET /health` |
| `order-service`   | Node + Express | `GET /health` |
| `cart-service`    | Python/FastAPI | `GET /health` |

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
| `SERVICE_NAME`            | service-specific        | Service name registered with mesh    |
| `SERVICE_VERSION`         | `1.0.0`                 | Service semantic version             |
| `SERVICE_BIND_HOST`       | `127.0.0.1`             | Host/interface the service binds to  |
| `SERVICE_ADVERTISE_HOST`  | `SERVICE_BIND_HOST`     | Hostname/IP sent to Rusty Mesh       |
| `SERVICE_PORT`            | `0`                     | `0` asks the OS for a free port      |
| `HEARTBEAT_INTERVAL_SECS` | `5`                     | Registration refresh interval        |

Each service is self-contained. The Rust, Node, and Python registry clients live inside the service
folders that use them, so each service can be built and shipped independently.

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

Discovery:

```http
GET /api/v1/mesh/services/{service_name}/{service_version}/{service_port}
```

## Run The Demo

Start Rusty Mesh first from the repository root:

```bash
APP__ENV=development cargo run
```

Then start any demo service in another terminal.

## Run All Services With Docker Compose

From this directory:

```bash
docker compose up --build
```

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
curl http://127.0.0.1:3080/api/v1/mesh/services
```

For ad hoc extra capacity, add another service entry to `compose.yaml` with the same build context
and a unique `SERVICE_ADVERTISE_HOST`, or run that service's Docker image separately with the same
mesh network and a distinct advertised host.

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
  -e SERVICE_BIND_HOST=127.0.0.1 \
  -e SERVICE_ADVERTISE_HOST=127.0.0.1 \
  demo-cart-service
```

## Verify Registration

List registered services:

```bash
curl http://127.0.0.1:3080/api/v1/mesh/services
```

Find a specific service using the dynamic port from either its startup log or the list response:

```bash
curl http://127.0.0.1:3080/api/v1/mesh/services/user-service/%5E1.0.0/<dynamic-port>
```

Stop a service with `Ctrl+C`, then list services again. The service should unregister during
shutdown. If a process is killed abruptly, Rusty Mesh removes it after the configured TTL once it
stops heartbeating.
