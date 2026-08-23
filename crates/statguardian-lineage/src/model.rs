use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A table at a specific transformation stage in a data warehouse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Unique identifier for this node
    pub node_id: String,

    /// Logical table identifier (e.g., "customers")
    pub table_id: String,

    /// Transformation stage (e.g., "raw", "cleaned", "enriched")
    pub stage: String,

    /// Schema version for this stage
    pub version: u32,

    /// Warehouse type
    pub warehouse: String,

    /// Database name
    pub database: String,

    /// Schema/dataset name within database
    pub schema_name: String,

    /// Physical table name
    pub table_name: String,

    /// SQL transformation that produces this table
    #[serde(default)]
    pub transformation_sql: Option<String>,

    /// Human-readable name for the transformation
    #[serde(default)]
    pub transformation_name: Option<String>,

    /// Description of what this stage contains
    #[serde(default)]
    pub description: Option<String>,

    /// Table owner
    #[serde(default)]
    pub owner: Option<String>,

    /// When this node was first discovered
    pub created_at: DateTime<Utc>,

    /// When this node was last accessed
    #[serde(default)]
    pub last_accessed: Option<DateTime<Utc>>,

    /// Column names in this stage (for lineage tracking)
    #[serde(default)]
    pub columns: Vec<String>,

    /// Row count (if available)
    #[serde(default)]
    pub row_count: Option<i64>,
}

impl LineageNode {
    pub fn new(
        table_id: String,
        stage: String,
        version: u32,
        warehouse: String,
        database: String,
        schema_name: String,
        table_name: String,
    ) -> Self {
        Self {
            node_id: format!("{}__{}__{}", table_id, stage, version),
            table_id,
            stage,
            version,
            warehouse,
            database,
            schema_name,
            table_name,
            transformation_sql: None,
            transformation_name: None,
            description: None,
            owner: None,
            created_at: Utc::now(),
            last_accessed: None,
            columns: Vec::new(),
            row_count: None,
        }
    }

    /// Full qualified name for this table
    pub fn qualified_name(&self) -> String {
        format!("{}.{}.{}", self.database, self.schema_name, self.table_name)
    }

    /// Unique key for caching/lookup
    pub fn cache_key(&self) -> String {
        format!("{}:{}:v{}", self.table_id, self.stage, self.version)
    }
}

/// A dependency edge from one table to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Unique edge identifier
    pub edge_id: String,

    /// Source node (upstream table)
    pub source_node_id: String,

    /// Target node (downstream table)
    pub target_node_id: String,

    /// Type of join (if applicable)
    #[serde(default)]
    pub join_type: Option<String>,

    /// Columns from source used in target
    #[serde(default)]
    pub columns_used: Vec<String>,

    /// Filter condition applied during transformation
    #[serde(default)]
    pub filter_condition: Option<String>,

    /// Version of this edge
    pub version: u32,

    /// When this dependency was first established
    pub created_at: DateTime<Utc>,

    /// When this edge was last modified
    #[serde(default)]
    pub last_modified: Option<DateTime<Utc>>,

    /// Reason for the last modification
    #[serde(default)]
    pub modification_reason: Option<String>,
}

impl LineageEdge {
    pub fn new(source_node_id: String, target_node_id: String) -> Self {
        Self {
            edge_id: format!("{}__{}", source_node_id, target_node_id),
            source_node_id,
            target_node_id,
            join_type: None,
            columns_used: Vec::new(),
            filter_condition: None,
            version: 1,
            created_at: Utc::now(),
            last_modified: None,
            modification_reason: None,
        }
    }
}

/// Complete DAG of table transformations in a warehouse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    /// Unique warehouse identifier
    pub warehouse_id: String,

    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// All nodes in the graph
    pub nodes: IndexMap<String, LineageNode>,

    /// All edges in the graph
    pub edges: Vec<LineageEdge>,
}

impl LineageGraph {
    pub fn new(warehouse_id: String) -> Self {
        Self {
            warehouse_id,
            timestamp: Utc::now(),
            nodes: IndexMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: LineageNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: LineageEdge) {
        self.edges.push(edge);
    }

    /// Get all upstream tables (tables that feed into target)
    pub fn get_upstream(&self, target_node_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.target_node_id == target_node_id)
            .map(|e| e.source_node_id.clone())
            .collect()
    }

    /// Get all downstream tables (tables that depend on source)
    pub fn get_downstream(&self, source_node_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.source_node_id == source_node_id)
            .map(|e| e.target_node_id.clone())
            .collect()
    }

    /// Compute impact chain: all tables affected if target_node breaks
    pub fn get_impact_chain(&self, target_node_id: &str) -> Vec<String> {
        let mut affected = Vec::new();
        let mut queue = vec![target_node_id.to_string()];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            affected.push(current.clone());

            // Add all downstream tables
            for downstream in self.get_downstream(&current) {
                if !visited.contains(&downstream) {
                    queue.push(downstream);
                }
            }
        }

        affected
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&LineageNode> {
        self.nodes.get(node_id)
    }

    /// Count total nodes and edges
    pub fn size(&self) -> (usize, usize) {
        (self.nodes.len(), self.edges.len())
    }
}

/// A snapshot of the lineage at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageVersion {
    /// Unique version identifier
    pub version_id: String,

    /// Complete lineage graph
    pub lineage_graph: LineageGraph,

    /// Sequential version number
    pub version_number: u32,

    /// When this version was captured
    pub timestamp: DateTime<Utc>,

    /// Schema versions for each table (table_id -> version_number)
    pub schema_versions: HashMap<String, u32>,

    /// Quality scores for each table (table_id -> score 0.0-1.0)
    #[serde(default)]
    pub quality_scores: HashMap<String, f64>,

    /// Changes from previous version
    #[serde(default)]
    pub changes_from_previous: Vec<LineageChange>,

    /// Overall severity of changes
    #[serde(default)]
    pub change_severity: ChangeSeverity,
}

impl LineageVersion {
    pub fn new(warehouse_id: String, version_number: u32, lineage_graph: LineageGraph) -> Self {
        Self {
            version_id: format!("{}__v{}", warehouse_id, version_number),
            lineage_graph,
            version_number,
            timestamp: Utc::now(),
            schema_versions: HashMap::new(),
            quality_scores: HashMap::new(),
            changes_from_previous: Vec::new(),
            change_severity: ChangeSeverity::None,
        }
    }
}

/// A single change in the lineage graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageChange {
    /// Type of change
    pub change_type: ChangeType,

    /// Source table affected
    pub source_table: String,

    /// Target table affected (if applicable)
    #[serde(default)]
    pub target_table: Option<String>,

    /// Why this change was made
    #[serde(default)]
    pub change_reason: Option<String>,

    /// When this change occurred
    pub changed_at: DateTime<Utc>,

    /// Who made the change
    #[serde(default)]
    pub changed_by: Option<String>,

    /// Tables affected by this change
    pub tables_affected: Vec<String>,

    /// Severity of this change
    pub severity: ChangeSeverity,

    /// Does this edge pass schema changes to downstream?
    #[serde(default)]
    pub propagates_schema_changes: bool,
}

impl LineageChange {
    pub fn new(change_type: ChangeType, source_table: String, severity: ChangeSeverity) -> Self {
        Self {
            change_type,
            source_table,
            target_table: None,
            change_reason: None,
            changed_at: Utc::now(),
            changed_by: None,
            tables_affected: Vec::new(),
            severity,
            propagates_schema_changes: false,
        }
    }
}

/// Type of change in lineage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    #[serde(rename = "node_added")]
    NodeAdded,
    #[serde(rename = "node_removed")]
    NodeRemoved,
    #[serde(rename = "edge_added")]
    EdgeAdded,
    #[serde(rename = "edge_removed")]
    EdgeRemoved,
    #[serde(rename = "edge_modified")]
    EdgeModified,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::NodeAdded => write!(f, "node_added"),
            ChangeType::NodeRemoved => write!(f, "node_removed"),
            ChangeType::EdgeAdded => write!(f, "edge_added"),
            ChangeType::EdgeRemoved => write!(f, "edge_removed"),
            ChangeType::EdgeModified => write!(f, "edge_modified"),
        }
    }
}

/// Severity level of changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum ChangeSeverity {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl std::fmt::Display for ChangeSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeSeverity::None => write!(f, "NONE"),
            ChangeSeverity::Low => write!(f, "LOW"),
            ChangeSeverity::Medium => write!(f, "MEDIUM"),
            ChangeSeverity::High => write!(f, "HIGH"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineage_node_creation() {
        let node = LineageNode::new(
            "customers".to_string(),
            "raw".to_string(),
            1,
            "snowflake".to_string(),
            "raw_db".to_string(),
            "public".to_string(),
            "customers_raw".to_string(),
        );

        assert_eq!(node.table_id, "customers");
        assert_eq!(node.stage, "raw");
        assert_eq!(node.version, 1);
        assert_eq!(node.qualified_name(), "raw_db.public.customers_raw");
        assert_eq!(node.cache_key(), "customers:raw:v1");
    }

    #[test]
    fn test_lineage_edge_creation() {
        let edge = LineageEdge::new("node1".to_string(), "node2".to_string());
        assert_eq!(edge.source_node_id, "node1");
        assert_eq!(edge.target_node_id, "node2");
        assert_eq!(edge.version, 1);
    }

    #[test]
    fn test_lineage_graph_operations() {
        let mut graph = LineageGraph::new("warehouse1".to_string());

        let node1 = LineageNode::new(
            "customers".to_string(),
            "raw".to_string(),
            1,
            "snowflake".to_string(),
            "db".to_string(),
            "schema".to_string(),
            "customers_raw".to_string(),
        );

        let node2 = LineageNode::new(
            "customers".to_string(),
            "cleaned".to_string(),
            1,
            "snowflake".to_string(),
            "db".to_string(),
            "schema".to_string(),
            "customers_cleaned".to_string(),
        );

        graph.add_node(node1.clone());
        graph.add_node(node2.clone());

        let edge = LineageEdge::new(node1.node_id.clone(), node2.node_id.clone());
        graph.add_edge(edge);

        let (node_count, edge_count) = graph.size();
        assert_eq!(node_count, 2);
        assert_eq!(edge_count, 1);

        let node1_id = node1.node_id.clone();
        let node2_id = node2.node_id.clone();

        let upstream = graph.get_upstream(&node2_id);
        assert_eq!(upstream, vec![node1_id.clone()]);

        let downstream = graph.get_downstream(&node1_id);
        assert_eq!(downstream, vec![node2.node_id]);
    }

    #[test]
    fn test_impact_chain() {
        let mut graph = LineageGraph::new("warehouse1".to_string());

        // Create chain: A -> B -> C
        let node_a = LineageNode::new(
            "t".to_string(),
            "a".to_string(),
            1,
            "db".to_string(),
            "db".to_string(),
            "s".to_string(),
            "a".to_string(),
        );
        let node_b = LineageNode::new(
            "t".to_string(),
            "b".to_string(),
            1,
            "db".to_string(),
            "db".to_string(),
            "s".to_string(),
            "b".to_string(),
        );
        let node_c = LineageNode::new(
            "t".to_string(),
            "c".to_string(),
            1,
            "db".to_string(),
            "db".to_string(),
            "s".to_string(),
            "c".to_string(),
        );

        let id_a = node_a.node_id.clone();
        let id_b = node_b.node_id.clone();
        let id_c = node_c.node_id.clone();

        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);

        graph.add_edge(LineageEdge::new(id_a.clone(), id_b.clone()));
        graph.add_edge(LineageEdge::new(id_b.clone(), id_c.clone()));

        let impact = graph.get_impact_chain(&id_a);
        assert!(impact.contains(&id_a));
        assert!(impact.contains(&id_b));
        assert!(impact.contains(&id_c));
    }

    #[test]
    fn test_serialization() {
        let node = LineageNode::new(
            "customers".to_string(),
            "raw".to_string(),
            1,
            "snowflake".to_string(),
            "db".to_string(),
            "schema".to_string(),
            "table".to_string(),
        );

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: LineageNode = serde_json::from_str(&json).unwrap();

        assert_eq!(node.node_id, deserialized.node_id);
        assert_eq!(node.table_id, deserialized.table_id);
        assert_eq!(node.stage, deserialized.stage);
    }

    #[test]
    fn test_multi_stage_lineage() {
        // Test: customers_raw -> customers_cleaned -> customers_enriched -> customer_metrics
        let mut graph = LineageGraph::new("warehouse1".to_string());

        let nodes = vec![
            LineageNode::new(
                "customers".to_string(),
                "raw".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "raw".to_string(),
            ),
            LineageNode::new(
                "customers".to_string(),
                "cleaned".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "cleaned".to_string(),
            ),
            LineageNode::new(
                "customers".to_string(),
                "enriched".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "enriched".to_string(),
            ),
            LineageNode::new(
                "metrics".to_string(),
                "aggregated".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "metrics".to_string(),
            ),
        ];

        let node_ids: Vec<_> = nodes.iter().map(|n| n.node_id.clone()).collect();

        for node in nodes {
            graph.add_node(node);
        }

        // Create chain: raw -> cleaned -> enriched -> metrics
        for i in 0..3 {
            graph.add_edge(LineageEdge::new(
                node_ids[i].clone(),
                node_ids[i + 1].clone(),
            ));
        }

        // Test impact: if raw breaks, all downstream affected
        let impact = graph.get_impact_chain(&node_ids[0]);
        assert_eq!(impact.len(), 4);
        assert!(impact.contains(&node_ids[0]));
        assert!(impact.contains(&node_ids[1]));
        assert!(impact.contains(&node_ids[2]));
        assert!(impact.contains(&node_ids[3]));
    }

    #[test]
    fn test_fan_out_lineage() {
        // Test: source -> [path1, path2, path3]
        let mut graph = LineageGraph::new("warehouse1".to_string());

        let source = LineageNode::new(
            "source".to_string(),
            "raw".to_string(),
            1,
            "db".to_string(),
            "db".to_string(),
            "s".to_string(),
            "source".to_string(),
        );

        let targets = vec![
            LineageNode::new(
                "target".to_string(),
                "path1".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "path1".to_string(),
            ),
            LineageNode::new(
                "target".to_string(),
                "path2".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "path2".to_string(),
            ),
            LineageNode::new(
                "target".to_string(),
                "path3".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "path3".to_string(),
            ),
        ];

        let source_id = source.node_id.clone();
        let target_ids: Vec<_> = targets.iter().map(|n| n.node_id.clone()).collect();

        graph.add_node(source);
        for target in targets {
            graph.add_node(target);
        }

        // Create edges from source to all targets
        for target_id in &target_ids {
            graph.add_edge(LineageEdge::new(source_id.clone(), target_id.clone()));
        }

        // Test downstream
        let downstream = graph.get_downstream(&source_id);
        assert_eq!(downstream.len(), 3);
        for target_id in target_ids {
            assert!(downstream.contains(&target_id));
        }

        // Test impact
        let impact = graph.get_impact_chain(&source_id);
        assert_eq!(impact.len(), 4); // source + 3 targets
    }

    #[test]
    fn test_fan_in_lineage() {
        // Test: [source1, source2] -> target
        let mut graph = LineageGraph::new("warehouse1".to_string());

        let sources = vec![
            LineageNode::new(
                "source".to_string(),
                "a".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "a".to_string(),
            ),
            LineageNode::new(
                "source".to_string(),
                "b".to_string(),
                1,
                "db".to_string(),
                "db".to_string(),
                "s".to_string(),
                "b".to_string(),
            ),
        ];

        let target = LineageNode::new(
            "target".to_string(),
            "joined".to_string(),
            1,
            "db".to_string(),
            "db".to_string(),
            "s".to_string(),
            "joined".to_string(),
        );

        let source_ids: Vec<_> = sources.iter().map(|n| n.node_id.clone()).collect();
        let target_id = target.node_id.clone();

        for source in sources {
            graph.add_node(source);
        }
        graph.add_node(target);

        // Create edges from both sources to target
        for source_id in &source_ids {
            graph.add_edge(LineageEdge::new(source_id.clone(), target_id.clone()));
        }

        // Test upstream
        let upstream = graph.get_upstream(&target_id);
        assert_eq!(upstream.len(), 2);
        for source_id in source_ids {
            assert!(upstream.contains(&source_id));
        }
    }

    #[test]
    fn test_lineage_version_creation() {
        let graph = LineageGraph::new("warehouse1".to_string());
        let mut version = LineageVersion::new("warehouse1".to_string(), 1, graph);

        version.schema_versions.insert("customers".to_string(), 5);
        version.quality_scores.insert("customers".to_string(), 0.95);

        assert_eq!(version.version_number, 1);
        assert_eq!(version.schema_versions.get("customers"), Some(&5));
        assert_eq!(version.quality_scores.get("customers"), Some(&0.95));
    }

    #[test]
    fn test_lineage_change_creation() {
        let change = LineageChange::new(
            ChangeType::NodeAdded,
            "customers_raw".to_string(),
            ChangeSeverity::Low,
        );

        assert_eq!(change.change_type, ChangeType::NodeAdded);
        assert_eq!(change.source_table, "customers_raw");
        assert_eq!(change.severity, ChangeSeverity::Low);
    }

    #[test]
    fn test_change_severity_ordering() {
        let none = ChangeSeverity::None;
        let low = ChangeSeverity::Low;
        let medium = ChangeSeverity::Medium;
        let high = ChangeSeverity::High;

        // Verify severity levels
        assert!(none < low);
        assert!(low < medium);
        assert!(medium < high);
    }

    #[test]
    fn test_graph_with_columns() {
        let mut node = LineageNode::new(
            "customers".to_string(),
            "raw".to_string(),
            1,
            "snowflake".to_string(),
            "db".to_string(),
            "schema".to_string(),
            "table".to_string(),
        );

        node.columns = vec!["id".to_string(), "name".to_string(), "email".to_string()];
        node.row_count = Some(1_000_000);

        assert_eq!(node.columns.len(), 3);
        assert_eq!(node.row_count, Some(1_000_000));
    }

    #[test]
    fn test_empty_graph() {
        let graph = LineageGraph::new("warehouse1".to_string());
        let (nodes, edges) = graph.size();
        assert_eq!(nodes, 0);
        assert_eq!(edges, 0);

        let upstream = graph.get_upstream("nonexistent");
        assert!(upstream.is_empty());

        let downstream = graph.get_downstream("nonexistent");
        assert!(downstream.is_empty());
    }
}
