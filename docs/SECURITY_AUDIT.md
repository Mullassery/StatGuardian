# Statguardian Security Audit

**Last Audited:** July 2026  
**Status:** SQL injection patterns found; Rust safety unclear

---

## 🔴 CRITICAL Vulnerabilities

### 1. SQL Injection Patterns (9 instances)
**Location:** `python/statguardian/_connectors.py` and validation logic  
**Risk:** Attackers can execute arbitrary SQL, extract/modify data  
**Severity:** CRITICAL  

**Finding:** Dynamic SQL construction patterns in database connectors
```python
# VULNERABLE pattern found
# Construction of Databricks/database queries without parameterization
```

**Impact:** Users providing malicious contract DSL or dataset paths could exploit this

**Recommended Fix:**
- Audit all database connector queries
- Use parameterized queries exclusively
- Validate DSL contract syntax before SQL generation

**Timeline:** v1.0.1 (Q3 2026) — Immediate security patch

---

## 🟡 HIGH Priority Issues

### 2. No Dependency Version Pinning
**Location:** `pyproject.toml`  
**Severity:** HIGH  
**Finding:** 0 pinned versions, 39 floating versions  

**Critical Dependencies (vulnerable versions exist):**
- `pyarrow` — Has known vulnerabilities in older versions
- `polars` — Version coupling with abi3 wheels
- `pandas` — Security patches important

**Action Items:**
```toml
# Current (VULNERABLE)
pyarrow = ">=10.0.0"
polars = "~=0.18"

# Recommended (SAFE)
pyarrow = "14.0.1"
polars = "0.18.19"
```

**Timeline:** v1.0.1 (Q3 2026) — Pin all versions

---

### 3. Environment Variable Secrets
**Location:** `python/statguardian/_connectors.py`  
**Risk:** AWS credentials, database passwords in environment  
**Severity:** HIGH  

**Finding:** Documentation mentions:
```
AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY  (in environment)
```

**Recommendation:**
- Use AWS IAM roles (not long-term credentials)
- Validate that secrets are never logged
- Document secure credential handling

**Timeline:** v1.1.0 (Q3 2026) — Security guide for deployment

---

## 🔵 MEDIUM Priority Issues

### 4. Rust Unsafe Blocks — Safety Unclear
**Location:** Rust codebase (src/)  
**Risk:** Memory safety, buffer overflows  
**Severity:** MEDIUM-HIGH  

**Action Items:**
- [ ] Audit all `unsafe` blocks in Rust code
- [ ] Verify bounds checking
- [ ] Run `cargo audit` for dependencies
- [ ] Run `miri` (interpreter for detecting UB)

**Command:**
```bash
cargo audit
cargo miri test  # Detect undefined behavior
```

**Timeline:** v1.0.1 (Q3 2026) — Run security audit

---

### 5. No Input Validation on DSL
**Risk:** Malformed contract files could crash validator or cause DoS  
**Severity:** MEDIUM  

**Recommendation:**
- Validate DSL schema before processing
- Set limits on file size, recursion depth
- Graceful error messages (not stack traces)

**Timeline:** v1.1.0 (Q3 2026)

---

### 6. Broad Exception Handling
**Risk:** Silent failures in validation could miss real errors  
**Severity:** MEDIUM  

**Timeline:** v1.1.0 (Q3 2026)

---

## 🔵 LOW Priority

### 7. No Secrets Scanning in CI
**Recommendation:** Add `truffleHog` to GitHub Actions  
**Timeline:** v1.0.2 (Q3 2026)

### 8. Documentation: No Security Deployment Guide
**Recommendation:** Add guide for:
- Secure AWS credential handling
- Database connection security
- Principle of least privilege for database user

**Timeline:** v1.1.0 (Q3 2026)

---

## Security Roadmap

| Issue | Severity | Target | Effort |
|-------|----------|--------|--------|
| Audit SQL injection | CRITICAL | v1.0.1 | 2 days |
| Pin dependencies | HIGH | v1.0.1 | 1 day |
| Audit Rust unsafe blocks | MEDIUM | v1.0.1 | 1 day |
| Secrets handling guide | HIGH | v1.1.0 | 1 day |
| DSL input validation | MEDIUM | v1.1.0 | 1 day |
| Exception handling review | MEDIUM | v1.1.0 | 1 day |
| CI secrets scanning | LOW | v1.0.2 | 0.5 days |

---

## Testing Recommendations

1. **Dependency Audit:**
   ```bash
   cargo audit
   pip-audit
   ```

2. **SAST for Rust:**
   ```bash
   cargo clippy -- -W clippy::all
   cargo miri test
   ```

3. **SQL Injection Testing:**
   - Manual code review
   - Fuzzing with malicious DSL inputs

4. **Input Validation Testing:**
   - Large/recursive contracts
   - Malformed SQL

---

## Deployment Recommendations

- Use IAM roles instead of long-term AWS credentials
- Run database user with minimal permissions (no DROP, no CREATE)
- Enable query logging for audit trail
- Never commit `.env` files (use `.env.example`)
