# Herdr build approval

FTML agents may inspect code and edit their assigned worktrees in parallel. Every non-controller agent and background process must request approval before a memory-intensive command. The controller serializes those commands through one build slot.

## Commands that require approval

Request approval before starting `cargo build`, `cargo test`, `cargo check`, `cargo clippy`, `cargo bench`, `cargo doc`, direct `rustc`, `wasm-pack`, or another command expected to compile Rust or run a full suite. `cargo fmt`, `rg`, Git inspection, fixture inspection, and focused scripts that reuse an existing binary do not require approval.

## Request and approval

Send the controller this exact information and wait:

```text
BUILD REQUEST
Agent: <Herdr agent name>
Worktree: <absolute path>
Command: <exact command or serial command batch>
Purpose: <observable result being checked>
Scope: <focused or full>
Memory: <low, medium, high, or unknown>
```

Approval is explicit and single-use. It applies only to the named agent, worktree, and command or serial command batch. A changed command needs a new request. The approval is consumed when the command finishes or fails.

The controller grants only one build slot. A running memory-intensive command blocks approval of the next one. While waiting, agents continue read-only analysis, small edits, fixture checks, and review work.

After execution, report the exit result and release the build slot. Do not silently retry a failed memory-intensive command; request approval again with the failure reason.

## Herdr layout

Use one clearly labeled workspace per implementation issue. Keep controller and read-only review panes in a review workspace, and keep documentation implementation and documentation review in separately labeled workspaces. Pane and agent names must include the issue number or review role.
