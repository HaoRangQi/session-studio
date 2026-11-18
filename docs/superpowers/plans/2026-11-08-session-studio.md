# Session Studio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first macOS desktop app for discovering, reviewing, exporting, safely trashing, and analyzing Claude Code, Codex, DeepSeek Harness, and Pi sessions, with read-only monitoring of supported CLI processes.

**Architecture:** Tauri 2 with a Rust backend, React/TypeScript/Vite frontend, and SQLite metadata/task store. A shared domain API is backed by independent session/activity/CLI adapters; all source-specific parsing, process matching, export, and trash behavior stays behind those adapters. Unknown formats/capabilities fail closed and remain visible as unsupported instead of being guessed.

**Tech Stack:** Tauri 2, Rust, rusqlite/SQLite, React, TypeScript, Vite, Vitest, Rust unit/integration tests.

---

## File Map

- Create `package.json`, `package-lock.json`, `tsconfig.json`, `vite.config.ts`, `index.html`: frontend build/test configuration.
- Create `src/main.tsx`, `src/App.tsx`, `src/styles.css`: functional desktop UI shell and navigation.
- Create `src/domain/types.ts`, `src/domain/format.ts`: shared frontend DTOs, display labels, safe formatting.
- Create `src/lib/commands.ts`: typed Tauri invoke/event boundary.
- Create `src/components/Sidebar.tsx`, `src/views/SessionsView.tsx`, `src/views/ProjectsView.tsx`, `src/views/CleanupView.tsx`, `src/views/ActivityView.tsx`, `src/views/SettingsView.tsx`: focused views for approved navigation/workflows.
- Create `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`: desktop runtime and security capability configuration.
- Create `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`: application startup and Tauri command registration.
- Create `src-tauri/src/domain/{mod.rs,session.rs,source.rs,activity.rs,cleanup.rs,export.rs,analysis.rs}`: source-neutral domain models and policies.
- Create `src-tauri/src/storage/{mod.rs,db.rs,migrations.rs,session_repo.rs,task_repo.rs}`: SQLite schema, migrations, and persistence.
- Create `src-tauri/src/providers/{mod.rs,claude_code.rs,codex.rs,deepseek_harness.rs,pi.rs,process.rs,cli.rs,trash.rs}`: independent source adapters and macOS integrations.
- Create `src-tauri/tests/{storage.rs,cleanup.rs,export.rs,providers.rs}` and `src/**/*.test.tsx`: focused automated tests and safe fixtures.
- Create `tests/fixtures/{claude,codex,pi}/` only from synthetic fixtures or user-approved redacted samples; never copy live conversation data into the repository.
- Update `README.md`, `docs/context/MAP.md`, `docs/context/RUNBOOK.md`, `docs/context/NOW.md`, `docs/context/RISKS.md`, `docs/context/DECISIONS.md`, `PROJECT_INDEX.md` as verified implementation facts change.

## Task 1: Bootstrap Tauri 2 + React/TypeScript

**Files:** frontend and Tauri configuration files from the file map.

- [ ] Check target root contains no existing app files; preserve all existing docs and `.ccgui/`.
- [ ] Create the minimal Tauri 2 + React/TypeScript/Vite shell and a window showing the Session Studio navigation without scanning user data.
- [ ] Run `npm install` and `npm run build`; fix only errors from this bootstrap.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`; expected startup crate compiles and test harness passes.
- [ ] Run Tauri development launch only if the available macOS SDK can compile the GUI; record a toolchain blocker instead of changing system Xcode configuration.

## Task 2: Define Domain Contracts First

**Files:** `src-tauri/src/domain/*.rs`, `src/domain/types.ts`, `src/lib/commands.ts`, matching unit tests.

- [ ] Write Rust tests for source IDs, session/container identity, source health states, candidate reasons, message ranges, and activity certainty.
- [ ] Run the tests and verify failures are due to missing domain definitions.
- [ ] Define source-neutral `SessionSummary`, `Message`, `AttachmentRef`, `SourceStatus`, `CleanupContainer`, `CleanupPreview`, `ActivityRecord`, and `AnalysisTask` types; represent unknown values explicitly rather than using empty strings as evidence.
- [ ] Define the provider trait boundary for metadata scan, session parse, container mapping, export capabilities, trash capabilities, activity association, and CLI capabilities.
- [ ] Define typed frontend DTOs that match Rust serialization and typed wrappers for Tauri commands.
- [ ] Run Rust and TypeScript tests/build; ensure serialization names/enums match.

## Task 3: SQLite Metadata and Task Persistence

**Files:** `src-tauri/src/storage/*`, `src-tauri/tests/storage.rs`.

- [ ] Write integration tests for schema creation, source scan status, session upsert, stable-ID updates, container mapping, candidate labels, task output retention, and clear/rebuild behavior.
- [ ] Verify each test fails before implementation.
- [ ] Implement SQLite migrations and repository methods under the app-private data directory; enable foreign keys and transactions.
- [ ] Store session metadata only; do not store parsed conversation bodies in the session index.
- [ ] Store AI task content/output separately with explicit user deletion; do not log prompt/input/output or credentials.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`; verify persistence and migration tests pass.

## Task 4: Provider Registry and Honest Source Status

**Files:** `src-tauri/src/providers/{mod.rs,claude_code.rs,codex.rs,deepseek_harness.rs,pi.rs}`, `src-tauri/tests/providers.rs`.

- [ ] Write provider contract tests for missing installation, permission denied, no sessions, unsupported format, malformed input, and successful scan.
- [ ] Implement a registry with four named providers and explicit capability/status values.
- [ ] Implement safe source-root discovery using known-path candidates only; do not recursively scan `$HOME` or arbitrary hidden folders.
- [ ] Do not read session bodies during metadata scans.
- [ ] For Claude Code, Codex, and Pi, implement parsing only after a synthetic fixture is grounded in a verified format or a user-approved redacted real sample; preserve unknown fields only when they do not leak sensitive payloads into the index.
- [ ] For DeepSeek Harness, use only a user-selected/authorized JSONL persistence `root`; official docs say it has no default. Support versioned JSONL only after implementing bounded header reads and verified Zstandard decoding. Current environment lacks the `dsh` executable and configured root, so leave the provider `needs_validation` until an installed CLI, configured root, and representative sample are available.
- [ ] Run provider tests and verify unsupported/permission errors never appear as a successful empty result.

## Task 5: Candidate Rules and Project/Session Views

**Files:** `src-tauri/src/domain/session.rs`, `src-tauri/src/storage/session_repo.rs`, `src/views/SessionsView.tsx`, `src/views/ProjectsView.tsx`, tests.

- [ ] Write tests for missing project path, no project association, 90-day inactivity, unknown/permission-denied path, and status refresh.
- [ ] Implement three distinct candidate reasons: missing project directory, unassociated session, and inactive for at least 90 days; do not label them all “orphan”.
- [ ] Implement global session queries and project aggregation over the same indexed records.
- [ ] Add source/project/time filters and search only over indexed metadata; do not claim body full-text search.
- [ ] Run Rust/React tests and verify project view groups the same sessions without duplicating records.

## Task 6: Activity Monitoring and Cleanup Safety

**Files:** `src-tauri/src/providers/process.rs`, `src-tauri/src/domain/activity.rs`, `src-tauri/src/domain/cleanup.rs`, `src-tauri/src/providers/trash.rs`, `src-tauri/tests/cleanup.rs`.

- [ ] Write tests for known CLI process recognition, child process grouping, working directory, start time, unknown process association, active-container block, and batch partial success.
- [ ] Implement monitoring for only the supported CLI executables and their children; expose state/working directory/start time and no CPU/memory/token/cost metrics.
- [ ] Implement container-level deduplication and safety recheck immediately before trashing.
- [ ] Block deletion for active or indeterminate container associations.
- [ ] Implement macOS Trash integration using a verified system API; if the selected API cannot move a directory/database container safely, report unsupported and do not fall back to permanent deletion or shell `rm`.
- [ ] Implement preview with exact container path, contained session count, impact list, blocked reason, and executable status.
- [ ] Implement item-by-item batch outcomes so blocked/failed items do not prevent unrelated approved items from proceeding.
- [ ] Run tests against a temporary fixture directory only; confirm tests never touch the real agent data roots.

## Task 7: Markdown and JSON Export

**Files:** `src-tauri/src/domain/export.rs`, `src-tauri/tests/export.rs`, export command/UI tests.

- [ ] Write tests for full messages, timestamps, tool calls/results, source metadata, attachment references, escaping, UTF-8, and multiple sessions.
- [ ] Implement deterministic Markdown/JSON serializers from parsed message DTOs.
- [ ] Keep attachment bytes out of exports; include references only.
- [ ] Add user-chosen destination save flow and per-session failure reporting.
- [ ] Run tests with synthetic sessions and verify no live content is used.

## Task 8: Safe CLI Analysis Tasks

**Files:** `src-tauri/src/providers/cli.rs`, `src-tauri/src/domain/analysis.rs`, `src-tauri/src/storage/task_repo.rs`, `src-tauri/tests/providers.rs`, analysis UI tests.

- [ ] Write tests for message-range selection, exact preview/input match, fixed argument arrays, no shell interpolation, cancellation, exit status, partial output retention, and secret-safe logs.
- [ ] Implement per-provider CLI capability descriptions; default to no CLI analysis if the executable/flags cannot be verified.
- [ ] Build one analysis input from multiple selected sessions, clearly labeling each source/session and selected message range.
- [ ] Prefer verified ephemeral/no-history mode; if unavailable, require an explicit confirmation that a new CLI session will be created.
- [ ] Configure read-only execution where the CLI guarantees it. If it cannot guarantee read-only, show the requested extra warning/confirmation and do not imply the process is sandboxed.
- [ ] Show the exact input, custom instruction, detectable model/backend, unknown-backend external-transfer warning, and per-run confirmation before spawn.
- [ ] Spawn with `std::process::Command` argument vectors, minimal validated environment, and a non-sensitive working directory; do not invoke a shell.
- [ ] Stream stdout/stderr into a local task record, support cancellation only for app-owned child processes, and retain exit code/error/partial output until user deletion.
- [ ] Never call a real model as part of automated tests; test with fixture executables that print, fail, block, and emit partial output.

## Task 9: Functional UI and Source Status

**Files:** `src/App.tsx`, `src/components/Sidebar.tsx`, `src/views/*.tsx`, `src/styles.css`, `src/**/*.test.tsx`.

- [ ] Write UI tests for fixed sidebar, global/project views, candidate filters, activity view, settings source status, blocked cleanup preview, partial outcomes, export, and AI confirmation.
- [ ] Implement the approved utilitarian UI without decorative marketing content or unsupported capability claims.
- [ ] Keep source status center in Settings and show compact source-state badges in filters/results.
- [ ] Clearly distinguish scanning, empty-success, uninstalled, permission denied, unsupported format, failed, and needs-validation states.
- [ ] Implement user-authorized per-source directory selection; do not request the entire home directory or Full Disk Access.
- [ ] Run frontend tests/build and verify Tauri invoke argument/result types compile.

## Task 10: End-to-End Verification and Documentation

**Files:** tests/fixtures, `README.md`, `PROJECT_INDEX.md`, `docs/context/{MAP.md,RUNBOOK.md,NOW.md,RISKS.md,DECISIONS.md}`.

- [ ] Run Rust unit/integration tests and frontend tests/build.
- [ ] On a macOS environment with a working Tauri toolchain, run a manual test against a temporary authorized directory: scan, browse, export, trash a disposable fixture container, restore it from Trash, launch/cancel/fail a fake CLI analysis process, and verify activity association protection.
- [ ] For each of Claude Code, Codex, DeepSeek Harness, and Pi, record real-sample discovery/browse/export/trash/CLI-analysis acceptance separately. Do not mark the four-source MVP complete until all four pass.
- [ ] Record current host blockers: DeepSeek Harness CLI absent; full Xcode absent. Do not install Xcode or access real conversation contents without explicit user direction.
- [ ] Update README and MAP with implemented facts only; update NOW with completed work, verification evidence, blockers, and next action.
- [ ] Check only changed files and direct links; no repository-wide audit is in scope.
