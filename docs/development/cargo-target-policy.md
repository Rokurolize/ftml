# Cargo target policy

FTML is one Cargo package with one lockfile. The normal local development
cache is the repository-level `target/` directory, made explicit in
`.cargo/config.toml`. Do not create another persistent target for the same
checkout.

The default `dev` and `test` profiles use line-table debug information for the
FTML crate and no debug information for dependencies. Use the explicit
`debugging` profile when a debugger needs full symbols:

```sh
cargo test --profile debugging --all-features
```

`release` remains the production profile and keeps link-time optimisation.
Normal local development retains Cargo's incremental compilation for fast
rebuilds.

## Host, WebAssembly, and candidate builds

FTML has separate host and `wasm32-unknown-unknown` build identities, and its
feature matrix includes default and `--no-default-features` builds. A
candidate or one-shot verification build must use a revision- and identity-
specific target outside the checkout:

```sh
CARGO_TARGET_DIR=/path/to/ftml-candidate-<revision>-<role> \
CARGO_INCREMENTAL=0 \
cargo test --all-features
```

Record the revision, target triple, feature set, `RUSTFLAGS`, Cargo profile,
owner, creation time, expiry, and evidence receipt. Keep only the active
candidate and the immediately preceding known-good rollback candidate. Once a
receipt is terminal and no process or lease references the target, remove it
and retain the receipt rather than the cache.

CI target caches are disposable performance caches, not provenance. The
library, WebAssembly, and lint jobs use separate cache keys and the cache
generation is bumped when profile or target policy changes. CI sets
`CARGO_INCREMENTAL=0` because those builds are one-shot; local development
does not.

Runtime data, browser evidence, generated `pkg/` releases, and published
artifacts are not Cargo targets and must not be removed by target cleanup.

## Safe cleanup

Before cleaning, verify that no Cargo/rustc/wasm-pack process or resource lease
uses the target. Inspect the effect first:

```sh
cargo clean --dry-run --profile dev
cargo clean --dry-run --release
```

Apply cleanup only to the canonical target or an explicitly identified,
expired candidate target. Do not use broad filesystem deletion.

This policy follows Cargo's documented [build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html), [profiles](https://doc.rust-lang.org/cargo/reference/profiles.html), [configuration](https://doc.rust-lang.org/cargo/reference/config.html), [build performance](https://doc.rust-lang.org/cargo/guide/build-performance.html), and [cargo clean](https://doc.rust-lang.org/cargo/commands/cargo-clean.html) guidance.
