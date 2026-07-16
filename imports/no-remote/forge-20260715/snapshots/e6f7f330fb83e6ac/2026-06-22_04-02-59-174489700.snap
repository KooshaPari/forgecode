"""FastAPI entrypoint for Tracera REST APIs."""

from __future__ import annotations

from fastapi import FastAPI
from tracertm.api.middleware.request_id import RequestIdMiddleware

from tracertm.api.routers.auth import router as auth_router
from tracertm.api.routers.traceability import router as traceability_router
from tracertm.api.routers.sdlc_pm import router as sdlc_pm_router
from tracertm.api.routers.evidence import router as evidence_router
from tracertm.api.routers.org_intel import router as org_intel_router


def create_app() -> FastAPI:
    """Create the Tracera API application."""
    app = FastAPI(title="Tracera API", version="0.2.0")
    # Request-ID middleware: read X-Request-Id from inbound request, or
    # generate a uuid4, store in the module-level ContextVar, and echo
    # the value on the response. See phenotype_request_id.fastapi.
    app.add_middleware(RequestIdMiddleware)
    app.include_router(auth_router, prefix="/api/v1")
    app.include_router(traceability_router, prefix="/api/v1")
    app.include_router(sdlc_pm_router, prefix="/api/v1")
    app.include_router(evidence_router, prefix="/api/v1")
    app.include_router(org_intel_router, prefix="/api/v1")

    @app.get("/healthz", include_in_schema=False)
    async def healthz() -> dict[str, str]:
        """Liveness probe — process is up and serving HTTP.

        Excluded from OpenAPI schema (operational endpoint for orchestrators).
        """
        return {"status": "ok"}

    @app.get("/readyz", include_in_schema=False)
    async def readyz() -> dict[str, str]:
        """Readiness probe — minimal check; orchestrator-driven verification.

        Excluded from OpenAPI schema (operational endpoint for orchestrators).
        Returns version + liveness; deeper downstream checks are deferred to
        orchestrator-side probes per ADR-OPS-HEALTH-001.
        """
        return {"status": "ready", "version": app.version}

    return app


app = create_app()
