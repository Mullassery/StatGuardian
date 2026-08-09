"""
StatGuardian Lineage Module — Lineage as First-Class Feature

This module provides Python access to lineage tracking, versioning, and analysis
for data warehouse transformations.

Example:
    ```python
    from statguardian import get_lineage_graph, get_lineage_version

    # Get current lineage graph
    lineage = get_lineage_graph(warehouse_config)

    # Access nodes and edges
    for node_id, node in lineage.nodes.items():
        print(f"{node.table_id}:{node.stage} at {node.warehouse}")

    # Find impact chain
    impact = lineage.get_impact_chain("customers_raw")
    print(f"If customers_raw breaks, affects: {impact}")
    ```
"""

from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional
from datetime import datetime, timezone
import json
import os

from . import _statguardian


def _utcnow_iso() -> str:
    """RFC3339 UTC timestamp (with a 'Z' offset) -- required for round-tripping
    through the Rust side's `DateTime<Utc>` (chrono), which rejects the offset-less
    strings that plain `datetime.utcnow().isoformat()` produces."""
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


@dataclass
class LineageNode:
    """A table at a specific transformation stage"""

    node_id: str
    table_id: str
    stage: str
    version: int
    warehouse: str
    database: str
    schema_name: str
    table_name: str
    transformation_sql: Optional[str] = None
    transformation_name: Optional[str] = None
    description: Optional[str] = None
    owner: Optional[str] = None
    created_at: Optional[str] = None  # ISO format datetime
    last_accessed: Optional[str] = None
    columns: List[str] = field(default_factory=list)
    row_count: Optional[int] = None

    def qualified_name(self) -> str:
        """Full qualified name"""
        return f"{self.database}.{self.schema_name}.{self.table_name}"

    def cache_key(self) -> str:
        """Unique cache key"""
        return f"{self.table_id}:{self.stage}:v{self.version}"


@dataclass
class LineageEdge:
    """A dependency from one table to another"""

    edge_id: str
    source_node_id: str
    target_node_id: str
    join_type: Optional[str] = None
    columns_used: List[str] = field(default_factory=list)
    filter_condition: Optional[str] = None
    version: int = 1
    created_at: Optional[str] = None
    last_modified: Optional[str] = None
    modification_reason: Optional[str] = None


@dataclass
class LineageChange:
    """A single change in the lineage graph"""

    change_type: str  # "node_added", "node_removed", "edge_added", "edge_removed", "edge_modified"
    source_table: str
    target_table: Optional[str] = None
    change_reason: Optional[str] = None
    changed_at: Optional[str] = None
    changed_by: Optional[str] = None
    tables_affected: List[str] = field(default_factory=list)
    severity: str = "none"  # "none", "low", "medium", "high"
    propagates_schema_changes: bool = False


@dataclass
class LineageGraph:
    """Complete DAG of table transformations"""

    warehouse_id: str
    timestamp: str  # ISO format datetime
    nodes: Dict[str, LineageNode] = field(default_factory=dict)
    edges: List[LineageEdge] = field(default_factory=list)

    def add_node(self, node: LineageNode) -> None:
        """Add a node to the graph"""
        self.nodes[node.node_id] = node

    def add_edge(self, edge: LineageEdge) -> None:
        """Add an edge to the graph"""
        self.edges.append(edge)

    def get_upstream(self, target_node_id: str) -> List[str]:
        """Get all upstream tables (tables that feed into target)"""
        return [
            e.source_node_id
            for e in self.edges
            if e.target_node_id == target_node_id
        ]

    def get_downstream(self, source_node_id: str) -> List[str]:
        """Get all downstream tables (tables that depend on source)"""
        return [
            e.target_node_id
            for e in self.edges
            if e.source_node_id == source_node_id
        ]

    def get_impact_chain(self, target_node_id: str) -> List[str]:
        """Compute impact chain: all tables affected if target breaks"""
        affected = []
        queue = [target_node_id]
        visited = set()

        while queue:
            current = queue.pop(0)
            if current in visited:
                continue
            visited.add(current)
            affected.append(current)

            for downstream in self.get_downstream(current):
                if downstream not in visited:
                    queue.append(downstream)

        return affected

    def get_node(self, node_id: str) -> Optional[LineageNode]:
        """Get node by ID"""
        return self.nodes.get(node_id)

    def size(self) -> tuple:
        """Return (node_count, edge_count)"""
        return (len(self.nodes), len(self.edges))


@dataclass
class LineageVersion:
    """Snapshot of lineage at a point in time"""

    version_id: str
    lineage_graph: LineageGraph
    version_number: int
    timestamp: str  # ISO format datetime
    schema_versions: Dict[str, int] = field(default_factory=dict)
    quality_scores: Dict[str, float] = field(default_factory=dict)
    changes_from_previous: List[LineageChange] = field(default_factory=list)
    change_severity: str = "none"


def create_lineage_node(
    table_id: str,
    stage: str,
    version: int,
    warehouse: str,
    database: str,
    schema_name: str,
    table_name: str,
) -> LineageNode:
    """Helper to create a LineageNode with generated node_id"""
    node_id = f"{table_id}__{stage}__{version}"
    return LineageNode(
        node_id=node_id,
        table_id=table_id,
        stage=stage,
        version=version,
        warehouse=warehouse,
        database=database,
        schema_name=schema_name,
        table_name=table_name,
        created_at=_utcnow_iso(),
    )


def create_lineage_edge(
    source_node_id: str,
    target_node_id: str,
) -> LineageEdge:
    """Helper to create a LineageEdge"""
    edge_id = f"{source_node_id}__{target_node_id}"
    return LineageEdge(
        edge_id=edge_id,
        source_node_id=source_node_id,
        target_node_id=target_node_id,
        created_at=_utcnow_iso(),
    )


def lineage_node_from_dict(data: dict) -> LineageNode:
    """Deserialize LineageNode from dict"""
    return LineageNode(**data)


def lineage_edge_from_dict(data: dict) -> LineageEdge:
    """Deserialize LineageEdge from dict"""
    return LineageEdge(**data)


def lineage_graph_from_dict(data: dict) -> LineageGraph:
    """Deserialize LineageGraph from dict"""
    nodes = {
        k: lineage_node_from_dict(v) for k, v in data.get("nodes", {}).items()
    }
    edges = [lineage_edge_from_dict(e) for e in data.get("edges", [])]

    return LineageGraph(
        warehouse_id=data["warehouse_id"],
        timestamp=data["timestamp"],
        nodes=nodes,
        edges=edges,
    )


def lineage_version_from_dict(data: dict) -> LineageVersion:
    """Deserialize LineageVersion from dict"""
    return LineageVersion(
        version_id=data["version_id"],
        lineage_graph=lineage_graph_from_dict(data["lineage_graph"]),
        version_number=data["version_number"],
        timestamp=data["timestamp"],
        schema_versions=data.get("schema_versions", {}),
        quality_scores=data.get("quality_scores", {}),
        changes_from_previous=[
            LineageChange(**c) for c in data.get("changes_from_previous", [])
        ],
        change_severity=data.get("change_severity", "none"),
    )


def _lineage_db_path(warehouse_config: dict) -> str:
    """Resolve the SQLite lineage store path for a warehouse config.

    Defaults to `.statguardian/lineage.db` under the current working directory
    (created on first write) unless `lineage_db_path` is set explicitly.
    """
    path = warehouse_config.get("lineage_db_path", ".statguardian/lineage.db")
    if path != ":memory:":
        parent = os.path.dirname(path)
        if parent:
            os.makedirs(parent, exist_ok=True)
    return path


def save_lineage_version(
    warehouse_config: dict,
    lineage_graph: LineageGraph,
    changes: Optional[List[LineageChange]] = None,
    quality_scores: Optional[Dict[str, float]] = None,
) -> LineageVersion:
    """
    Persist a new lineage version snapshot for a warehouse.

    The version number is assigned automatically (one past the current
    latest, or 1 if this is the first version).

    Args:
        warehouse_config: Configuration dict (see `_lineage_db_path`)
        lineage_graph: The current lineage graph to snapshot
        changes: Changes since the previous version (optional)
        quality_scores: Per-table quality scores (table_id -> 0.0-1.0)

    Returns:
        The saved LineageVersion, including its assigned version_number.
    """
    db_path = _lineage_db_path(warehouse_config)
    warehouse_id = lineage_graph.warehouse_id

    latest_json = _statguardian.lineage_get_latest_version(db_path, warehouse_id)
    next_version_number = (
        json.loads(latest_json)["version_number"] + 1 if latest_json else 1
    )

    changes = changes or []
    for change in changes:
        if change.changed_at is None:
            change.changed_at = _utcnow_iso()

    severity_rank = {"none": 0, "low": 1, "medium": 2, "high": 3}
    change_severity = "none"
    for change in changes:
        if severity_rank.get(change.severity, 0) > severity_rank.get(change_severity, 0):
            change_severity = change.severity

    version = LineageVersion(
        version_id=f"{warehouse_id}__v{next_version_number}",
        lineage_graph=lineage_graph,
        version_number=next_version_number,
        timestamp=_utcnow_iso(),
        quality_scores=quality_scores or {},
        changes_from_previous=changes,
        change_severity=change_severity,
    )

    _statguardian.lineage_save_version(db_path, json.dumps(asdict(version)))
    return version


def get_lineage_graph(warehouse_config: dict) -> LineageGraph:
    """
    Get the current (latest) lineage graph for a warehouse.

    Args:
        warehouse_config: Configuration dict with warehouse connection details

    Returns:
        LineageGraph: Complete warehouse DAG, or an empty graph if no version
        has been saved yet for this warehouse.
    """
    warehouse_id = warehouse_config.get("warehouse_id", "unknown")
    version = get_lineage_version(warehouse_config)

    if version is not None:
        return version.lineage_graph

    return LineageGraph(warehouse_id=warehouse_id, timestamp=_utcnow_iso())


def get_lineage_version(
    warehouse_config: dict, version: Optional[int] = None
) -> Optional[LineageVersion]:
    """
    Get a specific lineage version.

    Args:
        warehouse_config: Configuration dict
        version: Version number (None = latest)

    Returns:
        LineageVersion or None if not found
    """
    db_path = _lineage_db_path(warehouse_config)
    warehouse_id = warehouse_config.get("warehouse_id", "unknown")

    if version is None:
        version_json = _statguardian.lineage_get_latest_version(db_path, warehouse_id)
    else:
        version_json = _statguardian.lineage_get_version(db_path, warehouse_id, version)

    if version_json is None:
        return None

    return lineage_version_from_dict(json.loads(version_json))


def get_lineage_history(
    warehouse_config: dict, table_id: str, limit: int = 10
) -> List[LineageVersion]:
    """
    Get lineage history for a specific table: the versions among the most
    recent `limit` whose recorded changes actually touched `table_id`
    (as a source, target, or otherwise-affected table).

    Args:
        warehouse_config: Configuration dict
        table_id: Table identifier
        limit: Max recent versions to scan

    Returns:
        List of LineageVersion objects, most recent first.
    """
    db_path = _lineage_db_path(warehouse_config)
    warehouse_id = warehouse_config.get("warehouse_id", "unknown")

    version_numbers = _statguardian.lineage_list_versions(db_path, warehouse_id, limit)

    history = []
    for version_number in version_numbers:
        version_json = _statguardian.lineage_get_version(db_path, warehouse_id, version_number)
        if version_json is None:
            continue

        version = lineage_version_from_dict(json.loads(version_json))
        touches_table = any(
            change.source_table == table_id
            or change.target_table == table_id
            or table_id in change.tables_affected
            for change in version.changes_from_previous
        )
        if touches_table:
            history.append(version)

    return history


__all__ = [
    "LineageNode",
    "LineageEdge",
    "LineageGraph",
    "LineageVersion",
    "LineageChange",
    "create_lineage_node",
    "create_lineage_edge",
    "lineage_node_from_dict",
    "lineage_edge_from_dict",
    "lineage_graph_from_dict",
    "lineage_version_from_dict",
    "save_lineage_version",
    "get_lineage_graph",
    "get_lineage_version",
    "get_lineage_history",
]
