/// StatGuardian Lineage — Core lineage tracking and versioning for data warehouses
///
/// This module provides:
/// - LineageNode: Represents a table at a transformation stage
/// - LineageEdge: Represents dependencies between tables
/// - LineageGraph: Complete DAG of warehouse transformations
/// - LineageVersion: Snapshot of lineage at a point in time
/// - LineageStore: Persistent storage of lineage versions
///
/// Example:
/// ```ignore
/// use statguardian_lineage::*;
///
/// // Create a lineage graph
/// let mut graph = LineageGraph::new("warehouse1".to_string());
///
/// // Add tables
/// let node1 = LineageNode::new(
///     "customers".to_string(),
///     "raw".to_string(),
///     1,
///     "snowflake".to_string(),
///     "raw_db".to_string(),
///     "public".to_string(),
///     "customers_raw".to_string(),
/// );
///
/// let node2 = LineageNode::new(
///     "customers".to_string(),
///     "cleaned".to_string(),
///     1,
///     "snowflake".to_string(),
///     "analytics_db".to_string(),
///     "public".to_string(),
///     "customers_cleaned".to_string(),
/// );
///
/// graph.add_node(node1.clone());
/// graph.add_node(node2.clone());
///
/// // Add edge (dependency)
/// let edge = LineageEdge::new(node1.node_id, node2.node_id);
/// graph.add_edge(edge);
///
/// // Create version
/// let version = LineageVersion::new("warehouse1".to_string(), 1, graph);
/// ```
pub mod model;
pub mod storage;

pub use model::*;
pub use storage::*;

/// Re-exports for convenience
pub mod prelude {
    pub use crate::model::*;
    pub use crate::storage::*;
}
