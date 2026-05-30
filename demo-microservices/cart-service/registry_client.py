import asyncio
import json
import urllib.error
import urllib.request
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
