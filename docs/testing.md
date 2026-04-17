# Testing

## Test Layers

The project uses multiple test layers to reduce drift between parser code,
rendered output, and real systems.

### Unit Tests

Parser-focused unit tests cover:

- throttling bitmask parsing
- thermal parsing
- camera inventory parsing
- GPIO/pinctrl parsing
- OS-release parsing

### Fixture-Backed Tests

Fixtures under `fixtures/` simulate realistic Raspberry Pi layouts and captured
command output.

These tests cover:

- report construction across Pi 4 and Pi 5 layouts
- parser tolerance for real captured output
- feature combinations such as externally managed Python and camera detection

### Snapshot Tests

CLI-oriented tests still use snapshots for rendered output, but they are paired
with semantic assertions so behavior changes are harder to hide behind a
snapshot refresh.

### Live Integration Tests

Ignored live integration tests exercise the real host environment:

```bash
cargo test -p pi-doctor --test live_integration -- --ignored
```

These tests are intentionally opt-in and are useful on real Raspberry Pi
hardware when validating behavior against current firmware and OS releases.

## Recommended Validation Before Merging

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If you changed probe logic or host parsing behavior, also run:

```bash
cargo test -p pi-doctor --test live_integration -- --ignored
```
