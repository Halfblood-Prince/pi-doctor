# Development

## Local Workflow

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Useful Test Commands

Run only the CLI crate:

```bash
cargo test -p pi-doctor
```

Run only the report crate:

```bash
cargo test -p pi-doctor-report
```

Run ignored live integration tests manually:

```bash
cargo test -p pi-doctor --test live_integration -- --ignored
```

## Release Artifacts

Tagged releases publish:

- target-specific archives for `x86_64-unknown-linux-gnu`
- target-specific archives for `aarch64-unknown-linux-gnu`
- target-specific archives for `armv7-unknown-linux-gnueabihf`
- per-archive `.sha256` files
- a combined `SHA256SUMS` manifest
- the `install.sh` helper

## Documentation

The docs site is generated from `docs/` with MkDocs.

```bash
mkdocs serve
mkdocs build
```
