#!/usr/bin/env bash
set -euo pipefail

repo="Halfblood-Prince/pi-doctor"
bin_dir="${HOME}/.local/bin"
version="latest"
target=""
archive_path=""
checksum_path=""
archive_supplied="false"
skip_attestation="false"
tmp_dir=""
rollback="false"
uninstall="false"

cleanup() {
  if [[ -n "${tmp_dir}" && -d "${tmp_dir}" ]]; then
    rm -rf "${tmp_dir}"
  fi
}
trap cleanup EXIT

usage() {
  cat <<'EOF'
Usage: install.sh [--version <semver>] [--target <triple>] [--bin-dir <path>]
                  [--archive <path> --checksum <path> [--skip-attestation]]
                  [--rollback | --uninstall]

Installs pi-doctor from a verified GitHub release archive or a verified local
archive. Local archives require a matching --checksum file. Attestation can be
skipped only for explicitly supplied local archives.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="$2"
      shift 2
      ;;
    --target)
      target="$2"
      shift 2
      ;;
    --bin-dir)
      bin_dir="$2"
      shift 2
      ;;
    --archive)
      archive_path="$2"
      archive_supplied="true"
      shift 2
      ;;
    --checksum)
      checksum_path="$2"
      shift 2
      ;;
    --skip-attestation)
      skip_attestation="true"
      shift
      ;;
    --rollback)
      rollback="true"
      shift
      ;;
    --uninstall)
      uninstall="true"
      shift
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

backup_path() {
  echo "${bin_dir}/.pi-doctor.previous"
}

rollback_binary() {
  local backup tmp_binary
  backup="$(backup_path)"
  if [[ ! -f "$backup" ]]; then
    echo "no rollback binary found at $backup" >&2
    exit 1
  fi
  mkdir -p "$bin_dir"
  tmp_binary="$(mktemp "${bin_dir}/.pi-doctor.rollback.XXXXXX")"
  install -m 0755 "$backup" "$tmp_binary"
  mv -f "$tmp_binary" "${bin_dir}/pi-doctor"
  echo "rolled back pi-doctor using $backup"
}

uninstall_binary() {
  rm -f "${bin_dir}/pi-doctor"
  echo "removed ${bin_dir}/pi-doctor"
}

resolve_latest_version() {
  local resolved release_tag
  resolved="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/${repo}/releases/latest")"
  release_tag="${resolved##*/}"
  version="${release_tag#v}"
}

download_release_files() {
  local release_tag url checksum_url
  if [[ -z "$target" ]]; then
    target="$(detect_target)"
  fi
  if [[ "$version" == "latest" ]]; then
    resolve_latest_version
  fi

  release_tag="v${version}"
  url="https://github.com/${repo}/releases/download/${release_tag}/pi-doctor-v${version}-${target}.tar.gz"
  checksum_url="${url}.sha256"

  tmp_dir="$(mktemp -d)"
  archive_path="${tmp_dir}/pi-doctor-v${version}-${target}.tar.gz"
  checksum_path="${archive_path}.sha256"
  curl -fsSL "$url" -o "$archive_path"
  curl -fsSL "$checksum_url" -o "$checksum_path"
}

infer_local_archive_metadata() {
  local base candidate
  base="$(basename "$archive_path")"
  for candidate in \
    x86_64-unknown-linux-gnu \
    x86_64-unknown-linux-musl \
    aarch64-unknown-linux-gnu \
    aarch64-unknown-linux-musl \
    armv7-unknown-linux-gnueabihf \
    armv7-unknown-linux-musleabihf
  do
    if [[ "$base" == pi-doctor-v*-"${candidate}".tar.gz ]]; then
      target="$candidate"
      version="${base#pi-doctor-v}"
      version="${version%-${candidate}.tar.gz}"
      break
    fi
  done
  if [[ -z "$target" ]]; then
    target="$(detect_target)"
  fi
  if [[ "$version" == "latest" ]]; then
    echo "local archive name must include a concrete version, or pass --version" >&2
    exit 1
  fi
}

verify_checksum() {
  if [[ -z "$checksum_path" || ! -f "$checksum_path" ]]; then
    echo "missing checksum file; pass --checksum for local archives" >&2
    exit 1
  fi

  local expected actual
  expected="$(awk '{print $1; exit}' "$checksum_path")"
  actual="$(sha256sum "$archive_path" | awk '{print $1}')"
  if [[ -z "$expected" || "$actual" != "$expected" ]]; then
    echo "checksum verification failed for $(basename "$archive_path")" >&2
    exit 1
  fi
}

verify_attestation() {
  if [[ "$skip_attestation" == "true" ]]; then
    if [[ "$archive_supplied" != "true" ]]; then
      echo "--skip-attestation requires --archive and is only for trusted local archives" >&2
      exit 2
    fi
    return
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "GitHub CLI is required for attestation verification; --skip-attestation is allowed only with a trusted local --archive" >&2
    exit 1
  fi

  gh attestation verify "$archive_path" --repo "$repo"
}

validate_archive_manifest() {
  local archive_root expected actual
  archive_root="pi-doctor-v${version}-${target}"
  expected="$(
    cat <<EOF
${archive_root}/
${archive_root}/LICENSE
${archive_root}/README.md
${archive_root}/completions/
${archive_root}/completions/_pi-doctor
${archive_root}/completions/pi-doctor.bash
${archive_root}/completions/pi-doctor.fish
${archive_root}/completions/pi-doctor.ps1
${archive_root}/pi-doctor
EOF
  )"
  actual="$(tar -tzf "$archive_path" | sort)"

  if tar -tzf "$archive_path" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
    echo "archive contains unsafe paths" >&2
    exit 1
  fi
  if [[ "$actual" != "$expected" ]]; then
    echo "archive contents did not match expected manifest" >&2
    echo "expected:" >&2
    echo "$expected" >&2
    echo "actual:" >&2
    echo "$actual" >&2
    exit 1
  fi
}

install_binary() {
  local archive_root extract_dir binary_path tmp_binary backup
  archive_root="pi-doctor-v${version}-${target}"
  tmp_dir="${tmp_dir:-$(mktemp -d)}"
  extract_dir="${tmp_dir}/extract"
  mkdir -p "$extract_dir" "$bin_dir"
  tar -xzf "$archive_path" -C "$extract_dir"

  binary_path="${extract_dir}/${archive_root}/pi-doctor"
  if [[ ! -f "$binary_path" ]]; then
    echo "archive did not contain expected pi-doctor binary path" >&2
    exit 1
  fi

  tmp_binary="$(mktemp "${bin_dir}/.pi-doctor.XXXXXX")"
  install -m 0755 "$binary_path" "$tmp_binary"
  backup="$(backup_path)"
  if [[ -f "${bin_dir}/pi-doctor" ]]; then
    install -m 0755 "${bin_dir}/pi-doctor" "$backup"
  fi
  mv -f "$tmp_binary" "${bin_dir}/pi-doctor"
  echo "installed pi-doctor to ${bin_dir}/pi-doctor"
}

if [[ "$rollback" == "true" && "$uninstall" == "true" ]]; then
  echo "--rollback and --uninstall cannot be used together" >&2
  exit 2
fi

if [[ "$rollback" == "true" ]]; then
  rollback_binary
  exit 0
fi

if [[ "$uninstall" == "true" ]]; then
  uninstall_binary
  exit 0
fi

if [[ "$skip_attestation" == "true" && "$archive_supplied" != "true" ]]; then
  echo "--skip-attestation requires --archive and is only for trusted local archives" >&2
  exit 2
fi

if [[ -z "$archive_path" ]]; then
  download_release_files
else
  infer_local_archive_metadata
fi

verify_checksum
verify_attestation
validate_archive_manifest
install_binary
