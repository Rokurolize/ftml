# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring

- Read `CONTEXT.md` at the repo root, or `CONTEXT-MAP.md`, when either file exists.
- Read ADRs under `docs/adr/` when that directory exists and an ADR touches the area being changed.

If these paths do not exist, proceed silently. The domain-modeling skill creates them when needed.

## File structure

This is a single-context repo. Do not invent sub-context boundaries unless the repository adds them.

## Use the glossary's vocabulary

When naming a domain concept, use the term defined in `CONTEXT.md`. If the needed concept is not defined, reconsider the terminology or note the gap for domain modeling.

## Flag ADR conflicts

If output contradicts an existing ADR, surface the conflict explicitly instead of silently overriding it.
