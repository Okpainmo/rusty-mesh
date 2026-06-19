import asyncio
import json
import urllib.error
import urllib.request
from urllib.parse import quote
from dataclasses import dataclass


@dataclass(frozen=True)
class MeshRegistryClient:
    mesh_url: str
    mesh_token: str | None
    service_advertise_host: str
    service_name: str
    service_version: str
    service_port: int
    container_id: str | None = None
    external_host: str | None = None
    external_port: int | None = None
    external_scheme: str | None = None

    @property
    def services_url(self) -> str:
        return f"{self.mesh_url.rstrip('/')}/api/v1/mesh/services"

    @property
    def body(self) -> bytes:
        payload = {
            "service_name": self.service_name,
            "service_version": self.service_version,
            "service_port": self.service_port,
        }
        if self.external_host and self.external_port:
            payload["external_host"] = self.external_host
            payload["external_port"] = self.external_port
            payload["external_scheme"] = self.external_scheme or "http"

        return json.dumps(
            payload
        ).encode("utf-8")

    @property
    def auth_headers(self) -> dict[str, str]:
        if self.mesh_token:
            return {"authorization": f"Bearer {self.mesh_token}"}

        return {}

    async def register(self) -> dict:
        return await asyncio.to_thread(self._send, "POST", self.services_url)

    async def heartbeat(self) -> None:
        await asyncio.to_thread(
            self._send, "POST", f"{self.services_url}/heartbeat"
        )

    async def unregister(self) -> None:
        await asyncio.to_thread(self._send, "DELETE", self.services_url)

    async def discover(self, service_name: str) -> dict:
        return await asyncio.to_thread(self._discover, service_name)

    async def call_feedback(self, service: dict, path: str) -> dict:
        return await asyncio.to_thread(self._call_feedback, service, path)

    async def heartbeat_loop(self, heartbeat_interval_secs: int = 5) -> None:
        while True:
            await asyncio.sleep(heartbeat_interval_secs)
            try:
                await self.heartbeat()
            except Exception as error:
                print(
                    f"{self.service_name}:{self.service_version} heartbeat failed: {error}",
                    flush=True,
                )

    def _send(self, method: str, url: str) -> dict:
        request = urllib.request.Request(
            url,
            data=self.body,
            method=method,
            headers={
                "content-type": "application/json",
                "x-mesh-advertise-host": self.service_advertise_host,
                **self.container_headers,
                **self.auth_headers,
            },
        )

        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                if response.status >= 400:
                    raise RuntimeError(
                        f"{method} registry request failed with status {response.status}"
                    )
                payload = json.loads(response.read().decode("utf-8"))
                return payload.get("response") or {}
        except urllib.error.HTTPError as error:
            details = error.read().decode("utf-8")
            raise RuntimeError(
                f"{method} registry request failed with status {error.code}: {details}"
            ) from error

    def _discover(self, service_name: str) -> dict:
        version_requirement = quote("^1.0.0", safe="")
        url = f"{self.services_url}/{service_name}/{version_requirement}"
        request = urllib.request.Request(
            url,
            headers={
                "x-mesh-endpoint-scope": "internal",
                **self.auth_headers,
            },
        )

        with urllib.request.urlopen(request, timeout=5) as response:
            payload = json.loads(response.read().decode("utf-8"))

        service = payload.get("response")
        if service is None:
            raise RuntimeError(f"service '{service_name}' was not registered")

        return service

    def _call_feedback(self, service: dict, path: str) -> dict:
        url = f"http://{service['ip']}:{service['port']}{path}"

        try:
            with urllib.request.urlopen(url, timeout=5) as response:
                return json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as error:
            details = error.read().decode("utf-8")
            raise RuntimeError(
                f"feedback call to {service.get('name')}{path} failed with status {error.code}: {details}"
            ) from error

    @property
    def container_headers(self) -> dict[str, str]:
        if self.container_id:
            return {"x-mesh-container-id": self.container_id}

        return {}
