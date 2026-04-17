# pi-doctor

Human-first Raspberry Pi diagnostics.

`pi-doctor` is a read-only CLI for checking common Raspberry Pi problems such as
identity mismatches, thermal throttling, `config.txt` drift, GPIO conflicts,
camera detection, and Python environment pitfalls.

## Install

### Linux install script

```bash
curl -fsSL https://github.com/example/pi-doctor/releases/latest/download/install.sh | sh
```

Install into a custom directory:

```bash
curl -fsSL https://github.com/example/pi-doctor/releases/latest/download/install.sh | sh -s -- --bin-dir ~/.local/bin
```

Install a specific version:

```bash
curl -fsSL https://github.com/example/pi-doctor/releases/latest/download/install.sh | sh -s -- --version 0.1.0
```

### Manual release archive

Download the archive for your platform from GitHub Releases, extract it, and
copy `pi-doctor` into a directory on your `PATH`.

### Homebrew

A Homebrew formula template is provided in
`packaging/homebrew/pi-doctor.rb.in` for tap-based publishing.

### Debian packaging

Debian package metadata scaffolding is provided in `packaging/debian/` so the
project can be wrapped as a `.deb` once release automation and distro review are
ready.

## Usage

```bash
pi-doctor check
pi-doctor --json check
pi-doctor explain throttling
pi-doctor doctor gpio
pi-doctor support-bundle
```

## Documentation

Project documentation lives under `docs/` and is published as an MkDocs site.

Local preview:

```bash
python -m pip install mkdocs mkdocs-material
mkdocs serve
```

## CLI Contract

Automation should prefer `pi-doctor check --json`. The public CLI guarantees are
documented in [docs/cli-contract.md](docs/cli-contract.md), with supporting
details in [docs/exit-codes.md](docs/exit-codes.md) and
[docs/json-schema.md](docs/json-schema.md).

## Release artifacts

Tagged releases publish:

- target-specific archives for `x86_64-unknown-linux-gnu`
- target-specific archives for `aarch64-unknown-linux-gnu`
- target-specific archives for `armv7-unknown-linux-gnueabihf`
- per-archive `.sha256` files
- a combined `SHA256SUMS` manifest
- the `install.sh` helper

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
