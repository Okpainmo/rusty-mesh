function createRegistryClient({
  meshUrl = "http://127.0.0.1:3080",
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

  async function send(method) {
    const response = await fetch(`${baseUrl}/api/v1/mesh/services`, {
      method,
      headers: {
        "content-type": "application/json",
        "x-forwarded-for": serviceAdvertiseHost
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

  function startHeartbeat(heartbeatIntervalSecs = 5) {
    return setInterval(() => {
      register().catch((error) => {
        console.error(`${serviceName}:${serviceVersion} heartbeat failed`, error);
      });
    }, heartbeatIntervalSecs * 1000);
  }

  return {
    register,
    unregister,
    startHeartbeat
  };
}

module.exports = {
  createRegistryClient
};
