#!/usr/bin/env bash
set -Eeuo pipefail

repo=${FTML_CODEX_REPO:-/workspace/ftml}
script_revision=2026-07-14.1
required_rust_version=1.95.0
state_dir=${FTML_CODEX_STATE_DIR:-$HOME/.cache/ftml-codex-cloud}
cargo_home=${CARGO_HOME:-$state_dir/cargo-home}
tool_root=$state_dir/cargo-tools
tool_bin_dir=${FTML_CODEX_BIN_DIR:-/usr/local/bin}
trusted_requirements=$state_dir/check_conf-requirements.txt
rust_channel=
python_executable=
cargo_command=()

printf 'FTML Codex Cloud setup revision %s\n' "$script_revision"

retry() {
  local attempt=1
  local max_attempts=5
  local delay=2
  local status

  until "$@"; do
    status=$?
    if (( attempt >= max_attempts )); then
      printf 'Command failed after %d attempts (exit %d):' "$attempt" "$status" >&2
      printf ' %q' "$@" >&2
      printf '\n' >&2
      return "$status"
    fi

    printf 'Command failed (attempt %d/%d); retrying in %ds:' "$attempt" "$max_attempts" "$delay" >&2
    printf ' %q' "$@" >&2
    printf '\n' >&2
    sleep "$delay"
    attempt=$((attempt + 1))
    delay=$((delay * 2))
  done
}

prepare_process_environment() {
  local config
  local entry
  local path
  local clean_path=
  local -a path_entries=()

  for path in "$HOME" "$repo" "$state_dir" "$cargo_home" "$tool_bin_dir"; do
    if [[ "$path" != /* ]]; then
      printf 'Codex Cloud paths must be absolute, got %s.\n' "$path" >&2
      return 1
    fi
  done

  case "$cargo_home/" in
    "$repo/"*)
      printf 'CARGO_HOME must be outside the task checkout.\n' >&2
      return 1
      ;;
  esac
  case "$tool_bin_dir/" in
    "$repo/"*)
      printf 'The Cargo tool binary directory must be outside the task checkout.\n' >&2
      return 1
      ;;
  esac

  IFS=: read -r -a path_entries <<<"${PATH-}"
  for entry in "${path_entries[@]}"; do
    [[ -n "$entry" && "$entry" == /* ]] || continue
    case "$entry/" in
      "$repo/"*) continue ;;
    esac
    case ":$clean_path:" in
      *":$entry:"*) ;;
      *) clean_path+="${clean_path:+:}$entry" ;;
    esac
  done

  export PATH="$tool_bin_dir:$HOME/.cargo/bin${clean_path:+:$clean_path}"
  # rustup locates its proxies through the platform Cargo home. Pass the isolated Cargo home only to Cargo after rustup selects the toolchain.
  unset CARGO_HOME RUSTUP_TOOLCHAIN

  if [[ -L "$state_dir" ]]; then
    printf 'FTML Codex state directory must not be a symlink: %s\n' "$state_dir" >&2
    return 1
  fi
  mkdir -p "$state_dir"
  chmod 0700 "$state_dir"
  for path in "$cargo_home" "$tool_root"; do
    if [[ -L "$path" ]]; then
      printf 'FTML Codex tool directories must not be symlinks: %s\n' "$path" >&2
      return 1
    fi
    mkdir -p "$path"
    if [[ ! -d "$path" || -L "$path" ]]; then
      printf 'Could not create a trusted FTML Codex tool directory: %s\n' "$path" >&2
      return 1
    fi
  done
  if [[ -L "$trusted_requirements" ]]; then
    printf 'Trusted Python requirements must not be a symlink: %s\n' "$trusted_requirements" >&2
    return 1
  fi

  for config in /.cargo/config /.cargo/config.toml "$cargo_home/config" "$cargo_home/config.toml"; do
    if [[ -e "$config" || -L "$config" ]]; then
      printf 'Unexpected Cargo configuration could execute helpers during setup: %s\n' "$config" >&2
      return 1
    fi
  done

  cd /
}

select_python() {
  local version

  python_executable=$(command -v python3 2>/dev/null || true)
  if [[ -z "$python_executable" || "$python_executable" != /* ]]; then
    printf 'A trusted absolute python3 executable is required.\n' >&2
    return 1
  fi
  case "$python_executable/" in
    "$repo/"*)
      printf 'python3 must not resolve inside the task checkout: %s\n' "$python_executable" >&2
      return 1
      ;;
  esac

  version=$(
    "$python_executable" -I -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")'
  )
  if [[ "$version" != 3.13 ]]; then
    printf 'FTML configuration CI uses Python 3.13, got %s at %s.\n' "$version" "$python_executable" >&2
    return 1
  fi
}

validate_repository() {
  local metadata_output
  local package_name
  local manifest_version
  local path
  local -a metadata=()

  if [[ ! -d "$repo/.git" && ! -f "$repo/.git" ]]; then
    printf 'FTML repository not found at %s. Set FTML_CODEX_REPO if the checkout path differs.\n' "$repo" >&2
    return 1
  fi
  if [[ -e "$repo/rust-toolchain" || -L "$repo/rust-toolchain" ]]; then
    printf 'The extensionless rust-toolchain file is not allowed; rustup gives it precedence over rust-toolchain.toml.\n' >&2
    return 1
  fi
  for path in "$repo/Cargo.toml" "$repo/rust-toolchain.toml" "$repo/scripts/check_conf-requirements.txt"; do
    if [[ ! -f "$path" || -L "$path" ]]; then
      printf 'Required repository metadata must be a regular file, not a symlink: %s\n' "$path" >&2
      return 1
    fi
  done

  metadata_output=$(
    "$python_executable" -I - "$repo/Cargo.toml" "$repo/rust-toolchain.toml" <<'PY'
import pathlib
import sys
import tomllib

with pathlib.Path(sys.argv[1]).open("rb") as file:
    package = tomllib.load(file)["package"]
with pathlib.Path(sys.argv[2]).open("rb") as file:
    channel = tomllib.load(file)["toolchain"]["channel"]

print(package["name"])
print(package["rust-version"])
print(channel)
PY
  )
  mapfile -t metadata <<<"$metadata_output"
  if (( ${#metadata[@]} != 3 )); then
    printf 'Could not read FTML package and toolchain metadata.\n' >&2
    return 1
  fi

  package_name=${metadata[0]}
  manifest_version=${metadata[1]}
  rust_channel=${metadata[2]}
  if [[ "$package_name" != ftml ]]; then
    printf '%s is not an FTML checkout; package name is %s.\n' "$repo" "$package_name" >&2
    return 1
  fi
  if [[ ! "$rust_channel" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'rust-toolchain.toml must pin a numeric Rust release, got %s.\n' "$rust_channel" >&2
    return 1
  fi
  if [[ "$manifest_version" != "$rust_channel" || "$rust_channel" != "$required_rust_version" ]]; then
    printf 'Rust pins must all match %s; Cargo.toml has %s and rust-toolchain.toml has %s.\n' "$required_rust_version" "$manifest_version" "$rust_channel" >&2
    return 1
  fi

  cargo_command=(rustup run "$rust_channel" env "CARGO_HOME=$cargo_home" cargo)
}

validate_requirements() {
  local invalid
  local status
  local safe_requirement='^[[:space:]]*(#.*)?$|^[[:space:]]*[[:alnum:]][[:alnum:]._-]*([<>=!~]=?[[:alnum:].*+_-]+(,[<>=!~]=?[[:alnum:].*+_-]+)*)?[[:space:]]*$'

  if invalid=$(grep -Env -- "$safe_requirement" "$repo/scripts/check_conf-requirements.txt"); then
    printf 'Unsafe pip requirement syntax is not allowed in Codex Cloud setup:\n%s\n' "$invalid" >&2
    return 1
  else
    status=$?
    if (( status != 1 )); then
      printf 'Could not validate Python requirements (grep exit %d).\n' "$status" >&2
      return "$status"
    fi
  fi
}

configure_rust() {
  retry env RUSTUP_MAX_RETRIES=5 rustup toolchain install "$rust_channel" --profile minimal --component clippy,rustfmt,rust-src --no-self-update
  retry env RUSTUP_MAX_RETRIES=5 rustup target add --toolchain "$rust_channel" wasm32-unknown-unknown
  rustup run "$rust_channel" rustc --version
  "${cargo_command[@]}" --version
}

cargo_tool_binary_matches() {
  local crate=$1
  local version=$2
  local binary=$3
  local output
  local version_pattern=${version//./\.}

  if [[ ! -x "$binary" || -L "$binary" ]]; then
    return 1
  fi
  output=$("$binary" --version 2>&1) || return 1
  [[ "$output" =~ (^|[^0-9])$version_pattern([^0-9]|$) ]]
}

cargo_tool_record_matches() {
  local crate=$1
  local version=$2
  local install_list

  install_list=$("${cargo_command[@]}" install --list --root "$tool_root") || return 1
  grep -Fx -- "$crate v$version:" <<<"$install_list" >/dev/null
}

cargo_tool_installed() {
  local crate=$1
  local version=$2

  cargo_tool_record_matches "$crate" "$version" || return 1
  cargo_tool_binary_matches "$crate" "$version" "$tool_root/bin/$crate" || return 1
  cargo_tool_binary_matches "$crate" "$version" "$tool_bin_dir/$crate"
}

install_cargo_tool() {
  local crate=$1
  local version=$2

  if cargo_tool_installed "$crate" "$version"; then
    printf '%s %s is already installed and verified.\n' "$crate" "$version"
    return
  fi

  if cargo_tool_record_matches "$crate" "$version" && cargo_tool_binary_matches "$crate" "$version" "$tool_root/bin/$crate"; then
    sudo install -m 0755 "$tool_root/bin/$crate" "$tool_bin_dir/$crate"
  else
    retry env CARGO_NET_OFFLINE=false CARGO_NET_GIT_FETCH_WITH_CLI=false GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null RUSTFLAGS= "${cargo_command[@]}" install "$crate" --version "$version" --locked --force --root "$tool_root"
    sudo install -m 0755 "$tool_root/bin/$crate" "$tool_bin_dir/$crate"
  fi
  if ! cargo_tool_installed "$crate" "$version"; then
    printf '%s %s did not install a verified executable.\n' "$crate" "$version" >&2
    return 1
  fi
}

fetch_repository_dependencies() {
  local lockfile=$repo/Cargo.lock
  local -a lock_arguments=()

  if git -C "$repo" ls-files --error-unmatch -- Cargo.lock >/dev/null 2>&1; then
    if [[ ! -f "$lockfile" || -L "$lockfile" ]]; then
      printf 'A tracked Cargo.lock must be a present regular file.\n' >&2
      return 1
    fi
    lock_arguments=(--locked)
  else
    if [[ -d "$lockfile" ]]; then
      printf 'Cargo.lock must not be a directory.\n' >&2
      return 1
    fi
    rm -f -- "$lockfile"
  fi

  retry env CARGO_NET_OFFLINE=false CARGO_NET_GIT_FETCH_WITH_CLI=false GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null "${cargo_command[@]}" fetch --manifest-path "$repo/Cargo.toml" "${lock_arguments[@]}"
}

if [[ ! -d "$repo/.git" && ! -f "$repo/.git" ]]; then
  printf 'FTML repository not found at %s. Set FTML_CODEX_REPO if the checkout path differs.\n' "$repo" >&2
  exit 1
fi

prepare_process_environment

retry sudo apt-get update
retry sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake curl git libssl-dev pkg-config python3 python3-pip shellcheck

select_python
validate_repository
validate_requirements
configure_rust

install -m 0644 "$repo/scripts/check_conf-requirements.txt" "$trusted_requirements"
retry env PIP_CONFIG_FILE=/dev/null PIP_DISABLE_PIP_VERSION_CHECK=1 "$python_executable" -I -m pip --isolated install --break-system-packages --only-binary=:all: --requirement "$trusted_requirements"

export CARGO_NET_RETRY=5
export CARGO_HTTP_TIMEOUT=120

install_cargo_tool cargo-nextest 0.9.100
install_cargo_tool cargo-tarpaulin 0.32.8
install_cargo_tool cargo-machete 0.8.0
install_cargo_tool wasm-pack 0.13.1

fetch_repository_dependencies

# Prime wasm-pack's managed wasm-bindgen tooling on the trusted default branch while setup has network access.
env CARGO_HOME="$cargo_home" CARGO_NET_OFFLINE=false CARGO_NET_GIT_FETCH_WITH_CLI=false GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null RUSTUP_TOOLCHAIN="$rust_channel" RUSTFLAGS='-A unused -D warnings --cfg getrandom_backend="wasm_js"' "$tool_bin_dir/wasm-pack" build "$repo" --dev -- --no-default-features
rm -rf -- "$repo/pkg"

printf 'FTML Codex Cloud setup completed.\n'
