# Rusty Mesh k6 Traffic Simulation

This directory contains the k6 traffic simulation setup for Rusty Mesh.

## Files

```text
main.js       k6 entrypoint
.env.sample  local k6 environment template
```

Create your local k6 environment file when you need repeatable overrides:

```bash
cp k6-traffic-simulation/.env.sample k6-traffic-simulation/.env
```

Edit `k6-traffic-simulation/.env` with the values for your local run.

## Environment Source

k6 does not automatically load Rusty Mesh's root `.env`, `.env.development`, `.env.staging`, or
`.env.production` files.

Those files belong to the mesh runtime. Consider checking the root [README.md](../README.md) to
learn more about them.

The k6 script reads only k6 process environment variables through `__ENV`. Source the dedicated k6
env file before running:

```bash
set -a
. ./k6-traffic-simulation/.env
set +a
k6 run k6-traffic-simulation/main.js
```

## Run

Start Rusty Mesh first, then run:

```bash
set -a
. ./k6-traffic-simulation/.env
set +a
k6 run k6-traffic-simulation/main.js
```

## Current Flow

The script checks public health, registers one unique service instance per virtual user, and rotates
through heartbeat, list, discovery, internal-scope discovery, exact external-port discovery, and
unregister cleanup.
