import asyncio
import os
import socket

import uvicorn
from fastapi import FastAPI, HTTPException

from registry_client import MeshRegistryClient


SERVICE_NAME = os.getenv("SERVICE_NAME", "cart-service")
SERVICE_VERSION = os.getenv("SERVICE_VERSION", "1.0.0")
SERVICE_BIND_HOST = os.getenv("SERVICE_BIND_HOST", "127.0.0.1")
SERVICE_ADVERTISE_HOST = os.getenv("SERVICE_ADVERTISE_HOST", SERVICE_BIND_HOST)
SERVICE_PORT = int(os.getenv("SERVICE_PORT", "0"))
MESH_URL = os.getenv("MESH_URL", "http://127.0.0.1:3080")
MESH_TOKEN = os.getenv("MESH_TOKEN", "").strip() or None
HEARTBEAT_INTERVAL_SECS = int(os.getenv("HEARTBEAT_INTERVAL_SECS", "5"))
SERVICE_EXTERNAL_HOST = os.getenv("SERVICE_EXTERNAL_HOST", "").strip() or None
SERVICE_EXTERNAL_PORT = (
    int(os.getenv("SERVICE_EXTERNAL_PORT"))
    if os.getenv("SERVICE_EXTERNAL_PORT", "").strip()
    else None
)
SERVICE_EXTERNAL_SCHEME = os.getenv("SERVICE_EXTERNAL_SCHEME", "http").strip() or "http"

app = FastAPI(title=SERVICE_NAME)
assigned_port = 0
registry_client: MeshRegistryClient | None = None
registered_endpoint: dict = {}


def endpoint_details() -> dict:
    return {
        "ip": registered_endpoint.get("ip", SERVICE_ADVERTISE_HOST),
        "port": registered_endpoint.get("port", assigned_port),
        "internal_ip": registered_endpoint.get("internal_ip", SERVICE_ADVERTISE_HOST),
        "internal_port": registered_endpoint.get("internal_port", assigned_port),
        "url": registered_endpoint.get(
            "url",
            f"http://{SERVICE_ADVERTISE_HOST}:{assigned_port}",
        ),
    }


@app.get("/")
async def welcome():
    return {
        "service": SERVICE_NAME,
        "version": SERVICE_VERSION,
        "status": "ok",
        "message": f"{SERVICE_NAME} is running and registered with Rusty Mesh.",
        "health_url": "/health",
        "feedback_url": "/get-cart-feedback",
        **endpoint_details(),
    }


@app.get("/health")
async def health():
    return {
        "service": SERVICE_NAME,
        "version": SERVICE_VERSION,
        "status": "ok",
        **endpoint_details(),
    }


@app.get("/get-cart-feedback")
async def feedback():
    return {
        "service": SERVICE_NAME,
        "message": "Cart service says the demo cart is ready for checkout",
        **endpoint_details(),
        "data": {
            "cart_id": "cart-1001",
            "items": 3,
            "subtotal": 59.99,
        },
    }


@app.get("/call-order-service")
async def call_peer():
    if registry_client is None:
        raise HTTPException(status_code=503, detail="registry client is not ready")

    called_service = "order-service"

    try:
        peer = await registry_client.discover(called_service)
        peer_response = await registry_client.call_feedback(
            peer,
            "/get-order-feedback",
        )
        return {
            "service": SERVICE_NAME,
            "called_service": called_service,
            **endpoint_details(),
            "peer_response": peer_response,
        }
    except Exception as error:
        raise HTTPException(status_code=502, detail=str(error)) from error


async def main() -> None:
    global assigned_port, registry_client, registered_endpoint

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind((SERVICE_BIND_HOST, SERVICE_PORT))
    sock.listen(socket.SOMAXCONN)
    sock.setblocking(False)
    assigned_port = sock.getsockname()[1]

    registry_client = MeshRegistryClient(
        mesh_url=MESH_URL,
        mesh_token=MESH_TOKEN,
        service_advertise_host=SERVICE_ADVERTISE_HOST,
        service_name=SERVICE_NAME,
        service_version=SERVICE_VERSION,
        service_port=assigned_port,
        container_id=os.getenv("HOSTNAME"),
        external_host=SERVICE_EXTERNAL_HOST,
        external_port=SERVICE_EXTERNAL_PORT,
        external_scheme=SERVICE_EXTERNAL_SCHEME,
    )
    registered_endpoint = await register_until_ready(registry_client)
    heartbeat = asyncio.create_task(
        registry_client.heartbeat_loop(HEARTBEAT_INTERVAL_SECS)
    )

    print(
        f"{SERVICE_NAME}:{SERVICE_VERSION} listening on http://{SERVICE_BIND_HOST}:{assigned_port}",
        flush=True,
    )

    server = uvicorn.Server(uvicorn.Config(app, log_level="info"))

    try:
        await server.serve(sockets=[sock])
    finally:
        heartbeat.cancel()
        try:
            await heartbeat
        except asyncio.CancelledError:
            pass
        await registry_client.unregister()


async def register_until_ready(registry_client: MeshRegistryClient) -> dict:
    while True:
        try:
            return await registry_client.register()
        except Exception as error:
            print(
                f"{SERVICE_NAME}:{SERVICE_VERSION} initial registration failed: {error}",
                flush=True,
            )
            await asyncio.sleep(1)


if __name__ == "__main__":
    asyncio.run(main())
