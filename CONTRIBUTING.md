# Contributing to stardust-core

`stardust-core` is the shared Rust workspace that powers Stardust Pit. Audio I/O, MIDI, realtime primitives, DSP nodes, patch graph + show document data models, CLAP host. UI-agnostic on purpose — see [ADR-0008](https://github.com/StardustMT/stardust-workspace/blob/main/docs/adr/0008-crate-organization.md) for the crate layout.

This guide is the short version. Full conventions for both humans and Claude are in [`CLAUDE.md`](https://github.com/StardustMT/stardust-workspace/blob/main/CLAUDE.md).

## The three living documents

`stardust-core` shares the [Stardust Pit Project board](https://github.com/orgs/StardustMT/projects/1) and [docs site](https://stardustmt.github.io/docs/pit/). Most issues live in the [`stardust-pit`](https://github.com/StardustMT/stardust-pit) repo; core-specific work (a new node type, a crate refactor, a stardust-show schema change) is filed here.

- **Roadmap** — [pit roadmap](https://stardustmt.github.io/docs/pit/roadmap/) drives both repos
- **Issues** — cross-repo, on the shared project board
- **Docs** — [engine architecture](https://stardustmt.github.io/docs/pit/architecture/engine/) is the doc most relevant to core work

## Filing an issue

Use one of the templates:

- **Feature** — new functionality (new node type, new audio/MIDI primitive)
- **Task** — refactors, tech-debt, crate reorganization, CI
- **Bug** — broken behavior

Set the **milestone** and add an **area label** (most core work is `engine:*`). Leave **Estimate** and **Priority** blank until refinement.

## Architectural rules (durable)

These don't change without an ADR:

- **UI-agnostic.** `stardust-core` must not depend on Tauri, React, or any frontend lifecycle. UI consumers (stardust-pit) wrap us; we don't wrap them.
- **Realtime paths allocation-free.** Audio callback + MIDI dispatch + per-block processing — no allocation, no locks, no syscalls. Add `rt-assert`-style checks where realtime is non-obvious.
- **`!Send` plugin instances pinned to one thread.** clack-host's `PluginInstance<H>` is `!Send`; the engine thread is the only one that touches plugins.
- **Out-of-process plugin processes** (v0.7.0+) — shared-memory ring buffers, sub-ms IPC.
- **Schema versioning** — any persisted data type (patch, show, library entry) carries a schema version and a migration path per [ADR-0003](https://github.com/StardustMT/stardust-workspace/blob/main/docs/adr/0003-schema-versioning.md).
- **Flat workspace** — group crates by naming convention (`stardust-audio-*`, etc.); revisit nesting past ~15 crates ([ADR-0008](https://github.com/StardustMT/stardust-workspace/blob/main/docs/adr/0008-crate-organization.md)).

## Picking up an issue

1. **Self-assign** + flip Status to **🔨 In Progress** on the board
2. **Branch** off `main`: `git checkout -b <type>/<short-slug>`
3. **Read the acceptance criteria.** If anything's hand-wavy, comment + clarify before coding
4. **Log meaningful decisions** as issue comments while you work
5. **New work surfaces** → file a new issue + cross-link (native sub-issues for parent/child)

## Commits

- Reference the issue: `Add 3-band stereo EQ (StardustMT/stardust-pit#X)` — when referencing a pit issue from a core commit, use the cross-repo form
- One logical change per commit; commits should compile + `cargo test --workspace` passes
- **No `Co-Authored-By: Claude`** in messages

## Pull requests

The PR template asks you to:

- Link the issue this closes
- Paste the acceptance-criteria checklist with ticks
- Note tests run + docs updated

PRs land small. CI runs `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test --workspace` (once [#10](https://github.com/StardustMT/stardust-pit/issues/10) lands).

## Closing an issue

When the work ships:

- Close with a comment: **what shipped** + commit/PR ref
- Status → **✅ Done** on the board (or **🧊 Deferred** with rationale)
- Update affected docs (typically [architecture/engine](https://stardustmt.github.io/docs/pit/architecture/engine/) for engine work, or feature pages for new node types)

## Issue field reference

Identical to the [pit CONTRIBUTING](https://github.com/StardustMT/stardust-pit/blob/main/CONTRIBUTING.md#issue-field-reference) — the project board is shared. Key labels for core work:

- `engine:audio` · `engine:midi` · `engine:plugin` · `engine:transport` · `engine:graph` — primary areas
- `tech-debt` · `infrastructure` · `documentation` — cross-cutting
- `needs-refinement` — umbrella issues awaiting a spec session

## Where things live

- **Shared Rust libraries** — this repo (`stardust-core`)
- **App** — [`stardust-pit`](https://github.com/StardustMT/stardust-pit)
- **Docs + roadmap** — [`stardustmt.github.io`](https://github.com/StardustMT/stardustmt.github.io)
- **ADRs + workspace conventions** — [`stardust-workspace`](https://github.com/StardustMT/stardust-workspace)
