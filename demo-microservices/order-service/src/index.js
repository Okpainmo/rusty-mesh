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
const heartbeatIntervalSecs = Number.parseInt(
  process.env.HEARTBEAT_INTERVAL_SECS || "5",
  10
);

const app = express();
const server = createServer(app);

let registryClient = null;
let heartbeat = null;
let cleaned = false;

app.get("/health", (req, res) => {
  res.json({
    service: serviceName,
    version: serviceVersion,
    status: "ok",
    port: server.address().port
  });
});

app.get("/get-order-feedback", (req, res) => {
  res.json({
    service: serviceName,
    message: "Order service says the demo order is ready",
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
      await registryClient.register();
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
    serviceAdvertiseHost,
    serviceName,
    serviceVersion,
    servicePort: port
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
