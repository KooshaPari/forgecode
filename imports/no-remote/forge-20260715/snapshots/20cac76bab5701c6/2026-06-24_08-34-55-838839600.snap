"""Tests for the MLflow-compatible tracking facade."""

# ruff: noqa: ANN001, ANN002, ANN202, ANN204, D103, S101

from __future__ import annotations

import json

import httpx
import pytest

from tracertm.mlflow_compat import Run, TrackingClient


def test_file_run_logs_metrics_params_and_artifacts(tmp_path) -> None:
    artifact = tmp_path / "model.json"
    artifact.write_text('{"ok": true}', encoding="utf-8")
    run = Run(tracking_uri=tmp_path.as_uri())

    run.log_params({"model": "ranker", "epochs": 2})
    run.log_metric("loss", 0.4, step=1)
    copied_to = run.log_artifact(artifact)
    run.end()

    events = [
        json.loads(line)
        for line in (tmp_path / run.run_id / "events.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    ]
    assert events[1]["payload"] == {"key": "model", "run_id": run.run_id, "value": "ranker"}
    assert events[3]["payload"]["key"] == "loss"
    assert events[3]["payload"]["step"] == 1
    assert copied_to == tmp_path / run.run_id / "artifacts" / "model.json"
    assert copied_to.read_text(encoding="utf-8") == '{"ok": true}'


def test_tracking_client_get_and_search_runs_file_backend(tmp_path) -> None:
    client = TrackingClient(tmp_path.as_uri())
    run = client.start_run(run_id="run-1")
    run.log_metric("accuracy", 0.9, step=3)
    run.end()

    loaded = client.get_run("run-1")
    runs = client.search_runs()

    assert loaded["run_id"] == "run-1"
    assert loaded["events"][1]["payload"]["key"] == "accuracy"
    assert [item["run_id"] for item in runs] == ["run-1"]


def test_http_backend_uses_mlflow_rest_endpoints(monkeypatch) -> None:
    requests = []

    class FakeClient:
        def __init__(self, timeout) -> None:
            self.timeout = timeout

        def __enter__(self):
            return self

        def __exit__(self, *args) -> None:
            return None

        def post(self, url, json):
            requests.append(("POST", url, json))
            return httpx.Response(200, request=httpx.Request("POST", url))

        def get(self, url, params):
            requests.append(("GET", url, params))
            return httpx.Response(
                200,
                json={"runs": [{"info": {"run_id": "abc"}}]},
                request=httpx.Request("GET", url),
            )

    monkeypatch.setattr("tracertm.mlflow_compat.httpx.Client", FakeClient)
    client = TrackingClient("http://mlflow.local")
    run = client.start_run(run_id="abc")
    run.log_metric("lr", 0.01, step=4)

    assert client.search_runs() == [{"info": {"run_id": "abc"}}]
    assert requests[0] == (
        "POST",
        "http://mlflow.local/api/2.0/mlflow/runs/create",
        {"run_id": "abc", "experiment_id": "0"},
    )
    assert requests[1][1] == "http://mlflow.local/api/2.0/mlflow/runs/log-metric"
    assert requests[2][1] == "http://mlflow.local/api/2.0/mlflow/runs/search"


def test_ended_run_rejects_late_logging(tmp_path) -> None:
    run = Run(tracking_uri=tmp_path.as_uri())
    run.end()

    with pytest.raises(RuntimeError, match="already ended"):
        run.log_metric("loss", 0.1, step=1)


def test_emit_creates_trace_span_with_event_attributes(tmp_path, monkeypatch) -> None:
    spans = []

    class SpanContext:
        def __init__(self, name, attributes) -> None:
            self.name = name
            self.attributes = attributes

        def __enter__(self):
            spans.append((self.name, self.attributes))
            return self

        def __exit__(self, *args) -> None:
            return None

    class FakeTracer:
        def start_as_current_span(self, name, attributes):
            return SpanContext(name, attributes)

    monkeypatch.setattr("tracertm.mlflow_compat._TRACER", FakeTracer())

    run = Run(run_id="run-1", tracking_uri=tmp_path.as_uri())
    run.log_metric("loss", 0.1, step=1)

    assert spans[0][0] == "tracertm.bus.emit"
    assert spans[0][1]["event.type"] == "runs/create"
    assert spans[0][1]["event.id"]
    assert spans[0][1]["source"] == "tracertm.mlflow_compat"
    assert spans[0][1]["correlation_id"] == "run-1"
