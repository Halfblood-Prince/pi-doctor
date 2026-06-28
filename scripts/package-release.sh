#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh --target <triple> --binary <path> [--completions-binary <path>] [--output-dir <dir>] [--version <semver>]
EOF
}

target=""
binary=""
completions_binary=""
output_dir="dist"
version=""
source_date_epoch="${SOURCE_DATE_EPOCH:-}"

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
    --completions-binary)
      completions_binary="$2"
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

if [[ -z "$completions_binary" ]]; then
  completions_binary="$binary"
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

if [[ -z "$source_date_epoch" ]]; then
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    source_date_epoch="$(git log -1 --format=%ct)"
  else
    source_date_epoch="0"
  fi
fi

if ! [[ "$source_date_epoch" =~ ^[0-9]+$ ]]; then
  echo "SOURCE_DATE_EPOCH must be an integer Unix timestamp" >&2
  exit 2
fi

export TZ=UTC

archive_root="pi-doctor-v${version}-${target}"
stage_dir="${output_dir}/${archive_root}"
archive_path="${output_dir}/${archive_root}.tar.gz"
checksum_path="${archive_path}.sha256"

rm -rf "$stage_dir"
mkdir -p "$stage_dir/completions" "$output_dir"

cp "$binary" "$stage_dir/pi-doctor"
cp README.md "$stage_dir/README.md"
cp LICENSE "$stage_dir/LICENSE"

"$completions_binary" completions bash > "$stage_dir/completions/pi-doctor.bash"
"$completions_binary" completions zsh > "$stage_dir/completions/_pi-doctor"
"$completions_binary" completions fish > "$stage_dir/completions/pi-doctor.fish"
"$completions_binary" completions powershell > "$stage_dir/completions/pi-doctor.ps1"

find "$stage_dir" -type d -exec chmod 0755 {} +
find "$stage_dir" -type f -exec chmod 0644 {} +
chmod 0755 "$stage_dir/pi-doctor"
find "$stage_dir" -exec touch -h -d "@${source_date_epoch}" {} +

rm -f "$archive_path" "$checksum_path"
tar \
  --sort=name \
  --mtime="@${source_date_epoch}" \
  --owner=0 \
  --group=0 \
  --numeric-owner \
  -C "$output_dir" \
  -cf - "$archive_root" | gzip -n > "$archive_path"
(
  cd "$output_dir"
  sha256sum "$(basename "$archive_path")" > "$(basename "$checksum_path")"
)
