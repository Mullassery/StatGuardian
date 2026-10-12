use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// How a sensitive field is transformed before validation and reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedactionStrategy {
    /// Replace with `"[REDACTED]"`.
    Mask,
    /// Replace with a caller-supplied token.
    MaskWith(String),
    /// Replace with a deterministic, non-reversible FNV-1a hex digest. Stable
    /// for equal inputs, so uniqueness/completeness checks still work.
    Hash,
    /// Replace with JSON `null`.
    Null,
    /// Remove the field entirely.
    Drop,
}

/// A single field-level redaction rule. `field` matches by key name; when the
/// policy is recursive it matches at any depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionRule {
    pub field: String,
    pub strategy: RedactionStrategy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RedactionPolicy {
    pub rules: Vec<RedactionRule>,
    /// When true, redact matching keys at any nesting depth.
    pub recursive: bool,
}

/// Normalization is applied to string values of named fields only. It never
/// changes non-string values and never runs unless a field is listed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NormalizationPolicy {
    pub trim_fields: Vec<String>,
    pub lowercase_fields: Vec<String>,
    pub uppercase_fields: Vec<String>,
    /// When true, match normalization field names at any nesting depth.
    pub recursive: bool,
}

/// A description of one sanitization change. Never contains raw values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizationEvent {
    pub path: String,
    pub kind: SanitizationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanitizationKind {
    Masked,
    Hashed,
    Nulled,
    Dropped,
    Trimmed,
    Lowercased,
    Uppercased,
}

impl SanitizationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SanitizationKind::Masked => "masked",
            SanitizationKind::Hashed => "hashed",
            SanitizationKind::Nulled => "nulled",
            SanitizationKind::Dropped => "dropped",
            SanitizationKind::Trimmed => "trimmed",
            SanitizationKind::Lowercased => "lowercased",
            SanitizationKind::Uppercased => "uppercased",
        }
    }
}

/// Aggregate, leak-safe view of all sanitization that occurred in a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizationSummary {
    pub total_events: u64,
    pub by_kind: BTreeMap<String, u64>,
    pub by_field: BTreeMap<String, u64>,
}

impl SanitizationSummary {
    pub fn record(&mut self, event: &SanitizationEvent) {
        self.total_events += 1;
        *self
            .by_kind
            .entry(event.kind.as_str().to_string())
            .or_insert(0) += 1;
        let leaf = event
            .path
            .split(['.', '['])
            .filter(|s| !s.is_empty())
            .next_back()
            .unwrap_or(&event.path)
            .to_string();
        *self.by_field.entry(leaf).or_insert(0) += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.total_events == 0
    }
}

/// Applies a redaction policy and a normalization policy to JSON records.
#[derive(Debug, Clone, Default)]
pub struct Sanitizer {
    redaction: RedactionPolicy,
    normalization: NormalizationPolicy,
}

impl Sanitizer {
    pub fn new(redaction: RedactionPolicy, normalization: NormalizationPolicy) -> Self {
        Self {
            redaction,
            normalization,
        }
    }

    pub fn redaction(&self) -> &RedactionPolicy {
        &self.redaction
    }

    pub fn normalization(&self) -> &NormalizationPolicy {
        &self.normalization
    }

    /// True when the policy would never change any record.
    pub fn is_noop(&self) -> bool {
        self.redaction.rules.is_empty()
            && self.normalization.trim_fields.is_empty()
            && self.normalization.lowercase_fields.is_empty()
            && self.normalization.uppercase_fields.is_empty()
    }

    /// Returns a sanitized copy of `value` plus one event per change. The
    /// input is never mutated.
    pub fn sanitize(&self, value: &Value) -> (Value, Vec<SanitizationEvent>) {
        let mut events = Vec::new();
        if self.is_noop() {
            return (value.clone(), events);
        }
        let mut owned = value.clone();
        self.walk(&mut owned, "", &mut events);
        (owned, events)
    }

    fn redaction_for(&self, key: &str) -> Option<&RedactionStrategy> {
        self.redaction
            .rules
            .iter()
            .find(|r| r.field == key)
            .map(|r| &r.strategy)
    }

    fn walk(&self, value: &mut Value, path: &str, events: &mut Vec<SanitizationEvent>) {
        match value {
            Value::Object(map) => {
                let mut drop_keys: Vec<String> = Vec::new();
                for (key, child) in map.iter_mut() {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };

                    if let Some(strategy) = self.redaction_for(key) {
                        match strategy {
                            RedactionStrategy::Drop => {
                                drop_keys.push(key.clone());
                                events.push(SanitizationEvent {
                                    path: child_path,
                                    kind: SanitizationKind::Dropped,
                                });
                                continue;
                            }
                            RedactionStrategy::Mask => {
                                *child = Value::String("[REDACTED]".to_string());
                                events.push(SanitizationEvent {
                                    path: child_path,
                                    kind: SanitizationKind::Masked,
                                });
                                continue;
                            }
                            RedactionStrategy::MaskWith(token) => {
                                *child = Value::String(token.clone());
                                events.push(SanitizationEvent {
                                    path: child_path,
                                    kind: SanitizationKind::Masked,
                                });
                                continue;
                            }
                            RedactionStrategy::Null => {
                                *child = Value::Null;
                                events.push(SanitizationEvent {
                                    path: child_path,
                                    kind: SanitizationKind::Nulled,
                                });
                                continue;
                            }
                            RedactionStrategy::Hash => {
                                *child = Value::String(hash_value(child));
                                events.push(SanitizationEvent {
                                    path: child_path,
                                    kind: SanitizationKind::Hashed,
                                });
                                continue;
                            }
                        }
                    }

                    self.normalize_value(key, child, &child_path, events);

                    if self.redaction.recursive || self.normalization.recursive {
                        self.walk(child, &child_path, events);
                    }
                }
                for key in drop_keys {
                    map.remove(&key);
                }
            }
            Value::Array(arr) => {
                if self.redaction.recursive || self.normalization.recursive {
                    for (i, child) in arr.iter_mut().enumerate() {
                        self.walk(child, &format!("{path}[{i}]"), events);
                    }
                }
            }
            _ => {}
        }
    }

    fn normalize_value(
        &self,
        key: &str,
        value: &mut Value,
        path: &str,
        events: &mut Vec<SanitizationEvent>,
    ) {
        let Value::String(s) = value else {
            return;
        };

        if self.normalization.trim_fields.iter().any(|f| f == key) {
            let trimmed = s.trim().to_string();
            if trimmed != *s {
                *s = trimmed;
                events.push(SanitizationEvent {
                    path: path.to_string(),
                    kind: SanitizationKind::Trimmed,
                });
            }
        }
        if self.normalization.lowercase_fields.iter().any(|f| f == key) {
            let lower = s.to_lowercase();
            if lower != *s {
                *s = lower;
                events.push(SanitizationEvent {
                    path: path.to_string(),
                    kind: SanitizationKind::Lowercased,
                });
            }
        }
        if self.normalization.uppercase_fields.iter().any(|f| f == key) {
            let upper = s.to_uppercase();
            if upper != *s {
                *s = upper;
                events.push(SanitizationEvent {
                    path: path.to_string(),
                    kind: SanitizationKind::Uppercased,
                });
            }
        }
    }
}

/// Deterministic FNV-1a 64-bit hash, hex-encoded. Not cryptographic — it is a
/// stable pseudonymizer for redaction, not a security primitive.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash_value(value: &Value) -> String {
    let material = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    format!("{:016x}", fnv1a_64(material.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sanitizer(rules: Vec<RedactionRule>, recursive: bool) -> Sanitizer {
        Sanitizer::new(
            RedactionPolicy { rules, recursive },
            NormalizationPolicy::default(),
        )
    }

    #[test]
    fn mask_replaces_value_and_reports_event() {
        let s = sanitizer(
            vec![RedactionRule {
                field: "ssn".into(),
                strategy: RedactionStrategy::Mask,
            }],
            false,
        );
        let input = json!({"ssn": "123-45-6789", "name": "Ada"});
        let (out, events) = s.sanitize(&input);
        assert_eq!(out["ssn"], "[REDACTED]");
        assert_eq!(out["name"], "Ada");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, SanitizationKind::Masked);
        // Original untouched.
        assert_eq!(input["ssn"], "123-45-6789");
    }

    #[test]
    fn hash_is_deterministic_and_value_hiding() {
        let s = sanitizer(
            vec![RedactionRule {
                field: "email".into(),
                strategy: RedactionStrategy::Hash,
            }],
            false,
        );
        let (a, _) = s.sanitize(&json!({"email": "a@x.com"}));
        let (b, _) = s.sanitize(&json!({"email": "a@x.com"}));
        let (c, _) = s.sanitize(&json!({"email": "b@y.com"}));
        assert_eq!(a["email"], b["email"]);
        assert_ne!(a["email"], c["email"]);
        assert!(!a["email"].as_str().unwrap().contains("a@x.com"));
    }

    #[test]
    fn drop_removes_field() {
        let s = sanitizer(
            vec![RedactionRule {
                field: "token".into(),
                strategy: RedactionStrategy::Drop,
            }],
            false,
        );
        let (out, _) = s.sanitize(&json!({"token": "secret", "id": 1}));
        assert!(out.get("token").is_none());
        assert_eq!(out["id"], 1);
    }

    #[test]
    fn recursive_redaction_hits_nested_fields() {
        let s = sanitizer(
            vec![RedactionRule {
                field: "password".into(),
                strategy: RedactionStrategy::Mask,
            }],
            true,
        );
        let (out, events) = s.sanitize(&json!({"user": {"password": "hunter2"}}));
        assert_eq!(out["user"]["password"], "[REDACTED]");
        assert_eq!(events[0].path, "user.password");
    }

    #[test]
    fn non_recursive_leaves_nested_untouched() {
        let s = sanitizer(
            vec![RedactionRule {
                field: "password".into(),
                strategy: RedactionStrategy::Mask,
            }],
            false,
        );
        let (out, events) = s.sanitize(&json!({"user": {"password": "hunter2"}}));
        assert_eq!(out["user"]["password"], "hunter2");
        assert!(events.is_empty());
    }

    #[test]
    fn normalization_trims_and_lowercases() {
        let s = Sanitizer::new(
            RedactionPolicy::default(),
            NormalizationPolicy {
                trim_fields: vec!["city".into()],
                lowercase_fields: vec!["email".into()],
                uppercase_fields: vec!["country".into()],
                recursive: false,
            },
        );
        let (out, events) = s.sanitize(&json!({
            "city": "  New York  ",
            "email": "USER@Example.COM",
            "country": "us"
        }));
        assert_eq!(out["city"], "New York");
        assert_eq!(out["email"], "user@example.com");
        assert_eq!(out["country"], "US");
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn noop_policy_returns_identical_value_without_events() {
        let s = Sanitizer::default();
        assert!(s.is_noop());
        let input = json!({"a": 1});
        let (out, events) = s.sanitize(&input);
        assert_eq!(out, input);
        assert!(events.is_empty());
    }

    #[test]
    fn summary_aggregates_without_raw_values() {
        let mut summary = SanitizationSummary::default();
        let s = sanitizer(
            vec![RedactionRule {
                field: "ssn".into(),
                strategy: RedactionStrategy::Mask,
            }],
            true,
        );
        let (_, events) = s.sanitize(&json!({"ssn": "1", "meta": {"ssn": "2"}}));
        for e in &events {
            summary.record(e);
        }
        assert_eq!(summary.total_events, 2);
        assert_eq!(summary.by_kind["masked"], 2);
        assert_eq!(summary.by_field["ssn"], 2);
    }
}
