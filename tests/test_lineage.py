"""Tests for data lineage tracking (statguardian._lineage).

get_lineage_graph()/get_lineage_version()/get_lineage_history() used to be
stubs that always returned an empty graph / None / [] regardless of what was
recorded, discarding the warehouse_config entirely. The underlying Rust
storage engine (statguardian-lineage) was already real and tested -- it was
just never wired up to the Python-facing functions. These tests exercise the
real round trip through the compiled extension and a real SQLite file.
"""

import pytest

pytest.importorskip("statguardian")

import statguardian as sg


@pytest.fixture
def warehouse_config(tmp_path):
    return {
        "warehouse_id": "test_warehouse",
        "lineage_db_path": str(tmp_path / "lineage.db"),
    }


def _build_two_stage_graph(warehouse_id):
    graph = sg.LineageGraph(warehouse_id=warehouse_id, timestamp="2026-01-01T00:00:00Z")
    raw = sg.create_lineage_node("customers", "raw", 1, "snowflake", "raw_db", "public", "customers_raw")
    cleaned = sg.create_lineage_node(
        "customers", "cleaned", 1, "snowflake", "analytics_db", "public", "customers_cleaned"
    )
    graph.add_node(raw)
    graph.add_node(cleaned)
    graph.add_edge(sg.create_lineage_edge(raw.node_id, cleaned.node_id))
    return graph, raw, cleaned


class TestLineageRoundTrip:
    def test_get_lineage_graph_is_empty_before_any_version_saved(self, warehouse_config):
        graph = sg.get_lineage_graph(warehouse_config)

        assert graph.warehouse_id == "test_warehouse"
        assert graph.size() == (0, 0)

    def test_saved_graph_is_retrievable_with_real_nodes_and_edges(self, warehouse_config):
        graph, raw, cleaned = _build_two_stage_graph("test_warehouse")

        saved = sg.save_lineage_version(warehouse_config, graph)
        assert saved.version_number == 1

        retrieved = sg.get_lineage_graph(warehouse_config)

        assert retrieved.size() == (2, 1)
        assert raw.node_id in retrieved.nodes
        assert cleaned.node_id in retrieved.nodes
        assert retrieved.get_upstream(cleaned.node_id) == [raw.node_id]
        assert retrieved.get_downstream(raw.node_id) == [cleaned.node_id]

    def test_version_numbers_auto_increment(self, warehouse_config):
        graph1, _, _ = _build_two_stage_graph("test_warehouse")
        v1 = sg.save_lineage_version(warehouse_config, graph1)
        assert v1.version_number == 1

        graph2, _, _ = _build_two_stage_graph("test_warehouse")
        v2 = sg.save_lineage_version(warehouse_config, graph2)
        assert v2.version_number == 2

        latest = sg.get_lineage_version(warehouse_config)
        assert latest.version_number == 2

    def test_get_specific_version_by_number(self, warehouse_config):
        graph1, _, _ = _build_two_stage_graph("test_warehouse")
        sg.save_lineage_version(warehouse_config, graph1)

        graph2, _, _ = _build_two_stage_graph("test_warehouse")
        sg.save_lineage_version(warehouse_config, graph2)

        v1 = sg.get_lineage_version(warehouse_config, version=1)
        assert v1.version_number == 1

    def test_get_lineage_version_returns_none_when_nothing_saved(self, warehouse_config):
        assert sg.get_lineage_version(warehouse_config) is None
        assert sg.get_lineage_version(warehouse_config, version=1) is None

    def test_separate_warehouses_do_not_share_lineage(self, tmp_path):
        db_path = str(tmp_path / "lineage.db")
        config_a = {"warehouse_id": "warehouse_a", "lineage_db_path": db_path}
        config_b = {"warehouse_id": "warehouse_b", "lineage_db_path": db_path}

        graph_a, _, _ = _build_two_stage_graph("warehouse_a")
        sg.save_lineage_version(config_a, graph_a)

        assert sg.get_lineage_graph(config_a).size() == (2, 1)
        assert sg.get_lineage_graph(config_b).size() == (0, 0)


class TestLineageChangesAndHistory:
    def test_change_severity_is_derived_from_worst_change(self, warehouse_config):
        graph, raw, cleaned = _build_two_stage_graph("test_warehouse")
        changes = [
            sg.LineageChange(
                change_type="node_added", source_table="customers_raw", severity="low"
            ),
            sg.LineageChange(
                change_type="edge_removed",
                source_table="customers_raw",
                target_table="customers_cleaned",
                severity="high",
                tables_affected=["customers_cleaned"],
            ),
        ]

        saved = sg.save_lineage_version(warehouse_config, graph, changes=changes)

        assert saved.change_severity == "high"
        retrieved = sg.get_lineage_version(warehouse_config)
        assert len(retrieved.changes_from_previous) == 2

    def test_history_only_includes_versions_that_touched_the_table(self, warehouse_config):
        graph, _, _ = _build_two_stage_graph("test_warehouse")

        # v1: touches customers_raw
        sg.save_lineage_version(
            warehouse_config,
            graph,
            changes=[
                sg.LineageChange(
                    change_type="node_added", source_table="customers_raw", severity="low"
                )
            ],
        )
        # v2: touches an unrelated table
        sg.save_lineage_version(
            warehouse_config,
            graph,
            changes=[
                sg.LineageChange(
                    change_type="node_added", source_table="orders_raw", severity="low"
                )
            ],
        )
        # v3: affects customers_raw via tables_affected
        sg.save_lineage_version(
            warehouse_config,
            graph,
            changes=[
                sg.LineageChange(
                    change_type="edge_modified",
                    source_table="orders_raw",
                    severity="medium",
                    tables_affected=["customers_raw"],
                )
            ],
        )

        history = sg.get_lineage_history(warehouse_config, "customers_raw", limit=10)

        versions = {v.version_number for v in history}
        assert versions == {1, 3}

    def test_history_respects_limit_by_scanning_only_recent_versions(self, warehouse_config):
        graph, _, _ = _build_two_stage_graph("test_warehouse")

        for i in range(5):
            sg.save_lineage_version(
                warehouse_config,
                graph,
                changes=[
                    sg.LineageChange(
                        change_type="node_added", source_table="customers_raw", severity="low"
                    )
                ],
            )

        history = sg.get_lineage_history(warehouse_config, "customers_raw", limit=2)

        assert len(history) == 2
        assert {v.version_number for v in history} == {4, 5}


class TestImpactChain:
    def test_impact_chain_reflects_real_saved_topology(self, warehouse_config):
        graph, raw, cleaned = _build_two_stage_graph("test_warehouse")
        enriched = sg.create_lineage_node(
            "customers", "enriched", 1, "snowflake", "analytics_db", "public", "customers_enriched"
        )
        graph.add_node(enriched)
        graph.add_edge(sg.create_lineage_edge(cleaned.node_id, enriched.node_id))

        sg.save_lineage_version(warehouse_config, graph)

        retrieved = sg.get_lineage_graph(warehouse_config)
        impact = retrieved.get_impact_chain(raw.node_id)

        assert set(impact) == {raw.node_id, cleaned.node_id, enriched.node_id}
