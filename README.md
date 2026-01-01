# Local Grafana Observability Stack

Full observability stack using Docker Compose with Grafana, Alloy, Mimir, Loki, Tempo, Pyroscope, plus a Rust demo application and k6 load testing framework.

## Quick Start

Start the stack:

```bash
docker-compose up -d
```

Wait 30 seconds for services to initialize, then access Grafana at http://localhost:3000.

Verify all services are running:

```bash
docker-compose ps
```

## Load Testing with K6

```bash
./run-k6.sh --list
```
