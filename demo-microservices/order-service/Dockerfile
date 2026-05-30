FROM node:20-bookworm-slim

WORKDIR /app

COPY package.json ./
RUN npm install --omit=dev

COPY src ./src

ENV SERVICE_NAME=order-service
ENV SERVICE_VERSION=1.0.0
ENV SERVICE_BIND_HOST=0.0.0.0
ENV SERVICE_PORT=0
ENV MESH_URL=http://rusty-mesh:3080
ENV HEARTBEAT_INTERVAL_SECS=5

CMD ["npm", "start"]
