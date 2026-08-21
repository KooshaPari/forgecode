# OTel Collector Deployment Guide for Forgecode

This document provides instructions for deploying the OpenTelemetry Collector in a production environment.

## Prerequisites

- Docker and Docker Compose installed
- Environment variables configured in `.env` (optional, defaults provided)

## Quick Start

1. **Start the Collector**:
   ```bash
   ./deploy/otel-collector.sh up
   ```

2. **Check Status**:
   ```bash
   ./deploy/otel-collector.sh status
   ```

3. **Stop the Collector**:
   ```bash
   ./deploy/otel-collector.sh down
   ```

## Configuration

Edit `deploy/.env` to customize:
- `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP gRPC endpoint (default: `localhost:4317`)
- `JAEGER_URL`: Jaeger UI URL (default: `localhost:16686`)
- `PROMETHEUS_URL`: Prometheus URL (default: `localhost:9090`)

## Systemd Service (Linux)

To install as a service:
1. Copy `deploy/otel-collector.service` to `/etc/systemd/system/`
2. Run `sudo systemctl daemon-reload`
3. Run `sudo systemctl enable --now otel-collector`
