# syntax=docker/dockerfile:1.7

FROM node:22-bookworm-slim

WORKDIR /app

COPY package.json ./
RUN --mount=type=cache,id=demo-order-npm,target=/root/.npm \
    npm install --omit=dev --no-audit --no-fund

COPY src ./src

ENV SERVICE_NAME=order-service
ENV SERVICE_VERSION=1.0.0
ENV SERVICE_BIND_HOST=0.0.0.0
ENV SERVICE_PORT=0
ENV MESH_URL=http://rusty-mesh:3080
ENV HEARTBEAT_INTERVAL_SECS=5

EXPOSE 30304

CMD ["npm", "start"]
