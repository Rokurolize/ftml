# FTML parity campaign deadline-closure plan — 2026-08-19

## Purpose and hard boundary

This plan closes every task-owned remainder from the FTML Wikidot-parity campaign before **2026-08-19 15:31:14 JST**. The enforceable meaning of “no remaining work” is: no known task-owned defect, dirty checkout, uncommitted change, unpushed commit, open task PR, open task issue, failing required gate, unreconciled task worktree, or orphaned task artifact remains at the final audit boundary. A finite campaign cannot prove that no unknown bug exists, so this plan deliberately uses a fixed, reviewable acceptance surface instead of making an untestable claim.

The scope is the FTML campaign and the Wikijump caller integration required by its typed runtime contracts. It does **not** absorb the unrelated Wikijump compatibility backlog, PR #1353, its standing candidate/runtime, or unrelated Wikijump branches and worktrees.

## Feasibility verdict

At the 11:37:55 JST inventory point, **3 hours 53 minutes 19 seconds** remained. The plan is feasible because the FTML side is already merged and green, the Wikijump Social integration is substantially implemented in a current-develop worktree, and the expensive checks can run in parallel. It is not feasible to add open-ended parity exploration, complete the entire Wikijump backlog, or take over and redeploy PR #1353’s standing runtime in the same window. The optional worker diagnostics are the first cut; the Social caller contract, committed FTML pin, remote workflow exercise, and cleanup are not cut.

## Verified baseline at 11:37–11:41 JST

- FTML is clean on `main` at `3f57eb59a172a76d4d26f9467bb0eed8c77b0aaf`, exactly equal to `origin/main`, with one local branch, one origin branch, one worktree, no open FTML PRs, and no open FTML issues.
- PR #699, “Harden standing Wikidot parity discovery,” merged at 11:00 JST and reports 953 unit tests with 6 ignored, 489 integration tests, 1 doctest, Python gates 39/39, Chrome normalization 8/8, full-page checks 12/12, candidate Wikijump checks 8/8, live rotation 8/8, targeted mutations 49/49 caught, and a 2,000-run fuzz smoke with no crash or timeout.
- The three new standing workflows exist but have no completed GitHub Actions run yet: `wikidot-robustness.yml`, `wikidot-live-rotation.yml`, and `wikidot-mutation.yml`.
- Wikijump `origin/develop` is `aa34902f42eca59cd7edf0d266dc847fe6b332d4`, but its committed FTML pin is still `62ebba4efda1f10e82363c23c925061fbe939e49`; therefore the repository does not yet consume the merged FTML campaign.
- The task-owned Wikijump worktree `/home/roku/.devspace/worktrees/wikijump-2ed2f0e0` contains an uncommitted Social caller-runtime implementation and already pins FTML to `3f57eb59a172a76d4d26f9467bb0eed8c77b0aaf`. It has no PR yet.
- The task-owned Wikijump worktree `/home/roku/.devspace/worktrees/wikijump-0a1b1181` contains uncommitted FTML worker-failure diagnostics plus a full-page SCP-9506 test now duplicated by merged PR #1513. It is based on an old develop revision and has no PR.
- FTML task artifacts still present locally include `mutants.out`, `/tmp/ftml-robustness-coverage.json`, a small fuzz artifact directory, and dated Social discovery files under `/tmp`.

## Priority order

1. **P0 — Make the caller contract real:** finish and merge the Wikijump Social resolver and update the committed FTML pin to the exact FTML merge head. Without this, FTML’s `social-runtime` contract and robustness workflow are structurally broken against Wikijump `develop`.
2. **P1 — Validate the merged repository pair:** run the exact FTML and Wikijump contract gates against clean merged heads, then manually exercise all three new standing workflows.
3. **P2 — Preserve useful runtime diagnostics:** carry the FTML worker failure stage/source/generator logging into the Wikijump integration only if it does not threaten P0/P1 completion.
4. **P3 — Remove every task-owned remainder:** delete the two task worktrees and their local/remote branches after merge, remove temporary build/evidence outputs that are no longer authoritative, and prove final repository state.

## Execution schedule

### 11:45–12:20 — Reconcile the two Wikijump task worktrees into one mergeable branch

- Use the Social worktree as the integration branch because it is based on current `origin/develop` and already contains the correct FTML pin.
- Review and preserve the Social runtime changes in `deepwell/src/services/render/compat/wikidot_social.rs`, the render-service requirement resolution, ListPages selected-content reuse, FTML boundary note, and implementation-ledger entry.
- Copy only the reusable FTML worker-failure context/logging change and its focused test from the older canary worktree; do not copy its duplicate SCP-9506 test because PR #1513 already owns that regression.
- Reconcile the Social ledger wording against the finite caller contract. Broader custom-domain and browser-navigation research remains outside this deadline and must not be represented as completed by this PR.
- Run `cargo fmt --manifest-path deepwell/Cargo.toml --check` and the focused Social/worker-context tests immediately after reconciliation.

### 12:20–13:05 — Validate Wikijump against the exact merged FTML head

- Run all Social resolver tests, the SCP-9506 full-page test, and the seven caller-runtime tests named by FTML’s `caller-runtime-contracts.json`.
- Run FTML’s `scripts/check_wikijump_candidate.py --run-tests` against the clean/current Wikijump integration worktree so the dependency is temporarily rebound and independently verified.
- Run the two required compatibility checkers for changed rendering/classification code: `corpus-pinned-literals` and `wikijump-identifier-leaks` with their documented arguments.
- Run `RUSTFLAGS='-D warnings' cargo clippy --manifest-path deepwell/Cargo.toml --tests --no-deps` once for the final batch if the cached build budget permits; otherwise run clippy on the changed Deepwell package surface and record the exact limitation rather than silently omitting it.
- Inspect the Cargo lock delta and require the FTML source to resolve exactly to `3f57eb59a172a76d4d26f9467bb0eed8c77b0aaf`; reject unrelated dependency movement.

### 13:05–13:20 — Commit, push, open, and merge the Wikijump PR

- Commit the integration as one reviewable branch with separate commits for the FTML pin/Social runtime and optional worker diagnostics when both are included.
- Push immediately, open a PR against `develop`, include the exact FTML PR/merge SHA and local validation results, and reference the FTML Social caller-runtime contract.
- Merge normally only after local acceptance is green. Do not admin-merge, force-push protected branches, or touch PR #1353.
- Fetch `origin/develop` and verify the merge contains the exact FTML pin, Social test, runtime resolver, and optional diagnostics.

### 13:20–14:20 — Start remote workflows in parallel and run post-merge local acceptance

- Dispatch `Wikidot robustness`, `Wikidot live parity rotation`, and `Wikidot mutation ratchet` against FTML `main` only after the Wikijump merge, because the robustness workflow checks out Wikijump `develop` and requires the Social contract test to exist there.
- While those workflows run, execute on clean FTML `main`: Python gate tests, `parity_index`, Chrome normalization 8/8, full-page 12/12, candidate Wikijump 8/8, `cargo fmt --all -- --check`, full `cargo test`, and warnings-as-errors clippy.
- Run one bounded local fuzz smoke on the final merged pair. The 900-second scheduled workflow remains the long-running authority; the local smoke is for immediate regression detection.
- Treat a live-rotation mismatch as a real defect only after reproducing it against the captured artifact and the exact source hash. Do not edit or normalize evidence to force a pass.

### 14:20 hard decision gate

- If P0/P1 is green, stop all new discovery and proceed only with merge verification, workflow completion, and cleanup.
- If the optional worker diagnostics are the only blocker, drop them, delete the old canary worktree, and complete the Social/pin integration. This is the first planned cut.
- If a repository defect is localized and fixable within 20 minutes, fix it in a focused PR and rerun only the affected gate plus the final aggregate gate.
- If GitHub Actions fails solely from a transient external service or package-install failure, rerun once while preserving the failed artifact; local acceptance remains the code authority, but the final report must distinguish infrastructure failure from a passing workflow.
- Do not begin a new feature family, full-corpus live campaign, global mutation run, full Wikijump workspace test, or standing-runtime promotion after this gate.

### 14:20–15:00 — Post-merge and workflow closure

- Require all task PRs merged and all three manually dispatched workflows terminal. Record run IDs, head SHA, conclusions, and artifact names.
- Re-read FTML open PRs/issues and task-relevant Wikijump PR state after the merges; do not infer closure from the pre-merge inventory.
- Confirm FTML bindings have no unclassified mismatch and that every caller-runtime binding maps to a concrete test now present on Wikijump `develop`.
- Confirm Wikijump `develop` resolves the committed FTML revision and the full SCP-9506 canary passes with that committed dependency rather than a temporary local path.

### 15:00–15:20 — Cleanup with evidence preservation

- Remove the two task-owned Wikijump worktrees only after their useful changes are merged or deliberately cut, then delete their task branches locally and remotely.
- Remove FTML task artifacts that are reproducible and no longer authoritative: local `mutants.out`, transient coverage JSON, fuzz artifacts without an active failure, and `/tmp/ftml-social-*` discovery directories after confirming all required evidence was committed or uploaded.
- Preserve GitHub workflow artifacts and committed raw Wikidot references; they are acceptance evidence, not disposable build output.
- Do not delete or modify unrelated Wikijump worktrees, PR #1353 state, protected runtime volumes, or the standing runtime currently owned by the other compatibility session.

### 15:20–15:31:14 — Final zero-residual audit and report

- FTML: `main == origin/main`, clean worktree, one local branch, one origin branch, one worktree, open PR count 0, open issue count 0.
- Wikijump task scope: the integration PR merged, no task branch or task worktree remains, and the committed `develop` FTML pin equals the reported FTML merge SHA. Unrelated Wikijump branches/worktrees remain untouched and are not claimed as this task’s residue.
- Gates: record exact counts for Rust, Python, parity, browser, full-page, candidate, live rotation, mutation, and fuzz; report any external-service retry candidly.
- Delivery: report FTML and Wikijump merge SHAs, PR numbers, workflow run IDs, cleanup counts, and the exact standing-runtime exclusion caused by the active PR #1353 owner. Do not claim deployment or browser-serving identity was changed.

## Explicit cuts required to meet the deadline

- No open-ended search for additional Wikidot mismatches after the fixed 8-case live rotation.
- No global mutation testing; retain the targeted 49-mutant ratchet only.
- No full Wikijump workspace/database/browser suite; run the exact caller-runtime, full-page, Social, checker, format, and clippy surfaces required by this change.
- No new custom-domain, provider-navigation, or unrelated browser-lifecycle research for Social.
- No takeover, merge, cleanup, or deployment of PR #1353 or its standing runtime.
- Worker-failure diagnostics are optional and are cut before any P0/P1 acceptance gate.

## Definition of completion

The deadline task is complete only when the FTML repository is clean and fully synchronized, the required Wikijump caller integration is merged with the exact FTML pin, all fixed local gates pass, all three new standing workflows have terminal recorded outcomes, all task-owned worktrees/branches/artifacts are reconciled or removed, and the final report names every deliberate scope cut without presenting it as completed work.
