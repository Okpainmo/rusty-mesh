import { check, group, sleep } from "k6";
import http from "k6/http";
import exec from "k6/execution";

export const options = {
  scenarios: {
    registry_flow: {
      executor: "ramping-vus",
      stages: [
        { duration: __ENV.K6_WARMUP_DURATION || "30s", target: Number(__ENV.K6_WARMUP_VUS || 5) },
        { duration: __ENV.K6_STEADY_DURATION || "2m", target: Number(__ENV.K6_TARGET_VUS || 30) },
        { duration: __ENV.K6_COOLDOWN_DURATION || "30s", target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_failed: [`rate<${__ENV.K6_MAX_FAILURE_RATE || 0.05}`],
    http_req_duration: [`p(95)<${__ENV.K6_P95_MS || 500}`],
  },
};

const baseUrl = (__ENV.MESH_BASE_URL || "http://127.0.0.1:3080/api/v1/mesh").replace(/\/$/, "");
const meshToken = __ENV.MESH_TOKEN || "local-demo-mesh-token";

function authHeaders(extra = {}) {
  return {
    headers: {
      Authorization: `Bearer ${meshToken}`,
      "Content-Type": "application/json",
      ...extra,
    },
  };
}

function serviceIdentity() {
  const vu = exec.vu.idInTest;
  const port = 3000 + vu;

  return {
    service_name: `${__ENV.K6_SERVICE_PREFIX || "k6-orders"}-${vu}`,
    service_version: __ENV.K6_SERVICE_VERSION || "1.2.3",
    service_ip: __ENV.K6_INTERNAL_HOST || `10.0.0.${20 + vu}`,
    service_port: port,
    external_host: __ENV.K6_EXTERNAL_HOST || "127.0.0.1",
    external_port: Number(__ENV.K6_EXTERNAL_PORT || 43000 + vu),
    external_scheme: __ENV.K6_EXTERNAL_SCHEME || "http",
  };
}

function registerService(service) {
  return http.post(
    `${baseUrl}/services`,
    JSON.stringify(service),
    authHeaders({
      "x-mesh-advertise-host": service.service_ip,
    }),
  );
}

export function setup() {
  const response = http.get(`${baseUrl}/health`);
  check(response, {
    "mesh health is public": (res) => res.status === 200,
  });
}

export default function () {
  const service = serviceIdentity();
  const discoveryVersion = encodeURIComponent(__ENV.K6_SERVICE_VERSION_REQUIREMENT || "^1.0.0");

  group("mesh registry API", () => {
    const registerResponse = registerService(service);

    check(registerResponse, {
      "register status is 200": (res) => res.status === 200,
      "register returns endpoint": (res) => {
        try {
          return Boolean(res.json("response.url"));
        } catch (_) {
          return false;
        }
      },
    });

    const operation = exec.scenario.iterationInTest % 5;

    if (operation === 0) {
      const heartbeatResponse = http.post(
        `${baseUrl}/services/heartbeat`,
        JSON.stringify({
          service_name: service.service_name,
          service_version: service.service_version,
          service_ip: service.service_ip,
          service_port: service.service_port,
        }),
        authHeaders({ "x-mesh-advertise-host": service.service_ip }),
      );

      check(heartbeatResponse, {
        "heartbeat status is 200": (res) => res.status === 200,
      });
    } else if (operation === 1) {
      const listResponse = http.get(`${baseUrl}/services`, authHeaders());

      check(listResponse, {
        "list status is 200": (res) => res.status === 200,
      });
    } else if (operation === 2) {
      const findResponse = http.get(
        `${baseUrl}/services/${service.service_name}/${discoveryVersion}`,
        authHeaders(),
      );

      check(findResponse, {
        "discovery status is 200": (res) => res.status === 200,
      });
    } else if (operation === 3) {
      const exactPortResponse = http.get(
        `${baseUrl}/services/${service.service_name}/${discoveryVersion}/${service.external_port}`,
        authHeaders(),
      );

      check(exactPortResponse, {
        "exact external-port discovery status is 200": (res) => res.status === 200,
      });
    } else {
      const internalDiscoveryResponse = http.get(
        `${baseUrl}/services/${service.service_name}/${discoveryVersion}`,
        authHeaders({ "x-mesh-endpoint-scope": "internal" }),
      );

      check(internalDiscoveryResponse, {
        "internal discovery status is 200": (res) => res.status === 200,
      });
    }
  });

  sleep(Number(__ENV.K6_SLEEP_SECONDS || 1));
}

export function teardown() {
  const vuCount = Number(__ENV.K6_TARGET_VUS || 30);

  for (let vu = 1; vu <= vuCount; vu += 1) {
    const port = 3000 + vu;
    const service = {
      service_name: `${__ENV.K6_SERVICE_PREFIX || "k6-orders"}-${vu}`,
      service_version: __ENV.K6_SERVICE_VERSION || "1.2.3",
      service_ip: __ENV.K6_INTERNAL_HOST || `10.0.0.${20 + vu}`,
      service_port: port,
    };

    http.del(
      `${baseUrl}/services`,
      JSON.stringify(service),
      authHeaders({
        "x-mesh-advertise-host": service.service_ip,
      }),
    );
  }
}
