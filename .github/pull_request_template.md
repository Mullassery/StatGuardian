## What does this change?

<!-- One or two sentences: what problem does this solve, or what does it add? -->

## How was this tested?

<!-- Real commands you ran and their actual output, e.g.:
cargo test --workspace --exclude statguardian
pytest tests/
Be specific — "tests pass" without the command/output is not enough. -->

## Checklist

- [ ] `cargo test --workspace --exclude statguardian` passes
- [ ] `cargo clippy --workspace` has no new warnings
- [ ] `cargo fmt --all --check` passes
- [ ] `pytest tests/` passes (if Python code changed)
- [ ] New feature has at least one new test
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Docs updated if behavior, CLI, or the `.sg` DSL grammar changed
