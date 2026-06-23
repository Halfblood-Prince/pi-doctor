# Getting Started

## Install

### Linux install script

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

### Manual release archive

Download the archive for your platform from GitHub Releases, extract it, and
copy `pi-doctor` into a directory on your `PATH`.

### Homebrew

A Homebrew formula template is provided in `packaging/homebrew/pi-doctor.rb.in`
for tap-based publishing.

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
pi-doctor explain throttling
pi-doctor doctor gpio
pi-doctor support-bundle
pi-doctor completions bash
```

## Logging

Runtime diagnostics are emitted through `env_logger`.

```bash
PI_DOCTOR_LOG=warn pi-doctor check
PI_DOCTOR_LOG=debug pi-doctor check
```

Logs are written to stderr and are not part of the stable output contract.

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
