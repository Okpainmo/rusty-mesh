const express = require("express");
const { createServer } = require("http");
const pkg = require("../package.json");
const { createRegistryClient } = require("./registry-client");

const serviceName = process.env.SERVICE_NAME || pkg.name;
const serviceVersion = process.env.SERVICE_VERSION || pkg.version;
const serviceBindHost = process.env.SERVICE_BIND_HOST || "127.0.0.1";
const serviceAdvertiseHost =
  process.env.SERVICE_ADVERTISE_HOST || serviceBindHost;
const requestedPort = Number.parseInt(process.env.SERVICE_PORT || "0", 10);
const meshUrl = process.env.MESH_URL || "http://127.0.0.1:3080";
const meshToken = (process.env.MESH_TOKEN || "").trim() || null;
const serviceExternalHost =
  (process.env.SERVICE_EXTERNAL_HOST || "").trim() || null;
const serviceExternalPort =
  Number.parseInt(process.env.SERVICE_EXTERNAL_PORT || "", 10) || null;
const serviceExternalScheme =
  (process.env.SERVICE_EXTERNAL_SCHEME || "http").trim() || "http";
const heartbeatIntervalSecs = Number.parseInt(
  process.env.HEARTBEAT_INTERVAL_SECS || "5",
  10
);

const app = express();
const server = createServer(app);

let registryClient = null;
let heartbeat = null;
let cleaned = false;
let registeredEndpoint = {};

function endpointDetails() {
  const internalIp = registeredEndpoint.internal_ip || serviceAdvertiseHost;
  const internalPort = registeredEndpoint.internal_port || server.address()?.port || requestedPort;
  const ip = registeredEndpoint.ip || internalIp;
  const port = registeredEndpoint.port || internalPort;

  return {
    ip,
    port,
    internal_ip: internalIp,
    internal_port: internalPort,
    url: registeredEndpoint.url || `http://${ip}:${port}`
  };
}

app.get("/", (req, res) => {
  res.json({
    service: serviceName,
    version: serviceVersion,
    status: "ok",
    message: `${serviceName} is running and registered with Rusty Mesh.`,
    health_url: "/health",
    feedback_url: "/get-order-feedback",
    ...endpointDetails()
  });
});

app.get("/health", (req, res) => {
  res.json({
    service: serviceName,
    version: serviceVersion,
    status: "ok",
    ...endpointDetails()
  });
});

app.get("/get-order-feedback", (req, res) => {
  res.json({
    service: serviceName,
    message: "Order service says the demo order is ready",
    ...endpointDetails(),
    data: {
      order_id: "order-1001",
      status: "ready",
      total: 59.99
    }
  });
});

app.get("/call-user-service", async (req, res) => {
  const calledService = "user-service";

  try {
    const peer = await registryClient.discover(calledService);
    const peerResponse = await registryClient.callFeedback(
      peer,
      "/get-user-feedback"
    );

    res.json({
      service: serviceName,
      called_service: calledService,
      ...endpointDetails(),
      peer_response: peerResponse
    });
  } catch (error) {
    res.status(502).json({
      service: serviceName,
      error: error.message
    });
  }
});

async function cleanup() {
  if (cleaned) {
    return;
  }

  cleaned = true;
  if (heartbeat) {
    clearInterval(heartbeat);
  }
  
  if (registryClient) {
    await registryClient.unregister().catch((error) => {
      console.error(`${serviceName}:${serviceVersion} unregister failed`, error);
    });
  }
}

async function registerUntilReady() {
  for (;;) {
    try {
      registeredEndpoint = await registryClient.register();
      return;
    } catch (error) {
      console.error(`${serviceName}:${serviceVersion} initial registration failed`, error);
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
}

server.listen(requestedPort, serviceBindHost, async () => {
  const { port } = server.address();

  registryClient = createRegistryClient({
    meshUrl,
    meshToken,
    serviceAdvertiseHost,
    serviceName,
    serviceVersion,
    servicePort: port,
    containerId: process.env.HOSTNAME || null,
    externalHost: serviceExternalHost,
    externalPort: serviceExternalPort,
    externalScheme: serviceExternalScheme
  });

  await registerUntilReady();
  heartbeat = registryClient.startHeartbeat(heartbeatIntervalSecs);

  console.info(
    `${serviceName}:${serviceVersion} listening on http://${serviceBindHost}:${port}`
  );
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, async () => {
    await cleanup();
    server.close(() => process.exit(0));
  });
}

process.on("uncaughtException", async (error) => {
  console.error(error);
  await cleanup();
  process.exit(1);
});
