#!/usr/bin/env bash
set -euo pipefail

repo="Halfblood-Prince/pi-doctor"
bin_dir="${HOME}/.local/bin"
version="latest"
archive_path=""
tmp_dir=""

cleanup() {
  if [[ -n "${tmp_dir}" && -d "${tmp_dir}" ]]; then
    rm -rf "${tmp_dir}"
  fi
}
trap cleanup EXIT

usage() {
  cat <<'EOF'
Usage: install.sh [--version <semver>] [--bin-dir <path>] [--archive <path>]

Installs pi-doctor from a GitHub release archive or a local archive file.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="$2"
      shift 2
      ;;
    --bin-dir)
      bin_dir="$2"
      shift 2
      ;;
    --archive)
      archive_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

detect_target() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64) echo "x86_64-unknown-linux-gnu" ;;
    aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
    armv7l|armv7) echo "armv7-unknown-linux-gnueabihf" ;;
    *)
      echo "unsupported architecture: ${arch}" >&2
      exit 1
      ;;
  esac
}

download_archive() {
  local target release_tag url resolved
  target="$(detect_target)"
  if [[ "$version" == "latest" ]]; then
    resolved="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/${repo}/releases/latest")"
    release_tag="${resolved##*/}"
    version="${release_tag#v}"
    url="https://github.com/${repo}/releases/download/${release_tag}/pi-doctor-v${version}-${target}.tar.gz"
  else
    release_tag="v${version}"
    url="https://github.com/${repo}/releases/download/${release_tag}/pi-doctor-v${version}-${target}.tar.gz"
  fi

  tmp_dir="$(mktemp -d)"
  archive_path="${tmp_dir}/pi-doctor.tar.gz"
  curl -fsSL "$url" -o "$archive_path"
}

if [[ -z "$archive_path" ]]; then
  download_archive
fi

tmp_dir="${tmp_dir:-$(mktemp -d)}"
extract_dir="${tmp_dir}/extract"
mkdir -p "$extract_dir" "$bin_dir"
tar -xzf "$archive_path" -C "$extract_dir"

binary_path="$(find "$extract_dir" -type f -name pi-doctor | head -n 1)"
if [[ -z "$binary_path" ]]; then
  echo "archive did not contain a pi-doctor binary" >&2
  exit 1
fi

install -m 0755 "$binary_path" "${bin_dir}/pi-doctor"
echo "installed pi-doctor to ${bin_dir}/pi-doctor"
