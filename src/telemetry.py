"""
OpenTelemetry instrumentation module for ForgeCode.

Provides comprehensive observability through OTLP exporters (gRPC and HTTP),
custom spans for key operations, metrics collection (counters, histograms),
health check integration, context propagation, and graceful shutdown.
"""

from __future__ import annotations

import atexit
import logging
import os
import time
import threading
from contextlib import contextmanager
from enum import Enum
from typing import Any, Dict, Generator, Optional

from opentelemetry import context, metrics, trace
from opentelemetry.context import Context
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import (
    AggregationTemporality,
    PeriodicExportingMetricReader,
)
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    SERVICE_VERSION,
    DEPLOYMENT_ENVIRONMENT,
    Resource,
)
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor, SpanExporter
from opentelemetry.trace import StatusCode

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Exporter protocol selection
# ---------------------------------------------------------------------------

class ExporterProtocol(Enum):
    """Supported OTLP transport protocols."""
    GRPC = "grpc"
    HTTP = "http/protobuf"


def _resolve_endpoint(protocol: ExporterProtocol, base: str) -> str:
    """Derive the concrete exporter endpoint from protocol and base URL."""
    if protocol == ExporterProtocol.GRPC:
        return base
    # HTTP uses a path-based endpoint (e.g. /v1/traces)
    base = base.rstrip("/")
    return base


# ---------------------------------------------------------------------------
# Core telemetry wrapper
# ---------------------------------------------------------------------------

class ForgeTelemetry:
    """Unified OpenTelemetry facade for ForgeCode services.

    Responsibilities:
        - Initialise TracerProvider, MeterProvider, and LoggerProvider.
        - Configure OTLP exporters over gRPC or HTTP.
        - Expose helper methods for custom spans and metrics.
        - Provide health-check integration and graceful shutdown.

    Usage::

        tel = ForgeTelemetry(service_name="forgecode", service_version="0.1.0")
        tracer = tel.get_tracer("forgecode.core")
        with tracer.start_as_current_span("process-request"):
            ...
        tel.shutdown()
    """

    def __init__(
        self,
        service_name: str = "forgecode",
        service_version: str = "0.1.0",
        environment: Optional[str] = None,
        endpoint: Optional[str] = None,
        protocol: Optional[ExporterProtocol] = None,
        metric_export_interval_ms: int = 30_000,
        log_level: int = logging.INFO,
    ):
        self._service_name = service_name
        self._service_version = service_version
        self._environment = environment or os.getenv("DEPLOY_ENV", "development")
        self._endpoint = (
            endpoint
            or os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317")
        )
        self._protocol = protocol or ExporterProtocol(
            os.getenv("OTEL_EXPORTER_OTLP_PROTOCOL", "grpc")
        )
        self._metric_interval = metric_export_interval_ms

        # Health tracking
        self._initialised_at: Optional[float] = None
        self._shutdown_event = threading.Event()
        self._lock = threading.Lock()

        # Resource shared by all providers
        self._resource = Resource.create(
            {
                SERVICE_NAME: service_name,
                SERVICE_VERSION: service_version,
                DEPLOYMENT_ENVIRONMENT: self._environment,
            }
        )

        # Provider instances
        self._tracer_provider: Optional[TracerProvider] = None
        self._meter_provider: Optional[MeterProvider] = None
        self._logger_provider: Optional[LoggerProvider] = None

        # Metrics instruments (created lazily once meter is ready)
        self._request_counter = None
        self._error_counter = None
        self._latency_histogram = None
        self._active_connections_gauge = None
        self._health_gauge = None

        self._init_providers()
        self._init_instruments()
        self._initialised_at = time.time()
        atexit.register(self.shutdown)

        logger.info(
            "ForgeTelemetry initialised: service=%s version=%s env=%s proto=%s endpoint=%s",
            service_name,
            service_version,
            self._environment,
            self._protocol.value,
            self._endpoint,
        )

    # ------------------------------------------------------------------
    # Provider initialisation
    # ------------------------------------------------------------------

    def _init_providers(self) -> None:
        resolved = _resolve_endpoint(self._protocol, self._endpoint)

        self._tracer_provider = self._init_tracer_provider(resolved)
        self._meter_provider = self._init_meter_provider(resolved)
        self._logger_provider = self._init_logger_provider(resolved)

    def _init_tracer_provider(self, endpoint: str) -> TracerProvider:
        exporter = OTLPSpanExporter(endpoint=endpoint)
        provider = TracerProvider(resource=self._resource)
        provider.add_span_processor(BatchSpanProcessor(exporter))
        trace.set_tracer_provider(provider)
        return provider

    def _init_meter_provider(self, endpoint: str) -> MeterProvider:
        exporter = OTLPMetricExporter(
            endpoint=endpoint,
            preferred_temporality={},
        )
        reader = PeriodicExportingMetricReader(
            exporter,
            export_interval_millis=self._metric_interval,
        )
        provider = MeterProvider(resource=self._resource, metric_readers=[reader])
        metrics.set_meter_provider(provider)
        return provider

    def _init_logger_provider(self, endpoint: str) -> LoggerProvider:
        exporter = OTLPLogExporter(endpoint=endpoint)
        provider = LoggerProvider(resource=self._resource)
        provider.add_log_record_processor(
            __import__(
                "opentelemetry.sdk._logs.export",
                fromlist=["BatchLogRecordProcessor"],
            ).BatchLogRecordProcessor(exporter)
        )
        return provider

    # ------------------------------------------------------------------
    # Metrics instruments
    # ------------------------------------------------------------------

    def _init_instruments(self) -> None:
        meter = metrics.get_meter(self._service_name)

        self._request_counter = meter.create_counter(
            name="forgecode.requests.total",
            description="Total number of processed requests",
            unit="1",
        )
        self._error_counter = meter.create_counter(
            name="forgecode.errors.total",
            description="Total number of errors",
            unit="1",
        )
        self._latency_histogram = meter.create_histogram(
            name="forgecode.latency.ms",
            description="Request latency in milliseconds",
            unit="ms",
        )
        self._active_connections_gauge = meter.create_up_down_counter(
            name="forgecode.connections.active",
            description="Number of currently active connections",
            unit="1",
        )
        self._health_gauge = meter.create_observable_gauge(
            name="forgecode.health",
            description="Service health status (1 = healthy, 0 = unhealthy)",
            callbacks=[self._health_callback],
            unit="1",
        )

    def _health_callback(self, options: Any) -> Any:
        from opentelemetry.sdk.metrics.export import Gauge

        healthy = 0 if self._shutdown_event.is_set() else 1
        yield Gauge(value=healthy)

    # ------------------------------------------------------------------
    # Public helpers – tracers, meters, context
    # ------------------------------------------------------------------

    def get_tracer(self, name: str = "forgecode") -> trace.Tracer:
        return trace.get_tracer(name)

    def get_meter(self, name: str = "forgecode") -> metrics.Meter:
        return metrics.get_meter(name)

    # ------------------------------------------------------------------
    # Custom span helpers
    # ------------------------------------------------------------------

    @contextmanager
    def span(
        self,
        name: str,
        attributes: Optional[Dict[str, Any]] = None,
        tracer_name: str = "forgecode",
    ) -> Generator[trace.Span, None, None]:
        """Context manager that creates a named span with optional attributes."""
        tracer = trace.get_tracer(tracer_name)
        with tracer.start_as_current_span(name) as span:
            if attributes:
                for k, v in attributes.items():
                    span.set_attribute(k, v)
            try:
                yield span
            except Exception as exc:
                span.set_status(StatusCode.ERROR, str(exc))
                span.record_exception(exc)
                raise

    @contextmanager
    def span_with_metrics(
        self,
        span_name: str,
        metric_label: str,
        attributes: Optional[Dict[str, Any]] = None,
    ) -> Generator[trace.Span, None, None]:
        """Span that also records latency and success/error counts."""
        start = time.perf_counter()
        with self.span(span_name, attributes) as span:
            try:
                yield span
                elapsed_ms = (time.perf_counter() - start) * 1000
                self._request_counter.add(
                    1, {"operation": metric_label, "status": "success"}
                )
                self._latency_histogram.record(
                    elapsed_ms, {"operation": metric_label}
                )
            except Exception:
                elapsed_ms = (time.perf_counter() - start) * 1000
                self._error_counter.add(
                    1, {"operation": metric_label, "status": "error"}
                )
                self._latency_histogram.record(
                    elapsed_ms, {"operation": metric_label}
                )
                raise

    # ------------------------------------------------------------------
    # Metrics convenience
    # ------------------------------------------------------------------

    def record_request(self, operation: str, status: str = "success") -> None:
        self._request_counter.add(1, {"operation": operation, "status": status})

    def record_error(self, operation: str, error_type: str = "unknown") -> None:
        self._error_counter.add(
            1, {"operation": operation, "error_type": error_type}
        )

    def record_latency(self, operation: str, latency_ms: float) -> None:
        self._latency_histogram.record(latency_ms, {"operation": operation})

    def adjust_connections(self, delta: int) -> None:
        self._active_connections_gauge.add(delta)

    # ------------------------------------------------------------------
    # Context propagation helpers
    # ------------------------------------------------------------------

    @staticmethod
    def inject_context(carrier: Optional[Dict[str, str]] = None) -> Dict[str, str]:
        """Inject the current context into a carrier for cross-service propagation."""
        from opentelemetry.propagate import inject

        carrier = carrier or {}
        inject(carrier)
        return carrier

    @staticmethod
    def extract_context(carrier: Dict[str, str]) -> Context:
        """Extract a context from an incoming carrier."""
        from opentelemetry.propagate import extract

        return extract(carrier)

    @contextmanager
    def propagation_context(self, carrier: Dict[str, str]) -> Generator[Context, None, None]:
        """Temporarily set context extracted from an incoming carrier."""
        ctx = self.extract_context(carrier)
        token = context.attach(ctx)
        try:
            yield ctx
        finally:
            context.detach(token)

    # ------------------------------------------------------------------
    # Health check
    # ------------------------------------------------------------------

    def health_check(self) -> Dict[str, Any]:
        """Return a health check payload suitable for /healthz or /readyz endpoints."""
        uptime_s: Optional[float] = None
        if self._initialised_at:
            uptime_s = time.time() - self._initialised_at

        providers_ready = all(
            p is not None
            for p in (self._tracer_provider, self._meter_provider, self._logger_provider)
        )

        return {
            "status": "healthy" if providers_ready and not self._shutdown_event.is_set() else "unhealthy",
            "service": self._service_name,
            "version": self._service_version,
            "environment": self._environment,
            "uptime_seconds": uptime_s,
            "providers": {
                "tracer": self._tracer_provider is not None,
                "meter": self._meter_provider is not None,
                "logger": self._logger_provider is not None,
            },
        }

    # ------------------------------------------------------------------
    # Graceful shutdown
    # ------------------------------------------------------------------

    def shutdown(self) -> None:
        """Flush pending telemetry and release resources."""
        if self._shutdown_event.is_set():
            return

        with self._lock:
            if self._shutdown_event.is_set():
                return
            self._shutdown_event.set()

        logger.info("ForgeTelemetry shutting down …")

        for name, provider in [
            ("tracer", self._tracer_provider),
            ("meter", self._meter_provider),
            ("logger", self._logger_provider),
        ]:
            if provider is not None:
                try:
                    provider.shutdown()
                    logger.debug("%s provider shut down", name)
                except Exception:
                    logger.exception("Error shutting down %s provider", name)

        logger.info("ForgeTelemetry shut down complete.")


# ---------------------------------------------------------------------------
# Module-level singleton (lazy)
# ---------------------------------------------------------------------------

_default_telemetry: Optional[ForgeTelemetry] = None
_singleton_lock = threading.Lock()


def init_telemetry(**kwargs: Any) -> ForgeTelemetry:
    """Initialise (or re-initialise) the module-level singleton."""
    global _default_telemetry
    with _singleton_lock:
        _default_telemetry = ForgeTelemetry(**kwargs)
        return _default_telemetry


def get_telemetry() -> ForgeTelemetry:
    """Return the module-level singleton, initialising if necessary."""
    global _default_telemetry
    if _default_telemetry is None:
        with _singleton_lock:
            if _default_telemetry is None:
                _default_telemetry = ForgeTelemetry()
    return _default_telemetry
