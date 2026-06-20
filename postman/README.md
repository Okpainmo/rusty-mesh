# Rusty Mesh Postman Collections

This directory contains Postman collections for the Rusty Mesh API and the demo microservices that
integrate with it.

## Collections

| Collection name inside Postman | File                                  | Purpose                                      |
| ------------------------------- | ------------------------------------- | -------------------------------------------- |
| `rusty-mesh__core`              | `mesh-core.postman_collection.json`   | Exercise the core mesh health and registry API |
| `rusty-mesh__demo-microservices` | `demo-microservices.postman_collection.json` | Exercise the demo service health and feedback APIs |

## Import

1. Open Postman.
2. Select `Import`.
3. Choose one or both collection files from this directory.
4. Confirm the import.

Import `mesh-core.postman_collection.json` when you want to test Rusty Mesh directly. Import
`demo-microservices.postman_collection.json` when the demo services are running and you want to call
their public endpoints.

## Mesh Core Collection

The `rusty-mesh__core` collection has these folders:

- `health`
- `registry`

Default variables:

| Variable                      | Purpose                                     | Default                             |
| ----------------------------- | ------------------------------------------- | ----------------------------------- |
| `mesh_core_base_url`          | Rusty Mesh API base URL                     | `http://127.0.0.1:3080/api/v1/mesh` |
| `mesh_token`                  | Shared token for protected registry routes  | `local-demo-mesh-token`             |
| `service_name`                | Service name used in registry examples      | `orders`                            |
| `service_version`             | Exact service version used for registration | `1.2.3`                             |
| `service_version_requirement` | Encoded semver requirement for discovery    | `%5E1.0.0`                          |
| `internal_host`               | Internal service host/IP                    | `10.0.0.20`                         |
| `internal_port`               | Internal service port                       | `3000`                              |
| `external_host`               | Public service host/IP                      | `orders.example.com`                |
| `external_port`               | Public service port                         | `443`                               |
| `external_scheme`             | Public service URL scheme                   | `https`                             |
| `container_id`                | Optional Docker container ID for resolution | empty                               |

Protected registry requests use:

```http
Authorization: Bearer {{mesh_token}}
```

Recommended manual flow:

1. Run `health / Health Check`.
2. Run a registry registration request.
3. Run heartbeat, list, discovery, or exact-port discovery requests.
4. Run unregister when you want to remove the example service instance.

## Demo Microservices Collection

The `rusty-mesh__demo-microservices` collection has these folders:

- `cart`
- `catalog`
- `order`
- `user`

Default variables:

| Variable                   | Purpose                  | Default                  |
| -------------------------- | ------------------------ | ------------------------ |
| `cart_service_base_url`    | Cart demo service URL    | `http://127.0.0.1:30302` |
| `catalog_service_base_url` | Catalog demo service URL | `http://127.0.0.1:30303` |
| `order_service_base_url`   | Order demo service URL   | `http://127.0.0.1:30304` |
| `user_service_base_url`    | User demo service URL    | `http://127.0.0.1:30301` |

Use this collection after starting the demo services. Update the base URL variables if the services
are running on different host ports, especially when Docker assigns dynamic external ports.

## Notes

- The mesh health endpoint is public.
- Registry routes require the shared mesh token unless token enforcement is disabled for isolated
  local debugging.
- Discovery examples use URL-encoded semver requirements. For example, `^1.0.0` is represented as
  `%5E1.0.0`.
- When external endpoint resolution is enabled or explicit external endpoints are registered,
  public mesh responses can include external `ip`, `port`, and `url` plus `internal_ip` and
  `internal_port` for service-to-service calls.
