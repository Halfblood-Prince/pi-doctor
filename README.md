# pi-doctor

[![CI](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/ci.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/ci.yml)
[![Docs](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/docs.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/docs.yml)
[![Release](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/release.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/release.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

`pi-doctor` is a read-only CLI for checking common Raspberry Pi problems:
identity mismatches, thermal throttling, `config.txt` drift, GPIO conflicts,
camera detection failures, and Python environment pitfalls.

It is designed for both interactive troubleshooting and automation. Human
output explains what changed and what to run next; JSON output gives scripts
stable schemas. The current stable package release is `1.0.0`.

## Install

### Release Archive

```bash
version=1.0.0
target=x86_64-unknown-linux-gnu
curl -fsSLO "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v${version}/pi-doctor-v${version}-${target}.tar.gz"
curl -fsSLO "https://github.com/Halfblood-Prince/pi-doctor/releases/download/v${version}/pi-doctor-v${version}-${target}.tar.gz.sha256"
sha256sum -c "pi-doctor-v${version}-${target}.tar.gz.sha256"
tar -tzf "pi-doctor-v${version}-${target}.tar.gz"
tar -xzf "pi-doctor-v${version}-${target}.tar.gz"
mkdir -p ~/.local/bin
install -m 0755 "pi-doctor-v${version}-${target}/pi-doctor" ~/.local/bin/pi-doctor
```

Verify release provenance when GitHub CLI is available:

```bash
gh attestation verify "pi-doctor-v${version}-${target}.tar.gz" \
  --repo Halfblood-Prince/pi-doctor
```

Releases also publish `SHA256SUMS`, `SHA256SUMS.sha256`, and provenance
attestations for the checksum files.

### Verified Installer

```bash
curl -fsSLO https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh
curl -fsSLO https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh.sha256
sha256sum -c install.sh.sha256
gh attestation verify install.sh --repo Halfblood-Prince/pi-doctor
bash install.sh --version 1.0.0 --bin-dir ~/.local/bin
```

The manual archive route above has the smallest installer trust base: download
the archive, checksum, and attestation yourself before extracting the binary.

Use an explicit target such as `aarch64-unknown-linux-gnu` for Raspberry Pi OS
64-bit, `armv7-unknown-linux-gnueabihf` for 32-bit ARM, or the matching `musl`
target when you want the static build.

Rollback and uninstall are handled by the installer:

```bash
bash install.sh --rollback --bin-dir ~/.local/bin
bash install.sh --uninstall --bin-dir ~/.local/bin
```

### Debian

Debian packaging lives in [`debian/`](debian/) and is prepared for mentors
review. Until the package is accepted into Debian, build it locally:

```bash
sudo apt install devscripts equivs build-essential lintian autopkgtest
sudo mk-build-deps --install --remove --tool "apt-get -y" debian/control
dpkg-buildpackage -us -uc -b
sudo apt install ../pi-doctor_1.0.0_*.deb
```

Once accepted into Debian, installation will be the normal apt path:

```bash
sudo apt install pi-doctor
```

### Homebrew

A Homebrew formula is provided at
[`packaging/homebrew/pi-doctor.rb`](packaging/homebrew/pi-doctor.rb) and should
be published only after the matching signed release artifacts exist.

## Usage

```bash
pi-doctor check
pi-doctor --json check
pi-doctor --timeout 5 check
pi-doctor explain throttling
pi-doctor explain config
pi-doctor explain python
pi-doctor doctor camera
pi-doctor doctor gpio
pi-doctor support-bundle --dry-run
pi-doctor support-bundle --output ./bundles
```

Support bundles are sanitized by default and include `manifest.txt` with
SHA-256 hashes for payload files. Sensitive bundles require an explicit
acknowledgement:

```bash
pi-doctor support-bundle --include-sensitive --acknowledge-sensitive-data
```

`pi-doctor check` exits with:

| Code | Meaning |
| ---: | --- |
| 0 | healthy |
| 1 | warning |
| 2 | degraded |
| 3 | critical |
| 4 | unexpected runtime failure |

Automation should prefer:

```bash
pi-doctor --json check
```

JSON includes `probe_health` so scripts can tell a healthy subsystem from an
incomplete inspection.

Operational logs are written to stderr. For CI or field debugging, use
privacy-preserving JSON-lines logs:

```bash
PI_DOCTOR_LOG=debug PI_DOCTOR_LOG_FORMAT=json pi-doctor --json check > report.json 2> pi-doctor.log.jsonl
```

The JSON contract is documented in [`docs/cli-contract.md`](docs/cli-contract.md),
[`docs/json-schema.md`](docs/json-schema.md), and the schema files in
[`schema/`](schema/).

## Project Operations

- Security policy: [`SECURITY.md`](SECURITY.md)
- Contributing guide: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Maintainer policy: [`MAINTAINERS.md`](MAINTAINERS.md)
- Release process: [`docs/release-process.md`](docs/release-process.md)
- Known limitations: [`docs/known-limitations.md`](docs/known-limitations.md)

## License

`pi-doctor` is licensed under the [Apache License 2.0](LICENSE).
