function createRegistryClient({
  meshUrl = "http://127.0.0.1:3080",
  meshToken = null,
  serviceAdvertiseHost = "127.0.0.1",
  serviceName,
  serviceVersion,
  servicePort,
  containerId = null,
  externalHost = null,
  externalPort = null,
  externalScheme = "http"
}) {
  const baseUrl = meshUrl.replace(/\/$/, "");
  const body = {
    service_name: serviceName,
    service_version: serviceVersion,
    service_port: servicePort
  };
  if (externalHost && externalPort) {
    body.external_host = externalHost;
    body.external_port = externalPort;
    body.external_scheme = externalScheme || "http";
  }

  function authHeaders() {
    return meshToken ? { authorization: `Bearer ${meshToken}` } : {};
  }

  async function send(method, path = "/api/v1/mesh/services") {
    const response = await fetch(`${baseUrl}${path}`, {
      method,
      headers: {
        "content-type": "application/json",
        "x-mesh-advertise-host": serviceAdvertiseHost,
        ...(containerId ? { "x-mesh-container-id": containerId } : {}),
        ...authHeaders()
      },
      body: JSON.stringify(body)
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`${method} registry request failed: ${response.status} ${text}`);
    }

    const payload = await response.json();
    return payload.response || {};
  }

  async function register() {
    return send("POST");
  }

  async function heartbeat() {
    await send("POST", "/api/v1/mesh/services/heartbeat");
  }

  async function unregister() {
    await send("DELETE");
  }

  async function discover(serviceName) {
    const versionRequirement = encodeURIComponent("^1.0.0");
    const response = await fetch(
      `${baseUrl}/api/v1/mesh/services/${serviceName}/${versionRequirement}`,
      {
        headers: {
          "x-mesh-endpoint-scope": "internal",
          ...authHeaders()
        }
      }
    );

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`service discovery failed: ${response.status} ${text}`);
    }

    const payload = await response.json();
    if (!payload.response) {
      throw new Error(`service '${serviceName}' was not registered`);
    }

    return payload.response;
  }

  async function callFeedback(service, path) {
    const response = await fetch(`http://${service.ip}:${service.port}${path}`);

    if (!response.ok) {
      const text = await response.text();
      throw new Error(
        `feedback call to ${service.name}${path} failed: ${response.status} ${text}`
      );
    }

    return response.json();
  }

  function startHeartbeat(heartbeatIntervalSecs = 5) {
    return setInterval(() => {
      heartbeat().catch((error) => {
        console.error(`${serviceName}:${serviceVersion} heartbeat failed`, error);
      });
    }, heartbeatIntervalSecs * 1000);
  }

  return {
    callFeedback,
    discover,
    heartbeat,
    register,
    unregister,
    startHeartbeat
  };
}

module.exports = {
  createRegistryClient
};
