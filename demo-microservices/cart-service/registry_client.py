import asyncio
import json
import urllib.error
import urllib.request
from urllib.parse import quote
from dataclasses import dataclass


@dataclass(frozen=True)
class MeshRegistryClient:
    mesh_url: str
    service_advertise_host: str
    service_name: str
    service_version: str
    service_port: int

    @property
    def services_url(self) -> str:
        return f"{self.mesh_url.rstrip('/')}/api/v1/mesh/services"

    @property
    def body(self) -> bytes:
        return json.dumps(
            {
                "service_name": self.service_name,
                "service_version": self.service_version,
                "service_port": self.service_port,
            }
        ).encode("utf-8")

    async def register(self) -> None:
        await asyncio.to_thread(self._send, "POST")

    async def unregister(self) -> None:
        await asyncio.to_thread(self._send, "DELETE")

    async def discover(self, service_name: str) -> dict:
        return await asyncio.to_thread(self._discover, service_name)

    async def call_feedback(self, service: dict, path: str) -> dict:
        return await asyncio.to_thread(self._call_feedback, service, path)

    async def heartbeat_loop(self, heartbeat_interval_secs: int = 5) -> None:
        while True:
            await asyncio.sleep(heartbeat_interval_secs)
            try:
                await self.register()
            except Exception as error:
                print(
                    f"{self.service_name}:{self.service_version} heartbeat failed: {error}",
                    flush=True,
                )

    def _send(self, method: str) -> None:
        request = urllib.request.Request(
            self.services_url,
            data=self.body,
            method=method,
            headers={
                "content-type": "application/json",
                "x-forwarded-for": self.service_advertise_host,
            },
        )

        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                if response.status >= 400:
                    raise RuntimeError(
                        f"{method} registration failed with status {response.status}"
                    )
        except urllib.error.HTTPError as error:
            details = error.read().decode("utf-8")
            raise RuntimeError(
                f"{method} registration failed with status {error.code}: {details}"
            ) from error

    def _discover(self, service_name: str) -> dict:
        version_requirement = quote("^1.0.0", safe="")
        url = f"{self.services_url}/{service_name}/{version_requirement}"

        with urllib.request.urlopen(url, timeout=5) as response:
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
