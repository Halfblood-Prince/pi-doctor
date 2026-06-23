#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh --target <triple> --binary <path> [--output-dir <dir>] [--version <semver>]
EOF
}

target=""
binary=""
output_dir="dist"
version=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      target="$2"
      shift 2
      ;;
    --binary)
      binary="$2"
      shift 2
      ;;
    --output-dir)
      output_dir="$2"
      shift 2
      ;;
    --version)
      version="$2"
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

if [[ -z "$target" || -z "$binary" ]]; then
  usage >&2
  exit 2
fi

if [[ -z "$version" ]]; then
  version="$(
    awk '
      /^\[workspace\.package\]/ { in_section=1; next }
      /^\[/ { in_section=0 }
      in_section && /^version = / {
        gsub(/"/, "", $3)
        print $3
        exit
      }
    ' Cargo.toml
  )"
fi

archive_root="pi-doctor-v${version}-${target}"
stage_dir="${output_dir}/${archive_root}"
archive_path="${output_dir}/${archive_root}.tar.gz"
checksum_path="${archive_path}.sha256"

rm -rf "$stage_dir"
mkdir -p "$stage_dir/completions" "$output_dir"

cp "$binary" "$stage_dir/pi-doctor"
cp README.md "$stage_dir/README.md"
cp LICENSE "$stage_dir/LICENSE"

"$binary" completions bash > "$stage_dir/completions/pi-doctor.bash"
"$binary" completions zsh > "$stage_dir/completions/_pi-doctor"
"$binary" completions fish > "$stage_dir/completions/pi-doctor.fish"
"$binary" completions powershell > "$stage_dir/completions/pi-doctor.ps1"

tar -C "$output_dir" -czf "$archive_path" "$archive_root"
(
  cd "$output_dir"
  sha256sum "$(basename "$archive_path")" > "$(basename "$checksum_path")"
)
