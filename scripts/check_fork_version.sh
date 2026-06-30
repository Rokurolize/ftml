#!/usr/bin/env bash
set -euo pipefail

manifest="${1:-Cargo.toml}"

version="$(
  awk -F '"' '
    /^[[:space:]]*version[[:space:]]*=/ {
      print $2
      exit
    }
  ' "$manifest"
)"

pattern='^[0-9]+\.[0-9]+\.[0-9]+\+roku\.[0-9]{8}\.[0-9]+$'

if [[ ! "$version" =~ $pattern ]]; then
  cat >&2 <<EOF
Invalid ftml fork version: $version
Expected: <upstream-version>+roku.<yyyymmdd>.<n>
Example: 1.42.0+roku.20260630.1
EOF
  exit 1
fi

echo "ftml fork version OK: $version"
