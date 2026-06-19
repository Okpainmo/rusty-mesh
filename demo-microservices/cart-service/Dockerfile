# syntax=docker/dockerfile:1.7

FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt ./
RUN --mount=type=cache,id=demo-cart-pip,target=/root/.cache/pip \
    pip install -r requirements.txt

COPY main.py registry_client.py ./

ENV SERVICE_NAME=cart-service
ENV SERVICE_VERSION=1.0.0
ENV SERVICE_BIND_HOST=0.0.0.0
ENV SERVICE_PORT=0
ENV MESH_URL=http://rusty-mesh:3080
ENV HEARTBEAT_INTERVAL_SECS=5

EXPOSE 30302

CMD ["python", "main.py"]
