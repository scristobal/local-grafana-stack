# Local Grafana Observability Stack

Full Grafana observability stack (Alloy, Mimir, Loki, Tempo and Pyroscope) in Docker Compose. Plus a Rust demo application and k6 load tests.

## Quick Start

Start the stack:

```bash
docker-compose up -d
```

wait and verify all services are running:

```bash
docker-compose ps
```

load Tests:

```bash
./run-k6.sh --list
```

then access Grafana at http://localhost:3000
