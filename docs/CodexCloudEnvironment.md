[<< Return to the README](../README.md)

# Codex Cloud environment

## Scope

This environment is for parser, renderer, fixture, configuration, coverage, and WebAssembly work in `Rokurolize/ftml`. FTML is a Rust library and does not need Wikijump's Node.js applications, Docker Compose stack, PostgreSQL, Redis, MinIO, `libmagic`, or `sqlx-cli`.

The scripts prepare the toolchains and dependency caches used by the repository's GitHub Actions workflows. Runtime integration, database state, browser parity capture, and the Wikijump FTML pin bump canary remain Wikijump responsibilities under [`WikijumpBoundary.md`](WikijumpBoundary.md).

## Codex environment settings

Use these settings for the FTML environment.

| Setting | Value |
|---|---|
| Container image | Universal |
| Repository | `Rokurolize/ftml` |
| Rust package version | 1.95.0 |
| Python package version | 3.13 |
| Setup script | Paste the complete contents of `scripts/codex-cloud-setup.sh` |
| Maintenance script | Paste the complete contents of `scripts/codex-cloud-maintenance.sh` |
| Agent internet access | Off |
| Secrets | None |

The scripts require the numeric Rust release in `rust-toolchain.toml` to equal both the manifest's `rust-version` and their reviewed environment pin. They reject a missing pin, a symbolic link, an extensionless `rust-toolchain`, or version drift.

Configure these ordinary environment variables so they remain available during setup, maintenance, and the agent phase.

```text
CARGO_HOME=/root/.cache/ftml-codex-cloud/cargo-home
CARGO_HTTP_TIMEOUT=120
CARGO_NET_OFFLINE=true
CARGO_NET_RETRY=5
RUSTFLAGS=-A unused -D warnings
```

`CARGO_NET_OFFLINE=true` makes missing cache entries fail immediately during agent work. The scripts override it only for fixed Cargo tool installation, dependency fetching, and the setup phase WebAssembly preload.

The scripts preserve the configured `CARGO_HOME` for Cargo caches but remove it from rustup's own process environment. Cargo receives the dedicated directory only after `rustup run` has selected Rust 1.95.0. This separation is required because rustup otherwise treats the dedicated Cargo cache as its proxy installation directory during self update checks.

Do not add production credentials or Wikijump service values. FTML tests are self contained, and Codex removes secrets before the agent phase in any case.

## Why the scripts are pasted

Codex creates a cache from the repository's default branch and runs the setup script there. On a cached resume, Codex checks out the task branch before running maintenance.

Paste the reviewed scripts into the environment settings instead of configuring maintenance as `bash scripts/codex-cloud-maintenance.sh`. Executing the repository copy after task checkout would let a pull request replace the maintenance program that runs while internet access remains available.

The repository copies are the reviewable source for the pasted settings. After changing either copy, run the validation commands below and paste the complete new content into Codex settings. Codex invalidates the cached container when an environment script changes.

Each run begins by printing its script revision. The setup and maintenance revisions in the Cloud log must match each other and the current repository copies; an older revision means the environment settings still contain stale content.

## Setup behavior

The setup script performs these operations on the trusted default branch.

1. It removes task checkout directories and relative entries from `PATH`, changes to `/`, and rejects Cargo configuration in `/` or the dedicated `CARGO_HOME`.
2. It installs the native compiler and Python prerequisites needed by FTML.
3. It requires Python 3.13 and uses isolated mode so the checkout cannot shadow standard library or pip modules.
4. It validates the FTML package name and the three Rust pins, then installs the pinned Rust toolchain without updating rustup itself, plus `rustfmt`, Clippy, Rust source, and the `wasm32-unknown-unknown` target.
5. It accepts only simple package specifiers in `scripts/check_conf-requirements.txt`, copies the reviewed file outside the checkout, and installs binary wheels from that trusted copy.
6. It installs and verifies the GitHub Actions versions of `cargo-nextest`, `cargo-tarpaulin`, `cargo-machete`, and `wasm-pack`.
7. It removes an untracked or ignored `Cargo.lock`, resolves the manifest from a trusted working directory, and fetches Cargo dependencies. A future tracked regular lockfile is retained and fetched with `--locked`.
8. It performs one development WebAssembly build without default features so `wasm-pack` can cache its managed `wasm-bindgen` tooling before agent internet access is disabled. The generated `pkg/` directory is removed afterward.

The setup does not run the Rust test suite, configuration checker, fixture updater, coverage job, benchmark, or Wikijump integration. Run the applicable checks during the agent phase without internet access.

## Maintenance behavior

The maintenance script runs after Codex checks out the task branch, but it immediately changes to `/` and refers to repository files by absolute path. Python uses isolated mode, rustup runs without the dedicated `CARGO_HOME`, every Cargo operation receives that directory only after `rustup run` selects the validated numeric release, and `cargo fetch` cannot discover task local `.cargo/config` files from the manifest path.

Maintenance ensures the reviewed Rust toolchain and four Cargo tool binaries are installed and report the expected versions. It compares the task branch's Python requirements with the trusted setup copy and stops if they differ; it never passes a task controlled requirements file to pip. It also rejects Cargo manifests that introduce dependency `git`, `path`, or alternate `registry` sources, workspace membership or inheritance, plus `[patch]` or `[replace]` entries, before any repository dependency fetch can run.

Maintenance does not run `cargo build`, `cargo test`, `wasm-pack build`, repository Python programs, fixture generation, or package build scripts. `cargo fetch` runs with Git CLI fetching disabled and without global Git configuration, from a working directory outside the checkout, after dependency sources are constrained to the default registry.

## Agent phase validation

The normal validation sequence is:

```bash
cargo fmt --all -- --check
cargo machete
cargo build --all-features
cargo build --no-default-features
cargo nextest run --all-features --profile ci
cargo test --doc --all-features -- --nocapture --test-threads 1
cargo clippy --tests --no-deps
python3 --version
python3 scripts/check_conf.py
python3 -m unittest discover -s scripts -p '*_test.py'
```

`python3 --version` must report Python 3.13.x. The environment level `RUSTFLAGS` matches the native CI jobs, so individual native commands do not need to repeat it.

Run the exhaustive ignored tests only when the touched behavior or final premerge verification needs them.

```bash
cargo test --all-features -- --nocapture --ignored
```

Run both WebAssembly feature configurations when changes touch `src/wasm`, feature gates, serialization, dependency selection, or the public WebAssembly surface.

```bash
RUSTFLAGS='-A unused -D warnings --cfg getrandom_backend="wasm_js"' wasm-pack build --dev
RUSTFLAGS='-A unused -D warnings --cfg getrandom_backend="wasm_js"' wasm-pack build --dev -- --no-default-features
```

Run coverage only for coverage sensitive work or before a merge that changes broad behavior because it is substantially slower than focused tests.

```bash
cargo tarpaulin --all-features --workspace --timeout 120 --fail-under 94 --out xml --output-dir target/coverage
```

## Environment acceptance test

The repository tests verify script structure, pins, and hostile configuration guards, but they cannot prove that a newly created Cloud container has every offline artifact. After creating or materially changing the environment, reset the cache, let setup and maintenance finish, confirm agent internet is Off, remove generated outputs, and run the full validation matrix once.

```bash
rm -rf target pkg
cargo fmt --all -- --check
cargo machete
cargo build --all-features
cargo build --no-default-features
cargo nextest run --all-features --profile ci
cargo test --doc --all-features -- --nocapture --test-threads 1
cargo clippy --tests --no-deps
python3 --version
python3 scripts/check_conf.py
python3 -m unittest discover -s scripts -p '*_test.py'
RUSTFLAGS='-A unused -D warnings --cfg getrandom_backend="wasm_js"' wasm-pack build --dev
RUSTFLAGS='-A unused -D warnings --cfg getrandom_backend="wasm_js"' wasm-pack build --dev -- --no-default-features
cargo tarpaulin --all-features --workspace --timeout 120 --fail-under 94 --out xml --output-dir target/coverage
```

Neither WebAssembly build may attempt a managed tool download. Any offline dependency error means the environment cache is incomplete and must be rebuilt before the scripts are treated as operational.

## Updating the environment

Update both scripts when `rust-toolchain.toml`, the `rust-version` field, the four pinned tool versions in `.github/workflows/build.yaml`, or `scripts/check_conf-requirements.txt` changes. A Python requirements change intentionally makes maintenance stop until the reviewed setup script is pasted again and the cache is reset.

Validate script changes locally before pasting them into the environment.

```bash
bash -n scripts/codex-cloud-setup.sh
bash -n scripts/codex-cloud-maintenance.sh
shellcheck scripts/codex-cloud-setup.sh scripts/codex-cloud-maintenance.sh
python3 -m unittest discover -s scripts -p '*_test.py'
```

Use Reset cache when maintenance reports missing trusted state, a system command is missing, Python requirements changed, a cached `wasm-bindgen` tool no longer matches a dependency change, or a native prerequisite changed. Do not enable unrestricted agent internet access to repair a stale cache.

## Troubleshooting

| Symptom | Action |
|---|---|
| The repository is not found at `/workspace/ftml` | Set `FTML_CODEX_REPO` to the actual absolute checkout path in ordinary environment variables |
| The log reports an older or mismatched script revision | Paste both current repository scripts into the environment settings, save the environment, and reset the cache |
| Python is not 3.13 | Select Python 3.13 in the environment package settings and reset the cache |
| rustup reports that it is not installed at the dedicated `CARGO_HOME` | Replace both pasted scripts with the current repository copies; they keep `CARGO_HOME` out of rustup and disable rustup self updates during toolchain installation |
| A Rust metadata file is missing, linked, or inconsistent | Fix the repository pins or update both reviewed environment scripts; do not choose one source silently |
| Python requirements changed | Review the change, update the pasted setup and maintenance scripts, and reset the cache |
| Cargo reports that a package is unavailable in offline mode | Rerun maintenance or reset the cache so trusted `cargo fetch` runs for the task branch |
| `wasm-pack` tries to download tooling during the agent phase | Reset the cache; if the branch intentionally changes `wasm-bindgen`, rebuild the environment before validating that branch |
| A Cargo tool record exists but its binary is missing or has the wrong version | Let maintenance reinstall the fixed version; reset the cache if verification still fails |
| Tarpaulin fails on a non Linux host | Use the Linux Universal environment or rely on the GitHub Actions coverage job |

## Security boundary

Keep agent internet access Off for ordinary FTML implementation and security work. Setup and maintenance already have internet access for dependency preparation, and the repository does not need live sites to run its unit, fixture, configuration, WebAssembly, or coverage checks.

The maintenance security boundary depends on five fail closed rules: it never executes the task copy of itself, never imports Python modules from the checkout, never installs task controlled Python requirements, never accepts rustup's extensionless toolchain override, and never lets Cargo discover task local configuration while fetching.

Wikidot compatibility claims still require provenance backed fixtures or read only evidence collected outside this environment. If a task requires fresh browser evidence, use a separate environment with only the required domains and `GET`, `HEAD`, and `OPTIONS`; do not broaden the default FTML environment.

OpenAI documents the setup and maintenance order, separate Bash sessions, environment variable lifetime, secret removal, and container caching in [Cloud environments](https://learn.chatgpt.com/docs/environments/cloud-environment). OpenAI documents the default off policy and least privilege domain and method controls in [Agent internet access](https://learn.chatgpt.com/docs/cloud/internet-access).
