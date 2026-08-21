#!/usr/bin/env bash
# forgecode OTel Collector - Production Deployment Script
# Usage:
#   ./deploy.sh [up|down|logs|status|restart]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.prod.yml"
ENV_FILE="${SCRIPT_DIR}/.env"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()   { echo -e "${GREEN}[forgecode-otel]${NC} $*"; }
warn()  { echo -e "${YELLOW}[forgecode-otel]${NC} $*"; }
error() { echo -e "${RED}[forgecode-otel]${NC} $*" >&2; }

check_deps() {
    local missing=()
    for cmd in docker; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing required tools: ${missing[*]}"
        exit 1
    fi
    # Check for docker compose (v2 plugin)
    if ! docker compose version &>/dev/null 2>&1; then
        if command -v docker-compose &>/dev/null; then
            COMPOSE_CMD="docker-compose"
        else
            error "docker compose (v2) or docker-compose is required"
            exit 1
        fi
    else
        COMPOSE_CMD="docker compose"
    fi
}

ensure_env() {
    if [[ ! -f "$ENV_FILE" ]]; then
        warn ".env file not found. Copying from .env.example"
        cp "${SCRIPT_DIR}/.env.example" "$ENV_FILE"
        warn "Edit ${ENV_FILE} with your production settings before starting."
        exit 1
    fi
}

cmd_up() {
    ensure_env
    log "Starting forgecode OTel production stack..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d
    log "Stack started. Services:"
    $COMPOSE_CMD -f "$COMPOSE_FILE" ps
    echo ""
    log "Endpoints:"
    log "  Collector OTLP gRPC:  localhost:${OTEL_GRPC_PORT:-4317}"
    log "  Collector OTLP HTTP:  localhost:${OTEL_HTTP_PORT:-4318}"
    log "  Jaeger UI:            http://localhost:${JAEGER_UI_PORT:-16686}"
    log "  Prometheus:           http://localhost:${PROMETHEUS_PORT:-9090}"
    log "  Grafana:              http://localhost:${GRAFANA_PORT:-3000}"
    log "  Collector Health:     http://localhost:13133"
    log "  Collector zPages:     http://localhost:55679"
}

cmd_down() {
    log "Stopping forgecode OTel production stack..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down
    log "Stack stopped."
}

cmd_logs() {
    $COMPOSE_CMD -f "$COMPOSE_FILE" --env-file "$ENV_FILE" logs -f --tail=100 "$@"
}

cmd_status() {
    log "Service status:"
    $COMPOSE_CMD -f "$COMPOSE_FILE" ps
    echo ""
    log "Health check:"
    if command -v curl &>/dev/null; then
        curl -sf http://localhost:13133 2>/dev/null && echo "Collector: healthy" || warn "Collector: unhealthy"
    else
        warn "curl not installed, skipping health check"
    fi
}

cmd_restart() {
    log "Restarting forgecode OTel production stack..."
    cmd_down
    cmd_up
}

# Main
check_deps
cd "$SCRIPT_DIR"

case "${1:-up}" in
    up)      cmd_up ;;
    down)    cmd_down ;;
    logs)    shift; cmd_logs "$@" ;;
    status)  cmd_status ;;
    restart) cmd_restart ;;
    *)
        echo "Usage: $0 {up|down|logs|status|restart}"
        exit 1
        ;;
esac
