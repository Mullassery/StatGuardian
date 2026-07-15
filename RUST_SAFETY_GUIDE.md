# Statguardian Rust Safety Guide

## Running Security Audits

### Cargo Audit (Check for Vulnerable Dependencies)
```bash
# Install if needed
cargo install cargo-audit

# Run audit
cargo audit

# Only fail on high/critical severity
cargo audit --deny warnings
```

### Miri (Detect Undefined Behavior)
```bash
# Install if needed
rustup +nightly component add miri

# Run tests with Miri
MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test

# This detects:
- Buffer overflows
- Use-after-free
- Data races
- Invalid pointer arithmetic
```

### Clippy (Linting and Best Practices)
```bash
# Run clippy
cargo clippy --all-targets --all-features -- -W clippy::all

# This catches:
- Memory safety issues
- Performance problems
- Style violations
- Common mistakes
```

### AddressSanitizer (Runtime Memory Safety)
```bash
# For Linux only
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test

# Detects:
- Use-after-free
- Memory leaks
- Double-free
- Buffer overflows
```

## Unsafe Block Guidelines

When unsafe is absolutely necessary:

1. Document why it's needed (safety comment)
2. Specify what invariants must hold
3. Document the safety preconditions
4. Test with Miri

Example:
```rust
// SAFETY: We know the pointer is valid because:
// 1. It comes from a Box that's still alive
// 2. The memory was allocated on the heap
// 3. We're not invalidating it during access
unsafe {
    let data = *ptr as &[u8];
}
```

## Security Best Practices

1. **No Unsafe in Hot Paths**: Avoid unsafe if possible, especially in performance-critical code
2. **Minimal Unsafe Blocks**: Keep unsafe code as small as possible
3. **Panic = Safe**: Panics in unsafe code are safe (no undefined behavior)
4. **Test Thoroughly**: Run all three tools above (audit, miri, clippy)
5. **Peer Review**: Have unsafe code reviewed by team members

## CI/CD Integration

Add to `.github/workflows/security.yml`:

```yaml
name: Security Checks

on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: rustsec/audit-check-action@v1

  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          components: miri
      - run: MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo clippy --all-targets -- -D warnings
```

## References

- [The Rustonomicon (Unsafe Code)](https://doc.rust-lang.org/nomicon/)
- [Miri Documentation](https://github.com/rust-lang/miri)
- [Cargo Audit](https://github.com/rustsec/cargo-audit)
