# WAL Filesystem Injection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route WAL segment discovery and reads through the common `FileSystem` abstraction.

**Architecture:** Add owned directory-entry and read APIs to `FileSystem`; keep Tokio types in `RealFileSystem`. Give `WALBuilder` a real-filesystem convenience constructor and an explicit injection constructor for mock-driven tests.

**Tech Stack:** Rust 2024, async-trait, mockall, Tokio, existing WAL codecs.

---

### Task 1: Extend the common filesystem abstraction

**Files:**
- Modify: `src/common/fs.rs`

- [ ] Add `FileSystemEntry { path: PathBuf, is_file: bool }` and async `read_dir`/`read` methods to `FileSystem`.
- [ ] Implement both methods in `RealFileSystem`, collecting every Tokio directory entry and propagating `next_entry` and `file_type` errors.
- [ ] Run `cargo test common` to verify compilation and existing behavior.

### Task 2: Inject the filesystem into WALBuilder

**Files:**
- Modify: `src/engine/wal/manager/builder.rs`
- Modify: `src/engine/wal/manager/mod.rs`

- [ ] Add a failing mock test whose expected `read_dir` and `read` calls provide two framed segment files without creating files on disk.
- [ ] Verify RED because `WALBuilder` cannot yet accept the mock.
- [ ] Store `Arc<dyn FileSystem + Send + Sync>` in `WALBuilder`.
- [ ] Keep `new(config)` backed by `RealFileSystem`; add `with_file_system(config, file_system)` for dependency injection.
- [ ] Replace `tokio::fs::read_dir`, `Path::is_file`, and `tokio::fs::read` with trait results.
- [ ] Add mock error tests for directory and segment reads, preserving segment-path context.
- [ ] Run `cargo test engine::wal::manager::tests` and confirm all tests pass.

### Task 3: Verify and integrate

**Files:**
- Verify: `src/common/fs.rs`
- Verify: `src/engine/wal/manager/builder.rs`
- Verify: `src/engine/wal/manager/mod.rs`

- [ ] Confirm `rg -n "tokio::fs|std::fs|\.is_file" src/engine/wal/manager/builder.rs` returns no matches.
- [ ] Run `cargo test`.
- [ ] Run Clippy and distinguish pre-existing repository warnings from new warnings.
- [ ] Commit with a `[#227]` message and update the remote branch with `--force-with-lease` only if history was rewritten.
