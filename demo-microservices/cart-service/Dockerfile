FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py registry_client.py ./

ENV SERVICE_NAME=cart-service
ENV SERVICE_VERSION=1.0.0
ENV SERVICE_BIND_HOST=0.0.0.0
ENV SERVICE_PORT=0
ENV MESH_URL=http://rusty-mesh:3080
ENV HEARTBEAT_INTERVAL_SECS=5

CMD ["python", "main.py"]
