use crate::model::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use uuid::Uuid;

/// Trait for storing and retrieving lineage data
pub trait LineageStore: Send {
    /// Create or update a lineage version
    fn save_version(&mut self, version: &LineageVersion) -> Result<()>;

    /// Retrieve a specific lineage version
    fn get_version(
        &self,
        warehouse_id: &str,
        version_number: u32,
    ) -> Result<Option<LineageVersion>>;

    /// List all versions for a warehouse
    fn list_versions(&self, warehouse_id: &str, limit: usize) -> Result<Vec<u32>>;

    /// Get the latest version for a warehouse
    fn get_latest_version(&self, warehouse_id: &str) -> Result<Option<LineageVersion>>;

    /// Save a lineage change to the audit trail
    fn save_change(&mut self, change: &LineageChange, version_id: &str) -> Result<()>;

    /// Get changes for a version
    fn get_changes(&self, version_id: &str) -> Result<Vec<LineageChange>>;
}

/// SQLite implementation of LineageStore
pub struct SQLiteLineageStore {
    conn: RwLock<Connection>,
}

impl SQLiteLineageStore {
    /// Create or open a SQLite lineage store
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: RwLock::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Create in-memory store (for testing)
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: RwLock::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.read().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS lineage_versions (
                version_id TEXT PRIMARY KEY,
                warehouse_id TEXT NOT NULL,
                version_number INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                graph_json TEXT NOT NULL,
                schema_versions_json TEXT NOT NULL,
                quality_scores_json TEXT NOT NULL,
                change_severity TEXT NOT NULL,
                created_at TEXT NOT NULL,

                UNIQUE(warehouse_id, version_number)
            );

            CREATE TABLE IF NOT EXISTS lineage_changes (
                change_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                change_type TEXT NOT NULL,
                source_table TEXT NOT NULL,
                target_table TEXT,
                change_reason TEXT,
                changed_at TEXT NOT NULL,
                changed_by TEXT,
                tables_affected_json TEXT NOT NULL,
                severity TEXT NOT NULL,
                propagates_schema_changes INTEGER NOT NULL,

                FOREIGN KEY (version_id) REFERENCES lineage_versions(version_id),
                UNIQUE(version_id, source_table, change_type)
            );

            CREATE INDEX IF NOT EXISTS idx_lineage_warehouse ON lineage_versions(warehouse_id);
            CREATE INDEX IF NOT EXISTS idx_lineage_version_num ON lineage_versions(warehouse_id, version_number DESC);
            CREATE INDEX IF NOT EXISTS idx_lineage_changes ON lineage_changes(version_id);
            "#,
        )?;
        Ok(())
    }
}

impl LineageStore for SQLiteLineageStore {
    fn save_version(&mut self, version: &LineageVersion) -> Result<()> {
        let graph_json = serde_json::to_string(&version.lineage_graph)?;
        let schema_versions_json = serde_json::to_string(&version.schema_versions)?;
        let quality_scores_json = serde_json::to_string(&version.quality_scores)?;

        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO lineage_versions
            (version_id, warehouse_id, version_number, timestamp, graph_json,
             schema_versions_json, quality_scores_json, change_severity, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;

        stmt.execute(params![
            &version.version_id,
            &version.lineage_graph.warehouse_id,
            version.version_number,
            version.timestamp.to_rfc3339(),
            graph_json,
            schema_versions_json,
            quality_scores_json,
            version.change_severity.to_string(),
            Utc::now().to_rfc3339(),
        ])?;

        Ok(())
    }

    fn get_version(
        &self,
        warehouse_id: &str,
        version_number: u32,
    ) -> Result<Option<LineageVersion>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT version_id, version_number, timestamp, graph_json, schema_versions_json,
                    quality_scores_json, change_severity
             FROM lineage_versions
             WHERE warehouse_id = ? AND version_number = ?",
        )?;

        let result = stmt
            .query_row(params![warehouse_id, version_number], |row| {
                let version_id: String = row.get(0)?;
                let version_number: u32 = row.get(1)?;
                let timestamp_str: String = row.get(2)?;
                let graph_json: String = row.get(3)?;
                let schema_versions_json: String = row.get(4)?;
                let quality_scores_json: String = row.get(5)?;
                let change_severity_str: String = row.get(6)?;

                Ok((
                    version_id,
                    version_number,
                    timestamp_str,
                    graph_json,
                    schema_versions_json,
                    quality_scores_json,
                    change_severity_str,
                ))
            })
            .optional()?;

        if let Some((
            version_id,
            version_number,
            timestamp_str,
            graph_json,
            schema_versions_json,
            quality_scores_json,
            change_severity_str,
        )) = result
        {
            let lineage_graph: LineageGraph = serde_json::from_str(&graph_json)?;
            let schema_versions: HashMap<String, u32> =
                serde_json::from_str(&schema_versions_json)?;
            let quality_scores: HashMap<String, f64> = serde_json::from_str(&quality_scores_json)?;

            let change_severity = match change_severity_str.as_str() {
                "NONE" => ChangeSeverity::None,
                "LOW" => ChangeSeverity::Low,
                "MEDIUM" => ChangeSeverity::Medium,
                "HIGH" => ChangeSeverity::High,
                _ => ChangeSeverity::None,
            };

            let timestamp =
                DateTime::parse_from_rfc3339(&timestamp_str).map(|dt| dt.with_timezone(&Utc))?;

            let changes = self.get_changes(&version_id)?;

            Ok(Some(LineageVersion {
                version_id,
                lineage_graph,
                version_number,
                timestamp,
                schema_versions,
                quality_scores,
                changes_from_previous: changes,
                change_severity,
            }))
        } else {
            Ok(None)
        }
    }

    fn list_versions(&self, warehouse_id: &str, limit: usize) -> Result<Vec<u32>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT version_number FROM lineage_versions
             WHERE warehouse_id = ?
             ORDER BY version_number DESC
             LIMIT ?",
        )?;

        let versions = stmt.query_map(params![warehouse_id, limit as i32], |row| {
            row.get::<_, u32>(0)
        })?;

        let mut result = Vec::new();
        for version in versions {
            result.push(version?);
        }

        Ok(result)
    }

    fn get_latest_version(&self, warehouse_id: &str) -> Result<Option<LineageVersion>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT version_number FROM lineage_versions
             WHERE warehouse_id = ?
             ORDER BY version_number DESC
             LIMIT 1",
        )?;

        let latest_version: Option<u32> = stmt
            .query_row(params![warehouse_id], |row| row.get(0))
            .optional()?;

        if let Some(version_number) = latest_version {
            self.get_version(warehouse_id, version_number)
        } else {
            Ok(None)
        }
    }

    fn save_change(&mut self, change: &LineageChange, version_id: &str) -> Result<()> {
        let change_id = format!("{}_{}", version_id, Uuid::new_v4());
        let tables_affected_json = serde_json::to_string(&change.tables_affected)?;

        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO lineage_changes
            (change_id, version_id, change_type, source_table, target_table,
             change_reason, changed_at, changed_by, tables_affected_json, severity,
             propagates_schema_changes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;

        stmt.execute(params![
            change_id,
            version_id,
            change.change_type.to_string(),
            &change.source_table,
            &change.target_table,
            &change.change_reason,
            change.changed_at.to_rfc3339(),
            &change.changed_by,
            tables_affected_json,
            change.severity.to_string(),
            change.propagates_schema_changes as i32,
        ])?;

        Ok(())
    }

    fn get_changes(&self, version_id: &str) -> Result<Vec<LineageChange>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT change_type, source_table, target_table, change_reason,
                    changed_at, changed_by, tables_affected_json, severity,
                    propagates_schema_changes
             FROM lineage_changes
             WHERE version_id = ?",
        )?;

        let changes = stmt.query_map(params![version_id], |row| {
            let change_type_str: String = row.get(0)?;
            let source_table: String = row.get(1)?;
            let target_table: Option<String> = row.get(2)?;
            let change_reason: Option<String> = row.get(3)?;
            let changed_at_str: String = row.get(4)?;
            let changed_by: Option<String> = row.get(5)?;
            let tables_affected_json: String = row.get(6)?;
            let severity_str: String = row.get(7)?;
            let propagates_schema_changes: i32 = row.get(8)?;

            Ok((
                change_type_str,
                source_table,
                target_table,
                change_reason,
                changed_at_str,
                changed_by,
                tables_affected_json,
                severity_str,
                propagates_schema_changes,
            ))
        })?;

        let mut result = Vec::new();
        for change in changes {
            let (
                change_type_str,
                source_table,
                target_table,
                change_reason,
                changed_at_str,
                changed_by,
                tables_affected_json,
                severity_str,
                propagates_schema_changes,
            ) = change?;

            let change_type = match change_type_str.as_str() {
                "node_added" => ChangeType::NodeAdded,
                "node_removed" => ChangeType::NodeRemoved,
                "edge_added" => ChangeType::EdgeAdded,
                "edge_removed" => ChangeType::EdgeRemoved,
                "edge_modified" => ChangeType::EdgeModified,
                _ => ChangeType::NodeAdded,
            };

            let severity = match severity_str.as_str() {
                "NONE" => ChangeSeverity::None,
                "LOW" => ChangeSeverity::Low,
                "MEDIUM" => ChangeSeverity::Medium,
                "HIGH" => ChangeSeverity::High,
                _ => ChangeSeverity::None,
            };

            let changed_at =
                DateTime::parse_from_rfc3339(&changed_at_str).map(|dt| dt.with_timezone(&Utc))?;

            let tables_affected: Vec<String> = serde_json::from_str(&tables_affected_json)?;

            result.push(LineageChange {
                change_type,
                source_table,
                target_table,
                change_reason,
                changed_at,
                changed_by,
                tables_affected,
                severity,
                propagates_schema_changes: propagates_schema_changes != 0,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_store_creation() -> Result<()> {
        let _store = SQLiteLineageStore::memory()?;
        Ok(())
    }

    #[test]
    fn test_save_and_retrieve_version() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        let graph = LineageGraph::new("warehouse1".to_string());
        let version = LineageVersion::new("warehouse1".to_string(), 1, graph);

        store.save_version(&version)?;

        let retrieved = store.get_version("warehouse1", 1)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().version_number, 1);

        Ok(())
    }

    #[test]
    fn test_list_versions() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        for i in 1..=5 {
            let graph = LineageGraph::new("warehouse1".to_string());
            let version = LineageVersion::new("warehouse1".to_string(), i, graph);
            store.save_version(&version)?;
        }

        let versions = store.list_versions("warehouse1", 10)?;
        assert_eq!(versions.len(), 5);
        assert_eq!(versions[0], 5); // Latest first

        Ok(())
    }

    #[test]
    fn test_get_latest_version() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        for i in 1..=3 {
            let graph = LineageGraph::new("warehouse1".to_string());
            let version = LineageVersion::new("warehouse1".to_string(), i, graph);
            store.save_version(&version)?;
        }

        let latest = store.get_latest_version("warehouse1")?;
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version_number, 3);

        Ok(())
    }

    #[test]
    fn test_save_and_retrieve_changes() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        let graph = LineageGraph::new("warehouse1".to_string());
        let version = LineageVersion::new("warehouse1".to_string(), 1, graph);
        store.save_version(&version)?;

        let change = LineageChange::new(
            ChangeType::NodeAdded,
            "customers_raw".to_string(),
            ChangeSeverity::Low,
        );

        store.save_change(&change, &version.version_id)?;

        let changes = store.get_changes(&version.version_id)?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].source_table, "customers_raw");

        Ok(())
    }

    #[test]
    fn test_multiple_versions_same_warehouse() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        // Save 3 versions
        for i in 1..=3 {
            let graph = LineageGraph::new("warehouse1".to_string());
            let version = LineageVersion::new("warehouse1".to_string(), i, graph);
            store.save_version(&version)?;
        }

        let versions = store.list_versions("warehouse1", 10)?;
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0], 3); // Latest first
        assert_eq!(versions[2], 1); // Oldest last

        Ok(())
    }

    #[test]
    fn test_multiple_warehouses() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        // Save versions for different warehouses
        for warehouse_id in &["warehouse1", "warehouse2", "warehouse3"] {
            for version_num in 1..=2 {
                let graph = LineageGraph::new(warehouse_id.to_string());
                let version = LineageVersion::new(warehouse_id.to_string(), version_num, graph);
                store.save_version(&version)?;
            }
        }

        // Verify each warehouse has its own versions
        let v1_versions = store.list_versions("warehouse1", 10)?;
        let v2_versions = store.list_versions("warehouse2", 10)?;
        let v3_versions = store.list_versions("warehouse3", 10)?;

        assert_eq!(v1_versions.len(), 2);
        assert_eq!(v2_versions.len(), 2);
        assert_eq!(v3_versions.len(), 2);

        Ok(())
    }

    #[test]
    fn test_version_with_nodes_and_edges() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

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
        graph.add_edge(LineageEdge::new(
            node1.node_id.clone(),
            node2.node_id.clone(),
        ));

        let version = LineageVersion::new("warehouse1".to_string(), 1, graph);
        store.save_version(&version)?;

        let retrieved = store.get_version("warehouse1", 1)?;
        assert!(retrieved.is_some());

        let retrieved_version = retrieved.unwrap();
        assert_eq!(retrieved_version.lineage_graph.size(), (2, 1));

        Ok(())
    }

    #[test]
    fn test_multiple_changes_per_version() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        let graph = LineageGraph::new("warehouse1".to_string());
        let version = LineageVersion::new("warehouse1".to_string(), 1, graph);
        store.save_version(&version)?;

        // Add multiple changes
        let changes = vec![
            LineageChange::new(
                ChangeType::NodeAdded,
                "customers_raw".to_string(),
                ChangeSeverity::Low,
            ),
            LineageChange::new(
                ChangeType::EdgeAdded,
                "customers_raw".to_string(),
                ChangeSeverity::Medium,
            ),
            LineageChange::new(
                ChangeType::NodeRemoved,
                "old_table".to_string(),
                ChangeSeverity::High,
            ),
        ];

        for change in &changes {
            store.save_change(change, &version.version_id)?;
        }

        let retrieved = store.get_changes(&version.version_id)?;
        assert_eq!(retrieved.len(), 3);
        assert_eq!(retrieved[0].source_table, "customers_raw");
        assert_eq!(retrieved[1].source_table, "customers_raw");
        assert_eq!(retrieved[2].source_table, "old_table");

        Ok(())
    }

    #[test]
    fn test_version_limit() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        // Save 10 versions
        for i in 1..=10 {
            let graph = LineageGraph::new("warehouse1".to_string());
            let version = LineageVersion::new("warehouse1".to_string(), i, graph);
            store.save_version(&version)?;
        }

        // Retrieve only 5
        let versions = store.list_versions("warehouse1", 5)?;
        assert_eq!(versions.len(), 5);
        assert_eq!(versions[0], 10); // Latest
        assert_eq!(versions[4], 6); // Oldest of the 5

        Ok(())
    }

    #[test]
    fn test_nonexistent_version() -> Result<()> {
        let store = SQLiteLineageStore::memory()?;

        let result = store.get_version("warehouse1", 999)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_change_severity_persistence() -> Result<()> {
        let mut store = SQLiteLineageStore::memory()?;

        let graph = LineageGraph::new("warehouse1".to_string());
        let version = LineageVersion::new("warehouse1".to_string(), 1, graph);
        store.save_version(&version)?;

        // Test all severity levels
        let severities = vec![
            ChangeSeverity::None,
            ChangeSeverity::Low,
            ChangeSeverity::Medium,
            ChangeSeverity::High,
        ];

        for (i, severity) in severities.iter().enumerate() {
            let mut change =
                LineageChange::new(ChangeType::NodeAdded, format!("table_{}", i), *severity);
            change.tables_affected = vec![format!("affected_{}", i)];
            store.save_change(&change, &version.version_id)?;
        }

        let changes = store.get_changes(&version.version_id)?;
        assert_eq!(changes.len(), 4);
        assert_eq!(changes[0].severity, ChangeSeverity::None);
        assert_eq!(changes[1].severity, ChangeSeverity::Low);
        assert_eq!(changes[2].severity, ChangeSeverity::Medium);
        assert_eq!(changes[3].severity, ChangeSeverity::High);

        Ok(())
    }
}
