"""OKF (Open Knowledge Format) contract management for StatGuardian.

Stores and manages data quality contracts, drift baselines, and rule effectiveness
as portable, shareable OKF documents.
"""

import json
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    import frontmatter
except ImportError:
    raise ImportError(
        "python-frontmatter is required for OKF support. "
        "Install with: pip install python-frontmatter"
    )


class OKFContractDocument:
    """Represents a data quality contract as an OKF document."""

    def __init__(self, path: Path):
        """Load contract document from disk.

        Args:
            path: Path to .md file with contract definition
        """
        self.path = path
        self.post = frontmatter.load(str(path))
        self.metadata = self.post.metadata
        self.content = self.post.content

    @property
    def contract_name(self) -> str:
        """Contract name (e.g., 'customers', 'orders')."""
        return self.metadata.get("contract_name", self.path.stem)

    @property
    def version(self) -> str:
        """Contract version."""
        return self.metadata.get("version", "1.0.0")

    @property
    def schema(self) -> Dict[str, Any]:
        """Schema definition."""
        return self.metadata.get("schema", {})

    @property
    def rules(self) -> List[Dict[str, Any]]:
        """Quality rules."""
        return self.metadata.get("rules", [])

    @property
    def baseline(self) -> Optional[Dict[str, Any]]:
        """Statistical baseline (distribution, metrics)."""
        return self.metadata.get("baseline")

    @property
    def tags(self) -> List[str]:
        """Contract tags (domain, team, etc.)."""
        return self.metadata.get("tags", [])

    def get_metadata(self, key: str, default: Any = None) -> Any:
        """Get arbitrary metadata field."""
        return self.metadata.get(key, default)


class OKFContractCatalog:
    """Catalog of data quality contracts stored as OKF documents."""

    def __init__(self, catalog_dir: Path):
        """Initialize catalog from directory.

        Args:
            catalog_dir: Root directory containing OKF contract documents
        """
        self.catalog_dir = Path(catalog_dir)
        self.contracts: Dict[str, OKFContractDocument] = {}
        self.contract_index: Dict[str, List[str]] = {}  # domain -> [contract_ids]
        self._ensure_structure()
        self._load_all()

    def _ensure_structure(self) -> None:
        """Create catalog directory structure if missing."""
        subdirs = [
            "contracts",
            "baselines",
            "rules",
            "anomalies",
        ]
        self.catalog_dir.mkdir(parents=True, exist_ok=True)
        for subdir in subdirs:
            (self.catalog_dir / subdir).mkdir(exist_ok=True)

    def _load_all(self) -> None:
        """Load all contract documents from disk."""
        self.contracts.clear()
        self.contract_index.clear()

        for doc_path in self.catalog_dir.glob("contracts/*.md"):
            try:
                doc = OKFContractDocument(doc_path)
                contract_id = doc.contract_name
                self.contracts[contract_id] = doc

                # Index by domain/tags
                for tag in doc.tags:
                    if tag not in self.contract_index:
                        self.contract_index[tag] = []
                    self.contract_index[tag].append(contract_id)
            except Exception as e:
                print(f"Warning: Failed to load {doc_path}: {e}")

    def search_contracts(self, query: str = "*") -> List[OKFContractDocument]:
        """Find contracts by name, tag, or domain.

        Args:
            query: Search term or "*" for all

        Returns:
            List of matching contracts
        """
        results = []
        query_lower = query.lower() if query != "*" else ""

        for contract in self.contracts.values():
            if query == "*":
                results.append(contract)
            elif (query_lower in contract.contract_name.lower() or
                  any(query_lower in tag for tag in contract.tags)):
                results.append(contract)

        return results

    def get_contract(self, contract_name: str) -> Optional[OKFContractDocument]:
        """Get contract by name.

        Args:
            contract_name: Contract identifier

        Returns:
            Contract document or None
        """
        return self.contracts.get(contract_name)

    def get_baseline(self, contract_name: str) -> Optional[Dict[str, Any]]:
        """Get statistical baseline for a contract.

        Args:
            contract_name: Contract identifier

        Returns:
            Baseline metrics or None
        """
        contract = self.contracts.get(contract_name)
        if contract:
            return contract.baseline
        return None

    def save_contract(self, contract_name: str, schema: Dict[str, Any],
                     rules: List[Dict[str, Any]],
                     baseline: Optional[Dict[str, Any]] = None,
                     metadata: Optional[Dict[str, Any]] = None) -> Path:
        """Save a data quality contract.

        Args:
            contract_name: Unique contract identifier
            schema: Schema definition (fields, types)
            rules: Quality rules to enforce
            baseline: Statistical baseline (optional)
            metadata: Additional metadata (tags, version, etc.)

        Returns:
            Path to saved document
        """
        if metadata is None:
            metadata = {}

        # Build schema documentation
        schema_md = "## Schema\n```json\n"
        schema_md += json.dumps(schema, indent=2)
        schema_md += "\n```\n"

        # Build rules documentation
        rules_md = "## Quality Rules\n"
        for i, rule in enumerate(rules, 1):
            rules_md += f"### Rule {i}: {rule.get('name', 'Unnamed')}\n"
            rules_md += f"- Type: {rule.get('type', 'unknown')}\n"
            rules_md += f"- Severity: {rule.get('severity', 'error')}\n"
            if rule.get('description'):
                rules_md += f"- Description: {rule['description']}\n"
            rules_md += "\n"

        # Build baseline documentation
        baseline_md = ""
        if baseline:
            baseline_md = "## Baseline\n```json\n"
            baseline_md += json.dumps(baseline, indent=2, default=str)
            baseline_md += "\n```\n"

        content = f"""# Data Quality Contract: {contract_name}

{schema_md}{rules_md}{baseline_md}
"""

        # Create frontmatter metadata
        fm_metadata = {
            "type": "data-contract",
            "contract_name": contract_name,
            "schema": schema,
            "rules": rules,
            "timestamp": datetime.now().isoformat(),
        }

        if baseline:
            fm_metadata["baseline"] = baseline

        if metadata:
            fm_metadata.update(metadata)

        # Save OKF document
        doc_path = self.catalog_dir / "contracts" / f"{contract_name}.md"

        post = frontmatter.Post(content)
        post.metadata = fm_metadata
        doc_path.write_text(frontmatter.dumps(post))

        # Reload to sync index
        self._load_all()

        return doc_path

    def save_baseline(self, contract_name: str, baseline: Dict[str, Any]) -> Path:
        """Save or update baseline statistics for a contract.

        Args:
            contract_name: Contract identifier
            baseline: Baseline statistics (mean, std, quartiles, etc.)

        Returns:
            Path to saved baseline
        """
        contract = self.contracts.get(contract_name)
        if not contract:
            raise ValueError(f"Contract '{contract_name}' not found")

        # Update contract with new baseline
        metadata = contract.metadata.copy()
        metadata["baseline"] = baseline
        metadata["baseline_updated"] = datetime.now().isoformat()

        doc_path = self.catalog_dir / "contracts" / f"{contract_name}.md"

        post = frontmatter.Post(contract.content)
        post.metadata = metadata
        doc_path.write_text(frontmatter.dumps(post))

        self._load_all()
        return doc_path

    def reload(self) -> None:
        """Reload catalog from disk."""
        self._load_all()


class OKFRuleEffectiveness:
    """Track effectiveness of validation rules across runs."""

    def __init__(self, catalog_dir: Path):
        """Initialize rule effectiveness tracker.

        Args:
            catalog_dir: Root catalog directory
        """
        self.catalog_dir = Path(catalog_dir)
        self.rules_dir = self.catalog_dir / "rules"
        self.rules_dir.mkdir(parents=True, exist_ok=True)

    def record_rule_execution(self, contract_name: str, rule_name: str,
                             passed: bool, caught_issues: int = 0) -> Path:
        """Record a rule execution result.

        Args:
            contract_name: Contract being validated
            rule_name: Rule that was executed
            passed: Whether validation passed
            caught_issues: Number of issues caught (if failed)

        Returns:
            Path to recorded execution
        """
        content = f"""# Rule Execution

**Contract:** {contract_name}
**Rule:** {rule_name}
**Result:** {'✅ PASSED' if passed else '❌ FAILED'}
**Issues Caught:** {caught_issues}
"""

        metadata = {
            "type": "rule-execution",
            "contract_name": contract_name,
            "rule_name": rule_name,
            "passed": passed,
            "issues_caught": caught_issues,
            "timestamp": datetime.now().isoformat(),
        }

        # Save execution record
        rule_file = self.rules_dir / f"{contract_name}_{rule_name}.md"

        post = frontmatter.Post(content)
        post.metadata = metadata
        rule_file.write_text(frontmatter.dumps(post))

        return rule_file

    def get_rule_success_rate(self, contract_name: str, rule_name: str) -> Optional[float]:
        """Calculate historical success rate for a rule.

        Args:
            contract_name: Contract identifier
            rule_name: Rule name

        Returns:
            Success rate (0-1) or None if no history
        """
        pattern = f"{contract_name}_{rule_name}*.md"
        executions = list(self.rules_dir.glob(pattern))

        if not executions:
            return None

        passed_count = 0
        for exec_file in executions:
            try:
                post = frontmatter.load(str(exec_file))
                if post.metadata.get("passed", False):
                    passed_count += 1
            except Exception:
                pass

        return passed_count / len(executions) if executions else None


class OKFAnomalyPattern:
    """Track recurring data quality anomalies."""

    def __init__(self, catalog_dir: Path):
        """Initialize anomaly pattern tracker.

        Args:
            catalog_dir: Root catalog directory
        """
        self.catalog_dir = Path(catalog_dir)
        self.anomalies_dir = self.catalog_dir / "anomalies"
        self.anomalies_dir.mkdir(parents=True, exist_ok=True)

    def record_anomaly(self, contract_name: str, anomaly_type: str,
                      severity: str, description: str,
                      affected_rows: int = 0) -> Path:
        """Record a detected anomaly.

        Args:
            contract_name: Contract where anomaly was detected
            anomaly_type: Type of anomaly (drift, duplicate, outlier, etc.)
            severity: Severity level (warning, error, critical)
            description: Description of the anomaly
            affected_rows: Number of affected rows

        Returns:
            Path to saved anomaly record
        """
        content = f"""# Anomaly Detection

**Contract:** {contract_name}
**Type:** {anomaly_type}
**Severity:** {severity}
**Affected Rows:** {affected_rows}

## Description
{description}
"""

        metadata = {
            "type": "anomaly-detection",
            "contract_name": contract_name,
            "anomaly_type": anomaly_type,
            "severity": severity,
            "affected_rows": affected_rows,
            "timestamp": datetime.now().isoformat(),
        }

        # Save anomaly record
        anomaly_file = self.anomalies_dir / f"{contract_name}_{anomaly_type}_{datetime.now().timestamp()}.md"

        post = frontmatter.Post(content)
        post.metadata = metadata
        anomaly_file.write_text(frontmatter.dumps(post))

        return anomaly_file

    def get_recurring_anomalies(self, contract_name: str,
                               min_occurrences: int = 2) -> List[Dict[str, Any]]:
        """Identify recurring anomalies for a contract.

        Args:
            contract_name: Contract identifier
            min_occurrences: Minimum occurrences to be considered recurring

        Returns:
            List of recurring anomaly patterns
        """
        anomaly_counts: Dict[str, int] = {}
        anomaly_details: Dict[str, List[Dict[str, Any]]] = {}

        pattern = f"{contract_name}_*.md"
        for anomaly_file in self.anomalies_dir.glob(pattern):
            try:
                post = frontmatter.load(str(anomaly_file))
                anomaly_type = post.metadata.get("anomaly_type", "unknown")

                if anomaly_type not in anomaly_counts:
                    anomaly_counts[anomaly_type] = 0
                    anomaly_details[anomaly_type] = []

                anomaly_counts[anomaly_type] += 1
                anomaly_details[anomaly_type].append(post.metadata)
            except Exception:
                pass

        # Filter by minimum occurrences
        recurring = []
        for anomaly_type, count in anomaly_counts.items():
            if count >= min_occurrences:
                recurring.append({
                    "anomaly_type": anomaly_type,
                    "occurrences": count,
                    "severity": max(d.get("severity") for d in anomaly_details[anomaly_type]),
                    "total_affected_rows": sum(
                        d.get("affected_rows", 0) for d in anomaly_details[anomaly_type]
                    ),
                })

        return sorted(recurring, key=lambda x: x["occurrences"], reverse=True)
