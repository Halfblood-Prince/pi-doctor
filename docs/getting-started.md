# Getting Started

## Install

### Release archive

```bash
version=0.1.0
target=x86_64-unknown-linux-gnu
curl -fsSLO "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v${version}/pi-doctor-v${version}-${target}.tar.gz"
curl -fsSLO "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v${version}/pi-doctor-v${version}-${target}.tar.gz.sha256"
sha256sum -c "pi-doctor-v${version}-${target}.tar.gz.sha256"
gh attestation verify "pi-doctor-v${version}-${target}.tar.gz" \
  --repo Halfblood-Prince/pi-doctor
tar -xzf "pi-doctor-v${version}-${target}.tar.gz"
mkdir -p ~/.local/bin
install -m 0755 "pi-doctor-v${version}-${target}/pi-doctor" ~/.local/bin/pi-doctor
```

### Verified installer

```bash
curl -fsSLO https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh
bash install.sh --version 0.1.0 --bin-dir ~/.local/bin
```

The installer keeps the previous binary as a rollback point:

```bash
bash install.sh --rollback --bin-dir ~/.local/bin
bash install.sh --uninstall --bin-dir ~/.local/bin
```

### Homebrew

A Homebrew formula is provided in `packaging/homebrew/pi-doctor.rb`. Publish it
only after the matching release archive, checksum, SBOM, and provenance
attestation are available.

### Debian

Debian source packaging is provided in `debian/` for mentors review. Until the
package is accepted into Debian, build and install it locally:

```bash
sudo apt install devscripts equivs build-essential lintian autopkgtest
sudo mk-build-deps --install --remove --tool "apt-get -y" debian/control
dpkg-buildpackage -us -uc -b
sudo apt install ../pi-doctor_0.1.0-1_*.deb
```

## Core Commands

```bash
pi-doctor check
pi-doctor --json check
pi-doctor --timeout 5 check
pi-doctor explain throttling
pi-doctor doctor gpio
pi-doctor support-bundle --dry-run
pi-doctor support-bundle --output ./bundles
pi-doctor completions bash
```

## Support Bundles

Support bundle creation is explicit and shows the collection plan before
writing when `--dry-run` is used.

```bash
pi-doctor support-bundle --dry-run
pi-doctor support-bundle --output ./bundles
```

The default bundle is sanitized and contains `privacy.txt` plus `manifest.txt`
with SHA-256 hashes for payload files. Use sensitive mode only for trusted
expert support:

```bash
pi-doctor support-bundle --include-sensitive --acknowledge-sensitive-data
```

## Logging

Runtime diagnostics are emitted through `env_logger`.

```bash
PI_DOCTOR_LOG=warn pi-doctor check
PI_DOCTOR_LOG=debug pi-doctor check
PI_DOCTOR_LOG=debug PI_DOCTOR_LOG_FORMAT=json pi-doctor --json check > report.json 2> pi-doctor.log.jsonl
```

Logs are written to stderr. Structured logs redact common personal-data and
secret-like patterns before writing the message field, and they do not alter
normal stdout JSON reports.

## Local Docs Preview

The documentation site is built with MkDocs.

```bash
python -m pip install mkdocs mkdocs-material
mkdocs serve
```

Build the static site locally:

```bash
mkdocs build
```
