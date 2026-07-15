"""User-friendly error messages for Statguardian."""


class ValidationErrorMessage:
    """Error message for validation failures."""
    
    def __init__(self, title: str, message: str, suggestions: list = None):
        self.title = title
        self.message = message
        self.suggestions = suggestions or []
    
    def format(self) -> str:
        """Format for display."""
        lines = [f"\n❌ {self.title}\n", f"   {self.message}\n"]
        if self.suggestions:
            lines.append("   💡 How to fix:")
            for i, s in enumerate(self.suggestions, 1):
                lines.append(f"      {i}. {s}")
        return "\n".join(lines)
    
    def __str__(self) -> str:
        return self.format()


# Contract DSL errors
MISSING_DATASET_KEYWORD = ValidationErrorMessage(
    title="Invalid Contract: Missing 'dataset' Keyword",
    message="Statguardian contracts must start with 'dataset name { ... }'",
    suggestions=[
        "Add 'dataset your_table_name { ... }' to your contract",
        "Check spelling of 'dataset' keyword",
        "Ensure braces { } are properly matched",
        "Example: dataset orders { schema { order_id: string } }",
    ]
)

SCHEMA_VALIDATION_FAILED = ValidationErrorMessage(
    title="Schema Validation Failed",
    message="Data does not match the defined schema contract.",
    suggestions=[
        "Compare your data columns with the schema definition",
        "Check column data types (string, float, int, etc.)",
        "Ensure required columns are not null",
        "Run: statguardian validate --schema-only data.parquet",
        "Review the full validation report for specific column issues",
    ]
)

DRIFT_DETECTED = ValidationErrorMessage(
    title="Data Drift Detected",
    message="Statistical properties of data have changed beyond acceptable thresholds.",
    suggestions=[
        "Review the drift report to see which columns changed",
        "Check if this is expected (new data distribution, seasonal change)",
        "Update the contract with new statistics if this is normal",
        "Investigate data source for upstream changes",
        "Set higher tolerance in 'stats { amount.mean drift < X }'",
    ]
)

MISSING_REQUIRED_COLUMNS = ValidationErrorMessage(
    title="Missing Required Columns",
    message="Data is missing columns that are required by the contract.",
    suggestions=[
        "Check the 'schema' section of your contract for required columns",
        "Ensure all columns are being selected from source",
        "Verify column names match exactly (case-sensitive)",
        "Example: required columns: order_id, customer_id, amount",
    ]
)

ANOMALY_DETECTED = ValidationErrorMessage(
    title="Anomalies Detected in Data",
    message="The validation found suspicious patterns or outliers.",
    suggestions=[
        "Review the anomaly report to see which rows are flagged",
        "Check if these are valid edge cases or data quality issues",
        "Set IQR threshold higher if sensitive: detect_outliers(amount, iqr=3.0)",
        "Investigate source data for corruption or unexpected changes",
    ]
)


def get_schema_error(missing_cols: list) -> ValidationErrorMessage:
    """Error for missing schema columns."""
    return ValidationErrorMessage(
        title=f"Missing {len(missing_cols)} Required Columns",
        message=f"Data is missing columns: {', '.join(missing_cols)}",
        suggestions=[
            f"Add these columns to your dataset: {', '.join(missing_cols)}",
            "Or modify your contract schema to match your data",
            "Check for column name mismatches (case-sensitive)",
        ]
    )


def get_type_error(column: str, expected: str, actual: str) -> ValidationErrorMessage:
    """Error for type mismatch."""
    return ValidationErrorMessage(
        title=f"Type Mismatch in Column '{column}'",
        message=f"Expected {expected} but found {actual}",
        suggestions=[
            f"Cast column '{column}' to {expected} before validation",
            f"Or update your contract schema: {column}: {actual}",
            "Check if data source is providing correct types",
            f"Example fix: df['{column}'] = df['{column}'].astype('{expected}')",
        ]
    )
