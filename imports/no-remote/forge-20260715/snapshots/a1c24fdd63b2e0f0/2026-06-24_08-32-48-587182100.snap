06:19:57.958227 exec-cmd.c:266          trace: resolved executable dir: C:/Program Files/Git/mingw64/bin
06:19:57.973665 git.c:476               trace: built-in: git show :3:tests/unit/test_governance_and_models.py
"""Unit tests for governance evaluator and trace-link domain models."""
from __future__ import annotations

import uuid

import pytest

from tracertm.governance import (
    GovernanceSpec,
    GovernanceTrace,
    GovernanceViolation,
    evaluate_spec_first_governance,
)
from tracertm.models.trace_link import (
    ArtifactKind,
    RequirementStatus,
    TraceLink,
    TraceLinkType,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_spec(spec_id: str, status: str = "approved", criteria: list[str] | None = None, evidence: list[str] | None = None) -> GovernanceSpec:
    return GovernanceSpec(
        spec_id=spec_id,
        title=f"Spec {spec_id}",
        owner="test-owner",
        acceptance_criteria=criteria if criteria is not None else ["AC1"],
        evidence_links=evidence if evidence is not None else ["https://ci.example.com/1"],
        status=status,
    )


def _make_trace(spec_id: str, target_id: str = "impl-1", kind: str = "implementation") -> GovernanceTrace:
    return GovernanceTrace(spec_id=spec_id, target_id=target_id, kind=kind)  # type: ignore[arg-type]


def _make_link(confidence: float = 0.9) -> TraceLink:
    return TraceLink(
        project_id=uuid.uuid4(),
        source_artifact_id=uuid.uuid4(),
        target_artifact_id=uuid.uuid4(),
        link_type=TraceLinkType.SATISFIES,
        confidence=confidence,
        rationale="Requirement is satisfied by implementation",
    )


# ---------------------------------------------------------------------------
# Governance evaluator tests
# ---------------------------------------------------------------------------

def test_empty_specs_passes() -> None:
    report = evaluate_spec_first_governance([], [])
    assert report.status == "pass"
    assert report.spec_count == 0
    assert report.trace_count == 0
    assert report.violations == []


def test_approved_spec_with_criteria_and_evidence_passes() -> None:
    spec = _make_spec("FR-001")
    traces = [_make_trace("FR-001"), _make_trace("FR-001", "test-1", "test")]
    report = evaluate_spec_first_governance([spec], traces)
    assert report.status == "pass"
    assert report.violations == []


def test_unapproved_spec_fails() -> None:
    spec = _make_spec("FR-002", status="draft")
    report = evaluate_spec_first_governance([spec], [])
    assert report.status == "fail"
    codes = {v.code for v in report.violations}
    assert "not_approved" in codes


def test_missing_acceptance_criteria_fails() -> None:
    spec = _make_spec("FR-003", criteria=[])
    report = evaluate_spec_first_governance([spec], [])
    assert report.status == "fail"
    codes = {v.code for v in report.violations}
    assert "missing_acceptance" in codes


def test_missing_evidence_links_fails() -> None:
    spec = _make_spec("FR-004", evidence=[])
    report = evaluate_spec_first_governance([spec], [])
    assert report.status == "fail"
    codes = {v.code for v in report.violations}
    assert "missing_evidence" in codes


def test_orphan_trace_fails() -> None:
    trace = _make_trace("UNKNOWN-999", "impl-x")
    report = evaluate_spec_first_governance([], [trace])
    assert report.status == "fail"
    codes = {v.code for v in report.violations}
    assert "orphan_trace" in codes


def test_duplicate_spec_id_fails() -> None:
    spec1 = _make_spec("FR-005")
    spec2 = _make_spec("FR-005")
    report = evaluate_spec_first_governance([spec1, spec2], [])
    codes = {v.code for v in report.violations}
    assert "duplicate_spec" in codes


def test_counts_are_correct() -> None:
    specs = [_make_spec("FR-006"), _make_spec("FR-007")]
    traces = [_make_trace("FR-006"), _make_trace("FR-007"), _make_trace("FR-007", "test-1", "test")]
    report = evaluate_spec_first_governance(specs, traces)
    assert report.spec_count == 2
    assert report.trace_count == 3


# ---------------------------------------------------------------------------
# TraceLink domain model tests
# ---------------------------------------------------------------------------

def test_to_dict_contains_expected_keys() -> None:
    link = _make_link()
    d = link.to_dict()
    assert "project_id" in d
    assert "link_type" in d
    assert d["link_type"] == "SATISFIES"
    assert d["confidence"] == 0.9


def test_from_dict_roundtrip() -> None:
    link = _make_link()
    d = link.to_dict()
    restored = TraceLink.from_dict(d)
    assert restored.project_id == link.project_id
    assert restored.link_type == TraceLinkType.SATISFIES
    assert restored.confidence == link.confidence
    assert restored.rationale == link.rationale


def test_default_confidence_is_one() -> None:
    link = TraceLink(
        project_id=uuid.uuid4(),
        source_artifact_id=uuid.uuid4(),
        target_artifact_id=uuid.uuid4(),
        link_type=TraceLinkType.VERIFIES,
    )
    assert link.confidence == 1.0


def test_trace_link_types_have_correct_values() -> None:
    assert TraceLinkType.IMPLEMENTS.value == "IMPLEMENTS"
    assert TraceLinkType.VERIFIES.value == "VERIFIES"
    assert TraceLinkType.SATISFIES.value == "SATISFIES"


# ---------------------------------------------------------------------------
# Enum tests
# ---------------------------------------------------------------------------

def test_artifact_kind_values() -> None:
    assert ArtifactKind.REQUIREMENT.value == "requirement"
    assert ArtifactKind.TEST.value == "test"


def test_requirement_status_values() -> None:
    assert RequirementStatus.DRAFT.value == "draft"
    assert RequirementStatus.APPROVED.value == "approved"