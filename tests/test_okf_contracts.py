"""Tests for OKF contract management in StatGuardian."""

import tempfile
from pathlib import Path

import pytest

from statguardian.okf_contracts import (
    OKFAnomalyPattern,
    OKFContractCatalog,
    OKFContractDocument,
    OKFRuleEffectiveness,
)


@pytest.fixture
def temp_catalog():
    """Create temporary catalog directory."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield OKFContractCatalog(Path(tmpdir))


@pytest.fixture
def populated_catalog(temp_catalog):
    """Create catalog with sample contracts."""
    catalog = temp_catalog

    # Save customer contract
    catalog.save_contract(
        contract_name="customers",
        schema={
            "customer_id": {"type": "integer", "nullable": False},
            "email": {"type": "string", "nullable": False},
            "created_at": {"type": "timestamp", "nullable": False},
            "status": {"type": "string", "enum": ["active", "inactive"]},
        },
        rules=[
            {
                "name": "no_nulls",
                "type": "completeness",
                "severity": "error",
                "description": "No null values allowed",
            },
            {
                "name": "unique_email",
                "type": "uniqueness",
                "severity": "error",
                "description": "Email must be unique",
            },
        ],
        baseline={
            "row_count": 1000000,
            "columns": 4,
            "null_percentage": 0.0,
        },
        metadata={"tags": ["crm", "production"], "owner": "analytics-team"},
    )

    # Save orders contract
    catalog.save_contract(
        contract_name="orders",
        schema={
            "order_id": {"type": "integer", "nullable": False},
            "customer_id": {"type": "integer", "nullable": False},
            "amount": {"type": "decimal", "nullable": False},
        },
        rules=[
            {
                "name": "positive_amount",
                "type": "range",
                "severity": "error",
                "description": "Amount must be positive",
            },
        ],
        baseline={"row_count": 500000},
        metadata={"tags": ["commerce", "production"]},
    )

    return catalog


class TestOKFContractDocument:
    """Test contract document loading."""

    def test_load_contract(self, populated_catalog):
        """Test loading a contract document."""
        contract = populated_catalog.get_contract("customers")
        assert contract is not None

    def test_contract_properties(self, populated_catalog):
        """Test accessing contract properties."""
        contract = populated_catalog.get_contract("customers")
        assert contract.contract_name == "customers"
        assert "customer_id" in contract.schema
        assert len(contract.rules) == 2


class TestOKFContractCatalog:
    """Test catalog operations."""

    def test_search_contracts(self, populated_catalog):
        """Test searching contracts."""
        results = populated_catalog.search_contracts("customers")
        assert len(results) >= 1
        assert results[0].contract_name == "customers"

    def test_search_by_tag(self, populated_catalog):
        """Test searching by tag."""
        results = populated_catalog.search_contracts("production")
        assert len(results) >= 1

    def test_get_baseline(self, populated_catalog):
        """Test retrieving baseline."""
        baseline = populated_catalog.get_baseline("customers")
        assert baseline is not None
        assert baseline["row_count"] == 1000000

    def test_save_contract(self, temp_catalog):
        """Test saving a new contract."""
        path = temp_catalog.save_contract(
            contract_name="test_table",
            schema={"id": {"type": "integer"}},
            rules=[{"name": "test_rule", "type": "format"}],
        )

        assert path.exists()
        assert "contracts" in str(path)

    def test_update_baseline(self, populated_catalog):
        """Test updating baseline."""
        new_baseline = {"row_count": 1100000, "columns": 4}
        populated_catalog.save_baseline("customers", new_baseline)

        updated = populated_catalog.get_baseline("customers")
        assert updated["row_count"] == 1100000

    def test_catalog_structure(self, temp_catalog):
        """Test that catalog creates proper structure."""
        expected_dirs = ["contracts", "baselines", "rules", "anomalies"]
        for subdir in expected_dirs:
            assert (temp_catalog.catalog_dir / subdir).exists()


class TestOKFRuleEffectiveness:
    """Test rule execution tracking."""

    def test_record_rule_execution(self, temp_catalog):
        """Test recording rule execution."""
        tracker = OKFRuleEffectiveness(temp_catalog.catalog_dir)

        path = tracker.record_rule_execution(
            contract_name="customers",
            rule_name="no_nulls",
            passed=True,
            caught_issues=0,
        )

        assert path.exists()

    def test_record_failed_execution(self, temp_catalog):
        """Test recording failed execution."""
        tracker = OKFRuleEffectiveness(temp_catalog.catalog_dir)

        path = tracker.record_rule_execution(
            contract_name="customers",
            rule_name="unique_email",
            passed=False,
            caught_issues=5,
        )

        assert path.exists()

    def test_get_success_rate(self, temp_catalog):
        """Test calculating success rate."""
        tracker = OKFRuleEffectiveness(temp_catalog.catalog_dir)

        # Record multiple executions
        tracker.record_rule_execution("customers", "no_nulls", True)
        tracker.record_rule_execution("customers", "no_nulls", True)
        tracker.record_rule_execution("customers", "no_nulls", False)

        rate = tracker.get_success_rate("customers", "no_nulls")
        assert rate is not None
        assert abs(rate - 2/3) < 0.01  # ~66% success rate

    def test_no_history(self, temp_catalog):
        """Test when no execution history exists."""
        tracker = OKFRuleEffectiveness(temp_catalog.catalog_dir)
        rate = tracker.get_success_rate("nonexistent", "rule")
        assert rate is None


class TestOKFAnomalyPattern:
    """Test anomaly detection tracking."""

    def test_record_anomaly(self, temp_catalog):
        """Test recording an anomaly."""
        tracker = OKFAnomalyPattern(temp_catalog.catalog_dir)

        path = tracker.record_anomaly(
            contract_name="customers",
            anomaly_type="duplicate",
            severity="error",
            description="Duplicate email addresses detected",
            affected_rows=42,
        )

        assert path.exists()

    def test_drift_anomaly(self, temp_catalog):
        """Test recording drift anomaly."""
        tracker = OKFAnomalyPattern(temp_catalog.catalog_dir)

        path = tracker.record_anomaly(
            contract_name="customers",
            anomaly_type="drift",
            severity="warning",
            description="Column distribution has shifted 15%",
            affected_rows=150000,
        )

        assert path.exists()

    def test_get_recurring_anomalies(self, temp_catalog):
        """Test identifying recurring anomalies."""
        tracker = OKFAnomalyPattern(temp_catalog.catalog_dir)

        # Record duplicate anomalies multiple times
        for i in range(3):
            tracker.record_anomaly(
                contract_name="customers",
                anomaly_type="duplicate",
                severity="error",
                description=f"Duplicate detected run {i}",
                affected_rows=10 + i,
            )

        # Record drift anomaly once
        tracker.record_anomaly(
            contract_name="customers",
            anomaly_type="drift",
            severity="warning",
            description="Drift detected",
            affected_rows=100,
        )

        recurring = tracker.get_recurring_anomalies("customers", min_occurrences=2)

        assert len(recurring) >= 1
        duplicates = [r for r in recurring if r["anomaly_type"] == "duplicate"]
        assert len(duplicates) > 0
        assert duplicates[0]["occurrences"] == 3

    def test_anomaly_filtering(self, temp_catalog):
        """Test filtering by minimum occurrences."""
        tracker = OKFAnomalyPattern(temp_catalog.catalog_dir)

        # Record anomalies
        for i in range(2):
            tracker.record_anomaly(
                contract_name="customers",
                anomaly_type="type_a",
                severity="error",
                description=f"Type A {i}",
            )

        tracker.record_anomaly(
            contract_name="customers",
            anomaly_type="type_b",
            severity="error",
            description="Type B",
        )

        # Only recurring with >= 2 occurrences
        recurring = tracker.get_recurring_anomalies("customers", min_occurrences=2)
        assert all(r["occurrences"] >= 2 for r in recurring)
