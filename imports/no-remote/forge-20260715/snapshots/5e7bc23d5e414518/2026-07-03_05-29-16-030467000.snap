#!/usr/bin/env python3
"""Parse a Criterion `cargo bench --output-format bencher` run into a flat dict.

Invocation:
    python scripts/bench_parse_criterion.py <bench_output.json>

Where `<bench_output.json>` is a Criterion-format JSON document produced by
an external harness that invokes each benchmark binary individually and
collects its estimates. We do NOT require cargo's `--format json-stream`
(which is unstable) — instead we accept two shapes:

  1. Direct: ``{"<crate>::<bench>::<sample>": {"value": <ns>, ...}, ...}``
  2. Criterion raw: see the ESTIMATES schema, where we pluck the median.

The output is a stable flat dict on stdout, sorted by key, suitable for
`bench_diff.py` to consume.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Stable ordering for reproducibility.
OUT_KEYS = ("crate", "bench", "value_ns", "lower_ns", "upper_ns", "unit")


def parse(json_path: Path) -> list[dict]:
    raw = json.loads(json_path.read_text(encoding="utf-8"))
    rows: list[dict] = []

    # Shape 1: flat dict where key is "<crate>::<bench>::<sample>"
    if isinstance(raw, dict):
        for key, val in raw.items():
            parts = key.split("::")
            if len(parts) < 2:
                continue
            if not isinstance(val, dict):
                continue
            value = val.get("value") or val.get("median") or val.get("point_estimate")
            if value is None:
                continue
            rows.append(
                {
                    "crate": parts[0],
                    "bench": parts[1],
                    "value_ns": float(value),
                    "lower_ns": float(val.get("lower", val.get("lower_bound", value))),
                    "upper_ns": float(val.get("upper", val.get("upper_bound", value))),
                    "unit": str(val.get("unit", "ns")),
                }
            )
        return sorted(rows, key=lambda r: (r["crate"], r["bench"]))

    # Shape 2: criterion's own JSON output. Expect a list of records.
    if isinstance(raw, list):
        for record in raw:
            full_id = record.get("full_id") or record.get("name") or ""
            parts = full_id.split("::")
            if len(parts) < 2:
                continue
            estimate = record.get("estimate") or record.get("median") or {}
            value = estimate.get("value") or estimate.get("median") or record.get("value")
            if value is None:
                continue
            rows.append(
                {
                    "crate": parts[0],
                    "bench": parts[1],
                    "value_ns": float(value),
                    "lower_ns": float(estimate.get("lower", value)),
                    "upper_ns": float(estimate.get("upper", value)),
                    "unit": str(estimate.get("unit", "ns")),
                }
            )
        return sorted(rows, key=lambda r: (r["crate"], r["bench"]))

    raise ValueError(f"Unsupported root shape: {type(raw).__name__}")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    rows = parse(Path(sys.argv[1]).resolve())
    for row in rows:
        print(json.dumps({k: row[k] for k in OUT_KEYS}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
