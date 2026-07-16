"""Coverage matrix and impact analysis REST endpoints."""

from __future__ import annotations

from collections import defaultdict, deque
from datetime import UTC, datetime
from typing import Literal

from fastapi import APIRouter
from pydantic import BaseModel, Field

from tracertm.governance import (
    GovernanceReport,
    GovernanceSpec,
    GovernanceTrace,
    evaluate_spec_first_governance,
)

router = APIRouter(tags=["traceability"])

TraceRelationship = Literal[
    "satisfies",
    "verifies",
    "implements",
    "derives_from",
    "refines",
    "conflicts_with",
    "duplicates",
]
CoverageState = Literal["covered", "partial", "missing", "stale", "conflict"]


class TraceLinkInput(BaseModel):
    """Trace link submitted to the REST API."""

    source_id: str = Field(..., min_length=1)
    target_id: str = Field(..., min_length=1)
    relationship: TraceRelationship
    confidence: float = Field(1.0, ge=0.0, le=1.0)
    updated_at: datetime | None = None


class MatrixCellResponse(BaseModel):
    """One coverage matrix cell."""

    source_id: str
    target_id: str
    coverage: CoverageState
    links: list[TraceLinkInput]


class CoverageMatrixRequest(BaseModel):
    """Coverage matrix build request."""

    links: list[TraceLinkInput] = Field(default_factory=list)
    stale_after_days: int = Field(90, ge=1)


class CoverageMatrixResponse(BaseModel):
    """Coverage matrix build response."""

    generated_at: datetime
    link_count: int
    cell_count: int
    stale_links: int
    cells: list[MatrixCellResponse]


class ImpactRequest(CoverageMatrixRequest):
    """Impact analysis request."""

    changed_artifact_ids: list[str] = Field(..., min_length=1)
    max_depth: int = Field(10, ge=0)


class ImpactNodeResponse(BaseModel):
    """One artifact affected by a change."""

    artifact_id: str
    depth: int
    via: list[TraceRelationship]
    score: float


class ImpactResponse(BaseModel):
    """Impact analysis response."""

    seeds: list[str]
    affected: list[ImpactNodeResponse]
    total_score: float
    truncated: bool
    max_depth_seen: int
    conflicts: list[TraceLinkInput]


class GovernanceCheckRequest(BaseModel):
    """Spec-first governance gate request."""

    specs: list[GovernanceSpec] = Field(default_factory=list)
    traces: list[GovernanceTrace] = Field(default_factory=list)


@router.post("/coverage-matrix", response_model=CoverageMatrixResponse)
async def build_coverage_matrix(request: CoverageMatrixRequest) -> CoverageMatrixResponse:
    """Build a coverage matrix from trace links."""
    return _build_coverage_matrix(request)


@router.post("/governance/spec-check", response_model=GovernanceReport)
async def check_spec_first_governance(request: GovernanceCheckRequest) -> GovernanceReport:
    """Run the spec-first governance gate for planned work."""
    return evaluate_spec_first_governance(request.specs, request.traces)


@router.post("/impact", response_model=ImpactResponse)
async def analyze_impact(request: ImpactRequest) -> ImpactResponse:
    """Compute impacted artifacts from changed artifact IDs."""
    matrix = _build_coverage_matrix(request)
    adjacency: dict[str, list[tuple[str, TraceLinkInput]]] = defaultdict(list)
    link_by_pair = {(cell.source_id, cell.target_id): cell.links for cell in matrix.cells}
    for (source_id, target_id), links in link_by_pair.items():
        for link in links:
            adjacency[source_id].append((target_id, link))
            adjacency[target_id].append((source_id, link))

    visited: dict[str, ImpactNodeResponse] = {
        seed: ImpactNodeResponse(artifact_id=seed, depth=0, via=[], score=1.0)
        for seed in request.changed_artifact_ids
    }
    queue: deque[tuple[str, int, float, list[TraceRelationship]]] = deque(
        (seed, 0, 1.0, []) for seed in request.changed_artifact_ids
    )
    conflicts: list[TraceLinkInput] = []
    truncated = False
    max_depth_seen = 0

    while queue:
        artifact_id, depth, decay, via = queue.popleft()
        max_depth_seen = max(max_depth_seen, depth)
        if request.max_depth and depth >= request.max_depth:
            truncated = True
            continue
        for neighbor_id, link in adjacency.get(artifact_id, []):
            multiplier = _impact_multiplier(link.relationship)
            score = round(decay * link.confidence * multiplier, 6)
            next_via = [*via, link.relationship]
            if link.relationship == "conflicts_with":
                conflicts.append(link)
            current = visited.get(neighbor_id)
            if current and current.depth <= depth + 1 and abs(current.score) >= abs(score):
                continue
            visited[neighbor_id] = ImpactNodeResponse(
                artifact_id=neighbor_id,
                depth=depth + 1,
                via=next_via,
                score=score,
            )
            if not request.max_depth or depth + 1 <= request.max_depth:
                queue.append((neighbor_id, depth + 1, decay * 0.85, next_via))

    affected = sorted(visited.values(), key=lambda node: (-abs(node.score), node.artifact_id))
    return ImpactResponse(
        seeds=request.changed_artifact_ids,
        affected=affected,
        total_score=round(sum(node.score for node in affected), 6),
        truncated=truncated,
        max_depth_seen=max_depth_seen,
        conflicts=conflicts,
    )


def _build_coverage_matrix(request: CoverageMatrixRequest) -> CoverageMatrixResponse:
    grouped: dict[tuple[str, str], list[TraceLinkInput]] = defaultdict(list)
    stale_links = 0
    now = datetime.now(UTC)
    for link in request.links:
        grouped[link.source_id, link.target_id].append(link)
        if link.updated_at and (now - link.updated_at).days > request.stale_after_days:
            stale_links += 1

    cells = [
        MatrixCellResponse(
            source_id=source_id,
            target_id=target_id,
            coverage=_classify_coverage(links, request.stale_after_days, now),
            links=links,
        )
        for (source_id, target_id), links in sorted(grouped.items())
    ]
    return CoverageMatrixResponse(
        generated_at=now,
        link_count=len(request.links),
        cell_count=len(cells),
        stale_links=stale_links,
        cells=cells,
    )


def _classify_coverage(
    links: list[TraceLinkInput],
    stale_after_days: int,
    now: datetime,
) -> CoverageState:
    if any(link.relationship == "conflicts_with" for link in links):
        return "conflict"
    if any(
        link.relationship in {"verifies", "satisfies"} and link.confidence >= 0.9
        for link in links
    ):
        return "covered"
    if any(link.relationship in {"verifies", "satisfies"} for link in links):
        return "partial"
    if any(link.updated_at and (now - link.updated_at).days > stale_after_days for link in links):
        return "stale"
    return "missing"


def _impact_multiplier(relationship: TraceRelationship) -> float:
    if relationship == "conflicts_with":
        return -1.5
    if relationship in {"satisfies", "implements", "refines"}:
        return 1.0
    if relationship == "verifies":
        return 0.75
    return 0.25


# ---------------------------------------------------------------------------
# Confidence scoring endpoint (FR-TRC-019)
# ---------------------------------------------------------------------------

class ConfidenceRequest(BaseModel):
    requirement_text: str = Field(..., description="Text of the requirement")
    artifact_text: str = Field(..., description="Text or summary of the artifact")


class ConfidenceResponse(BaseModel):
    confidence: float = Field(..., ge=0.0, le=1.0)
    rationale: str


@router.post("/confidence", response_model=ConfidenceResponse)
def compute_confidence(req: ConfidenceRequest) -> ConfidenceResponse:
    """Score agreement between a requirement and an artifact using JaccardScorer (FR-TRC-019)."""
    from tracertm.ports.scorer import JaccardScorer

    result = JaccardScorer().score(req.requirement_text, req.artifact_text)
    return ConfidenceResponse(confidence=result.score, rationale=result.rationale)
