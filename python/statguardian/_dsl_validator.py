"""DSL validation to prevent crashes and DoS attacks."""

import re
from typing import Tuple


def validate_dsl_contract(dsl_content: str) -> Tuple[bool, str]:
    """
    Validate DSL contract syntax before processing.
    
    Args:
        dsl_content: DSL contract content
        
    Returns:
        (is_valid, error_message)
    """
    if not dsl_content or not dsl_content.strip():
        return False, "Empty contract"
    
    # Check size limit (1MB max)
    if len(dsl_content) > 1_000_000:
        return False, "Contract too large (max 1MB)"
    
    # Check for basic DSL structure
    if 'dataset' not in dsl_content:
        return False, "Contract must contain 'dataset' keyword"
    
    # Check nesting depth (prevent stack overflow in parser)
    max_depth = 0
    current_depth = 0
    for char in dsl_content:
        if char in '{([':
            current_depth += 1
            max_depth = max(max_depth, current_depth)
        elif char in '})]':
            current_depth -= 1
    
    if max_depth > 50:
        return False, f"Nesting too deep (max 50, got {max_depth})"
    
    # Check for suspicious patterns
    suspicious = [
        (r'__.*__', 'Double underscore (reserved)'),
        (r';\s*;', 'Multiple semicolons'),
        (r'\x00', 'Null byte'),
    ]
    
    for pattern, reason in suspicious:
        if re.search(pattern, dsl_content):
            return False, f"Suspicious pattern: {reason}"
    
    return True, ""


def validate_dataset_name(name: str) -> Tuple[bool, str]:
    """Validate dataset name in contract."""
    if not name or not re.match(r'^[a-zA-Z_][a-zA-Z0-9_]*$', name):
        return False, f"Invalid dataset name: {name}"
    
    if len(name) > 256:
        return False, "Dataset name too long (max 256 chars)"
    
    return True, ""
