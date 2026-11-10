#!/usr/bin/env bash
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
HEAVY="${FTML_HEAVY_BUILD:-$HOME/.local/state/wikijump/muse-bhl/tools/heavy-build.sh}"

FILTER=(
  mathml_mismatched_close_stays_escaped
  mathml_balanced_input_ends_balanced
  mathml_property_emits_only_allowlisted_tags
  highlight_tokens_cover_char_boundaries_on_fixed_inputs
  highlight_tokens_cover_generated_inputs
)

cargo_emit() {
  if [[ -f "$HEAVY" ]]; then
    bash "$HEAVY" cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "${FILTER[@]}"
  else
    cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "${FILTER[@]}"
  fi
}

restore() {
  local file="$1" bak="$2"
  if [[ -f "$bak" ]]; then
    cp "$bak" "$file"
    rm -f "$bak"
  fi
}

scratch() {
  local name="$1" file="$2" old="$3" new="$4"
  local bak
  bak="$(mktemp)"
  cp "$file" "$bak"
  if ! python3 - "$file" "$old" "$new" <<'PY'
import pathlib, sys
path, old, new = sys.argv[1:]
file = pathlib.Path(path)
text = file.read_text(encoding="utf-8")
count = text.count(old)
if count != 1:
    raise SystemExit(f"mutation anchor count {count} in {path}")
file.write_text(text.replace(old, new, 1), encoding="utf-8")
PY
  then
    restore "$file" "$bak"
    echo "SCRATCH ANCHOR FAILED: $name" >&2
    return 2
  fi
  local rc=0
  if cargo_emit; then
    rc=0
  else
    rc=$?
  fi
  restore "$file" "$bak"
  if [[ $rc -eq 0 ]]; then
    echo "MUTANT SURVIVED: $name" >&2
    return 1
  fi
  echo "caught: $name"
  return 0
}

if [[ "${1:-}" == "mutations" ]]; then
  status=0
  step=0
  scratch "mathml mismatched close is emitted" \
    "$REPO/src/render/html/element/math.rs" \
    'Some(MathmlTag::Close { name }) if stack.last().copied() == Some(name)' \
    'Some(MathmlTag::Close { name }) if true' \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch "highlight tokens drop the tail" \
    "$REPO/src/render/html/element/highlight.rs" \
    'pos = endpos + endlen;
        } else {
            emit(last_inner, pos, len);' \
    'pos = endpos + endlen;
        } else {
            emit(last_inner, pos, pos);' \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  exit $status
fi

if cargo_emit; then
  echo "PASS  emission"
  exit 0
fi
echo "FAIL  emission" >&2
exit 1
