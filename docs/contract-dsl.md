# Contract DSL (`.sg`)

StatGuardian contracts are declarative `.sg` files. They are parsed and
executed by the Rust core. This page documents the syntax shown in the
repository's [`examples/`](../examples/) — the parser is always the source of
truth; run `statguardian check <file.sg>` to validate a contract.

## Structure

```text
dataset <name> {
    schema    { ... }
    quality   { ... }
    stats     { ... }
    anomalies { ... }
    stream    { ... }   // optional
}
```

Only `schema` is typically present; the other blocks are optional.

## `schema`

```text
schema {
    id:         int,    not_null, unique, primary_key
    email:      string, regex="^[^@]+@[^@]+\.[^@]+$"
    age:        int,    between(0, 120)
    country:    string, not_null, enum=["US","UK","DE"]
    score:      float,  min=0.0, max=1.0
    code:       string, len(10, 50)
    amount:     float,  positive
    created_at: datetime
}
```

Field modifiers seen in the examples:

| Modifier | Meaning |
|---|---|
| `not_null` | no nulls |
| `unique` | values unique |
| `primary_key` | key field |
| `regex="..."` | string pattern |
| `enum=[...]` | allowed values |
| `between(a, b)` | inclusive range |
| `min=`, `max=` | numeric bounds |
| `len(a, b)` | string length bounds |
| `positive` | greater than zero |

Types shown: `int`, `float`, `string`, `datetime`.

## `quality`

```text
quality {
    completeness(id)    > 0.999
    uniqueness(email)   == 1.0
    @blocking: uniqueness(id) == 1.0
    @warning:  uniqueness(email) == 1.0
}
```

- `completeness(field)` and `uniqueness(field)` are the quality metrics shown.
- `@blocking:` marks a check as blocking (failure gates the pipeline).
- `@warning:` downgrades a check to a warning.

## `stats` (drift)

```text
stats {
    age.mean   drift < 0.10
    age.std    drift < 0.20
    amount.p95 drift < 0.25
}
```

Drift compares incoming data with `--reference`. The examples use
`mean`, `std`, and `p95` metrics with a `drift < threshold` expression.

## `anomalies`

```text
anomalies {
    detect_outliers(age, method="iqr")
    detect_outliers(score, method="zscore")
    detect_duplicates(id)
    detect_cardinality_explosion(currency)
    @blocking: detect_duplicates(order_id)
}
```

Rules shown: `detect_outliers` (`iqr` or `zscore`), `detect_duplicates`, and
`detect_cardinality_explosion`.

## `stream` (optional)

```text
stream {
    window    = "5m"
    watermark = "30s"
    emit      = "on_window_close"
}
```

Streaming configuration is used by StreamGuard; see
[StreamGuard](STREAMGUARD.md).

## Validate a contract and data

```bash
statguardian check contracts/orders.sg
statguardian validate --contract contracts/orders.sg --file orders.parquet
statguardian validate --contract contracts/orders.sg --file incoming.parquet --reference baseline.parquet
```

See [`examples/basic_validation.sg`](../examples/basic_validation.sg) and
[`examples/ecommerce_orders.sg`](../examples/ecommerce_orders.sg) for complete
contracts.
