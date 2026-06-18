function createRegistryClient({
  meshUrl = "http://127.0.0.1:3080",
  meshToken = null,
  serviceAdvertiseHost = "127.0.0.1",
  serviceName,
  serviceVersion,
  servicePort
}) {
  const baseUrl = meshUrl.replace(/\/$/, "");
  const body = {
    service_name: serviceName,
    service_version: serviceVersion,
    service_port: servicePort
  };

  function authHeaders() {
    return meshToken ? { authorization: `Bearer ${meshToken}` } : {};
  }

  async function send(method) {
    const response = await fetch(`${baseUrl}/api/v1/mesh/services`, {
      method,
      headers: {
        "content-type": "application/json",
        "x-mesh-advertise-host": serviceAdvertiseHost,
        ...authHeaders()
      },
      body: JSON.stringify(body)
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`${method} registration failed: ${response.status} ${text}`);
    }
  }

  async function register() {
    await send("POST");
  }

  async function unregister() {
    await send("DELETE");
  }

  async function discover(serviceName) {
    const versionRequirement = encodeURIComponent("^1.0.0");
    const response = await fetch(
      `${baseUrl}/api/v1/mesh/services/${serviceName}/${versionRequirement}`,
      {
        headers: authHeaders()
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
      register().catch((error) => {
        console.error(`${serviceName}:${serviceVersion} heartbeat failed`, error);
      });
    }, heartbeatIntervalSecs * 1000);
  }

  return {
    callFeedback,
    discover,
    register,
    unregister,
    startHeartbeat
  };
}

module.exports = {
  createRegistryClient
};
