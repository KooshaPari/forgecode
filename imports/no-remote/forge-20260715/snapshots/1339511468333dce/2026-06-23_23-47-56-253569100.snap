"""Request-ID middleware for FastAPI.

Reads ``X-Request-Id`` from the inbound request (or generates a UUID4),
stores it in a module-level :class:`contextvars.ContextVar`, and echoes the
value on the response.  This replaces the removed ``phenotype-request-id``
PyPI dependency (which was a project-private package unavailable on PyPI).
"""
from __future__ import annotations

import uuid
from contextvars import ContextVar

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.types import ASGIApp

#: Holds the current request-ID for the active request context.
request_id_var: ContextVar[str] = ContextVar("request_id", default="")

HEADER_NAME = "X-Request-ID"


class RequestIdMiddleware(BaseHTTPMiddleware):
    """Attach a unique request-ID to every request and response.

    Parameters
    ----------
    app:
        The ASGI application to wrap.
    header_name:
        The HTTP header used to read/write the request-ID.
        Defaults to ``X-Request-ID``.
    """

    def __init__(self, app: ASGIApp, header_name: str = HEADER_NAME) -> None:
        super().__init__(app)
        self.header_name = header_name

    async def dispatch(self, request: Request, call_next):
        req_id = request.headers.get(self.header_name) or str(uuid.uuid4())
        token = request_id_var.set(req_id)
        try:
            response = await call_next(request)
        finally:
            request_id_var.reset(token)
        response.headers[self.header_name] = req_id
        return response
