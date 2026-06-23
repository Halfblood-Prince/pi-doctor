# pi-doctor

[![CI](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/ci.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/ci.yml)
[![Docs](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/docs.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/docs.yml)
[![Release](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/release.yml/badge.svg)](https://github.com/Halfblood-Prince/pi-doctor/actions/workflows/release.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

`pi-doctor` is a read-only CLI for checking common Raspberry Pi problems:
identity mismatches, thermal throttling, `config.txt` drift, GPIO conflicts,
camera detection failures, and Python environment pitfalls.

It is designed for both interactive troubleshooting and automation. Human
output explains what changed and what to run next; JSON output gives scripts a
stable schema.

## Install

### Release Script

```bash
curl -fsSL https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh | sh
```

Install into a custom directory:

```bash
curl -fsSL https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh | sh -s -- --bin-dir ~/.local/bin
```

Install a specific version:

```bash
curl -fsSL https://github.com/Halfblood-Prince/pi-doctor/releases/latest/download/install.sh | sh -s -- --version 0.1.0
```

### Manual Archive

Download a release archive from
[GitHub Releases](https://github.com/Halfblood-Prince/pi-doctor/releases),
extract it, and place `pi-doctor` on your `PATH`.

### Debian

Debian packaging lives in [`debian/`](debian/) and is prepared for mentors
review. Until the package is accepted into Debian, build it locally:

```bash
sudo apt install devscripts equivs build-essential lintian autopkgtest
sudo mk-build-deps --install --remove --tool "apt-get -y" debian/control
dpkg-buildpackage -us -uc -b
sudo apt install ../pi-doctor_0.1.0-1_*.deb
```

Once accepted into Debian, installation will be the normal apt path:

```bash
sudo apt install pi-doctor
```

### Homebrew

A Homebrew formula template is provided at
[`packaging/homebrew/pi-doctor.rb.in`](packaging/homebrew/pi-doctor.rb.in).

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
pi-doctor support-bundle
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

The JSON contract is documented in [`docs/cli-contract.md`](docs/cli-contract.md)
and [`docs/json-schema.md`](docs/json-schema.md).

## License

`pi-doctor` is licensed under the [Apache License 2.0](LICENSE).
