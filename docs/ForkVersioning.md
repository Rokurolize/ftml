# Fork Versioning

This repository is Rokurolize's operational fork of `scpwiki/ftml`.
The fork tracks upstream changes, but it is not an upstream contribution channel.

## Version Format

Keep the package name as `ftml`.
Set the Cargo package version to the upstream-compatible version with Rokurolize build metadata:

```toml
version = "<upstream-version>+roku.<yyyymmdd>.<n>"
```

For example:

```toml
version = "1.42.0+roku.20260630.1"
```

The upstream version prefix says which upstream crate version the fork is based on.
The `+roku.<yyyymmdd>.<n>` suffix makes the fork snapshot visible in downstream lockfiles without renaming the crate.

## Tags

Tag merged fork snapshots with the full Cargo package version:

```text
roku-v<full-cargo-version>
```

For example:

```text
roku-v1.42.0+roku.20260630.1
```

Use annotated tags.
Do not use upstream-looking tags such as `v1.42.0-alpha.1` for fork snapshots.

## Sync Runbook

1. Fetch `upstream/main` and `origin/main`.
2. Create a branch from `origin/main`.
3. Merge `upstream/main` into that branch when upstream has moved.
4. Resolve conflicts while preserving fork-local policy and CI files.
5. Set `Cargo.toml` `version` to `<upstream-version>+roku.<yyyymmdd>.<n>`.
6. Run `scripts/check_fork_version.sh`.
7. Run the normal local verification for the change.
8. Open a PR against `Rokurolize/ftml` `main`.
9. Merge after CI and review gates pass.
10. Create an annotated `roku-v<full-cargo-version>` tag on the merged commit.
11. In `Rokurolize/wikijump/deepwell`, run `cargo update -p ftml`.
12. Open and merge the resulting `wikijump` lockfile update after Deepwell CI passes.

Keep `upstream` fetch-only.
Do not push to `scpwiki/ftml`.
