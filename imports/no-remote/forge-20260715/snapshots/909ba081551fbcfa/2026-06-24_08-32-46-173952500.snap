06:19:55.574023 exec-cmd.c:266          trace: resolved executable dir: C:/Program Files/Git/mingw64/bin
06:19:55.591248 git.c:476               trace: built-in: git show :3:tests/unit/ports/test_graph_contract.py
"""Unit tests for the canonical typed-graph schema contract.

Covers ``FR-TRC-018`` / ``NFR-TRC-010`` (single contract; drift impossible).
"""

from __future__ import annotations

import pytest

from tracertm.ports.graph_contract import (
    CANONICAL_EDGE_TYPES,
    CANONICAL_NODE_KINDS,
    EdgeType,
    GraphEdge,
    GraphNode,
    GraphPort,
    NodeKind,
    SchemaContractError,
    validate_edge,
    validate_node,
)


def _node(kind: NodeKind, id_: str = "x") -> GraphNode:
    return GraphNode(kind=kind, id=id_)


def test_canonical_vocabulary_is_closed_and_nonempty():
    assert CANONICAL_NODE_KINDS == frozenset(NodeKind)
    assert CANONICAL_EDGE_TYPES == frozenset(EdgeType)
    assert NodeKind.REQUIREMENT in CANONICAL_NODE_KINDS
    assert EdgeType.TRACES_TO in CANONICAL_EDGE_TYPES


def test_validate_node_accepts_canonical_node():
    n = _node(NodeKind.REQUIREMENT, "FR-TRC-018")
    assert validate_node(n) is n


def test_validate_node_rejects_empty_id():
    with pytest.raises(SchemaContractError):
        validate_node(GraphNode(kind=NodeKind.CODE, id="   "))


def test_validate_edge_accepts_valid_endpoints():
    edge = GraphEdge(
        type=EdgeType.IMPLEMENTS,
        src=_node(NodeKind.CODE, "mod.py"),
        dst=_node(NodeKind.REQUIREMENT, "FR-TRC-018"),
    )
    assert validate_edge(edge) is edge


def test_validate_edge_rejects_bad_source_kind():
    # A Requirement cannot IMPLEMENTS another node.
    edge = GraphEdge(
        type=EdgeType.IMPLEMENTS,
        src=_node(NodeKind.REQUIREMENT, "FR-TRC-018"),
        dst=_node(NodeKind.REQUIREMENT, "FR-TRC-019"),
    )
    with pytest.raises(SchemaContractError):
        validate_edge(edge)


def test_validate_edge_rejects_bad_target_kind():
    # COVERS must point at Requirement/Code/Spec, not a PR.
    edge = GraphEdge(
        type=EdgeType.COVERS,
        src=_node(NodeKind.TEST, "t1"),
        dst=_node(NodeKind.PR, "pr-1"),
    )
    with pytest.raises(SchemaContractError):
        validate_edge(edge)


def test_open_ended_edges_allow_any_endpoints():
    edge = GraphEdge(
        type=EdgeType.TRACES_TO,
        src=_node(NodeKind.PR, "pr-1"),
        dst=_node(NodeKind.OKR, "okr-1"),
    )
    assert validate_edge(edge) is edge


def test_graph_port_is_runtime_checkable_protocol():
    class _InMemoryGraph:
        def __init__(self) -> None:
            self.nodes: list[GraphNode] = []
            self.edges: list[GraphEdge] = []

        def upsert_node(self, node):
            self.nodes.append(validate_node(node))

        def upsert_edge(self, edge):
            self.edges.append(validate_edge(edge))

        def upsert_nodes(self, nodes):
            for n in nodes:
                self.upsert_node(n)

        def upsert_edges(self, edges):
            for e in edges:
                self.upsert_edge(e)

        def neighbors(self, node, *, edge_type=None, direction="out"):
            return [
                e
                for e in self.edges
                if (e.src == node if direction == "out" else e.dst == node)
                and (edge_type is None or e.type == edge_type)
            ]

    g = _InMemoryGraph()
    assert isinstance(g, GraphPort)
    g.upsert_edge(
        GraphEdge(
            type=EdgeType.IMPLEMENTS,
            src=_node(NodeKind.CODE, "graph_contract.py"),
            dst=_node(NodeKind.REQUIREMENT, "FR-TRC-018"),
        )
    )
    assert len(g.neighbors(_node(NodeKind.CODE, "graph_contract.py"))) == 1