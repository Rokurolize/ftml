#!/usr/bin/env bash
set -uo pipefail

TLA_VERSION=1.7.4
TLA_SHA256=936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
JAR="${TLA2TOOLS_JAR:-$HOME/.cache/tlaplus/$TLA_VERSION/tla2tools.jar}"
HEAVY="${FTML_HEAVY_BUILD:-$HOME/.local/state/wikijump/muse-bhl/tools/heavy-build.sh}"

command -v java >/dev/null || { echo "java not found (TLC needs Java 11+)" >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 not found" >&2; exit 2; }
if [[ ! -f "$JAR" ]]; then
  mkdir -p "$(dirname "$JAR")"
  curl -fsSL -o "$JAR" "https://github.com/tlaplus/tlaplus/releases/download/v$TLA_VERSION/tla2tools.jar" || exit 2
fi
echo "$TLA_SHA256  $JAR" | sha256sum -c --quiet - || { echo "tla2tools.jar checksum mismatch" >&2; exit 2; }

WORK="${FTML_RENDER_WALK_WORK:-$(mktemp -d)}"
mkdir -p "$WORK"
[[ -n "${FTML_RENDER_WALK_WORK:-}" ]] || trap 'rm -rf "$WORK"' EXIT
cp "$HERE/RefStacks.tla" "$HERE/EquationWalk.tla" "$HERE/TabNav.tla" "$HERE/TabLift.tla" "$WORK/"
python3 -I "$HERE/extract.py" "$WORK" || exit 2

tlc_run() {
  local name="$1" cfg="$2" module="$3"
  (cd "$WORK" && java -XX:+UseParallelGC -cp "$JAR" tlc2.TLC -deadlock -workers 1 \
      -config "$cfg" -metadir "$WORK/states-$name" "$module.tla") > "$WORK/$name.log" 2>&1
  return $?
}

expect_pass() {
  local name="$1" cfg="$2" module="$3"
  local start=$SECONDS
  tlc_run "$name" "$cfg" "$module"
  local rc=$?
  if [[ $rc -eq 0 ]]; then
    echo "PASS  $name ($(grep -oE '[0-9]+ distinct states found' "$WORK/$name.log" | head -1), $((SECONDS - start))s)"
    return 0
  fi
  echo "FAIL  $name" >&2
  if grep -q 'is violated' "$WORK/$name.log"; then
    sed -n '/is violated/,/^[0-9]* states generated/p' "$WORK/$name.log" >&2
  else
    echo "ERROR $name: TLC exited $rc without a counterexample" >&2
    cat "$WORK/$name.log" >&2
    return 2
  fi
  return 1
}

expect_fail() {
  local name="$1" cfg="$2" module="$3"
  local start=$SECONDS
  tlc_run "$name" "$cfg" "$module"
  local rc=$?
  if [[ $rc -ne 0 ]] && grep -q 'is violated' "$WORK/$name.log"; then
    echo "EXPECTED-FAIL $name ($(grep -oE '[0-9]+ distinct states found' "$WORK/$name.log" | head -1), $((SECONDS - start))s)"
    grep -m1 'is violated' "$WORK/$name.log"
    return 0
  fi
  echo "UNEXPECTED $name: expected a TLC violation, got exit $rc" >&2
  cat "$WORK/$name.log" >&2
  return 1
}

cfg_with() {
  local base="$1" dest="$2"
  shift 2
  cat "$base" > "$dest"
  echo "SPECIFICATION Spec" >> "$dest"
  echo "INVARIANT TypeOK" >> "$dest"
  for invariant in "$@"; do
    echo "INVARIANT $invariant" >> "$dest"
  done
}

override_cfg() {
  local base="$1" dest="$2" key="$3" value="$4"
  shift 4
  sed -E "s/^(  ${key} = ).*/\\1${value}/" "$base" > "$dest"
  echo "SPECIFICATION Spec" >> "$dest"
  echo "INVARIANT TypeOK" >> "$dest"
  for invariant in "$@"; do
    echo "INVARIANT $invariant" >> "$dest"
  done
}

note() {
  local rc=$1
  if [[ $rc -eq 0 ]]; then
    return
  fi
  if [[ $status -eq 0 || $rc -eq 2 ]]; then
    status=$rc
  fi
}

cargo_walk() {
  local filter=("$@")
  if [[ -f "$HEAVY" ]]; then
    bash "$HEAVY" cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "${filter[@]}"
  else
    cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "${filter[@]}"
  fi
}

if [[ "${1:-}" == "mutations" ]]; then
  declare -a rows=(
    "m-fn ref.cfg RefStacks FnContains FALSE NoDupFn"
    "m-exit ref.cfg RefStacks FnExitOnlyElements FALSE NoUnder"
    "m-bib ref.cfg RefStacks BibLegacyReturns FALSE NoUnder"
    "m-eq eq.cfg EquationWalk WalkFootnoteBodies FALSE NumbersAgree"
    "m-nav nav.cfg TabNav AbortOnDuplicate FALSE DupRejected"
    "m-flush lift.cfg TabLift FlushOuter FALSE LiftsConsumed"
    "m-depth lift.cfg TabLift RestoreDepth FALSE DepthRestored"
  )
  status=0
  for row in "${rows[@]}"; do
    read -r name base module key value invariant <<<"$row"
    override_cfg "$WORK/$base" "$WORK/$name.cfg" "$key" "$value" "$invariant"
    expect_fail "$name" "$name.cfg" "$module" || status=1
  done
  exit $status
fi

scratch_tree() {
  local name="$1" file="$2" old="$3" new="$4" filter="$5"
  local bak rc
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
    cp "$bak" "$file"
    rm -f "$bak"
    echo "SCRATCH ANCHOR FAILED: $name" >&2
    return 2
  fi
  local -a cmd
  if [[ -f "$HEAVY" ]]; then
    cmd=(bash "$HEAVY" cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "$filter")
  else
    cmd=(cargo test --manifest-path "$REPO/Cargo.toml" --lib -- --test-threads=8 "$filter")
  fi
  if timeout --signal=KILL 120 "${cmd[@]}"; then
    rc=0
  else
    rc=$?
  fi
  cp "$bak" "$file"
  rm -f "$bak"
  if [[ $rc -eq 0 ]]; then
    echo "MUTANT SURVIVED: $name" >&2
    return 1
  fi
  echo "caught: $name"
  return 0
}

if [[ "${1:-}" == "scratches" ]]; then
  status=0
  step=0
  scratch_tree "footnote self-cite guard removed" \
    "$REPO/src/render/html/context.rs" \
    'if self.footnote_render_stack.contains(&index)' \
    'if false' \
    footnote_self_cite_stays_bounded_and_releases_the_guard \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch_tree "footnote exit removed" \
    "$REPO/src/render/html/element/footnotes.rs" \
    'if tooltip_text.is_none() {
        ctx.exit_footnote_ref(index.get());
    }' \
    'if tooltip_text.is_none() {
        let _ = index.get();
    }' \
    footnote_self_cite_stays_bounded_and_releases_the_guard \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch_tree "bibliography reentry guard removed" \
    "$REPO/src/render/html/element/bibliography.rs" \
    'if !ctx.enter_bibliography_ref(label)' \
    'if false' \
    bibliography_self_cite_stays_bounded_and_releases_the_guard \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch_tree "equation footnote walk removed" \
    "$REPO/src/render/html/element/math.rs" \
    'stack.push((ctx.footnotes()[number - 1].as_slice(), 0));' \
    'let _ = ctx.footnotes()[number - 1].as_slice();' \
    legacy_equation_numbers_follow_footnote_references \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch_tree "duplicate tabview assignment accepted" \
    "$REPO/src/render/html/element/tabs.rs" \
    'if assignments[view_index].is_some() {
                return None;
            }' \
    'if assignments[view_index].is_some() {
                let _ = view_index;
            }' \
    tab_nav_duplicate_view_assignment_is_rejected \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  step=0
  scratch_tree "tabview lift flush dropped" \
    "$REPO/src/render/html/element/tabs.rs" \
    'let mut lifts = ctx.take_wikidot_tabview_lifts();' \
    'let mut lifts = 0;' \
    each_outer_tabview_consumes_its_own_lift \
    || step=$?
  if [[ $step -gt $status ]]; then status=$step; fi
  exit $status
fi

cfg_with "$WORK/ref.cfg" "$WORK/ref-pass.cfg" Balanced NoDupFn NoDupBib NoUnder
cfg_with "$WORK/ref.cfg" "$WORK/ref-reenter.cfg" NeverReentered
cfg_with "$WORK/eq.cfg" "$WORK/eq-pass.cfg" NumbersAgree
cfg_with "$WORK/eq.cfg" "$WORK/eq-number.cfg" NeverNumbered
cfg_with "$WORK/nav.cfg" "$WORK/nav-pass.cfg" KnownPinned DupRejected CoversOrNone
cfg_with "$WORK/lift.cfg" "$WORK/lift-pass.cfg" LiftsConsumed DepthRestored
cfg_with "$WORK/lift.cfg" "$WORK/lift-emit.cfg" NeverEmitted

status=0
expect_pass ref "$WORK/ref-pass.cfg" RefStacks || note $?
expect_fail ref-reenter "$WORK/ref-reenter.cfg" RefStacks || note $?
expect_pass eq "$WORK/eq-pass.cfg" EquationWalk || note $?
expect_fail eq-number "$WORK/eq-number.cfg" EquationWalk || note $?
expect_pass nav "$WORK/nav-pass.cfg" TabNav || note $?
expect_pass lift "$WORK/lift-pass.cfg" TabLift || note $?
expect_fail lift-emit "$WORK/lift-emit.cfg" TabLift || note $?
if [[ $status -ne 0 ]]; then
  exit $status
fi

echo "trees"
if cargo_walk verify_walks:: tab_nav_; then
  echo "PASS  trees"
else
  echo "FAIL  trees" >&2
  exit 1
fi
exit 0
