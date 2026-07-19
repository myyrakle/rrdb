# WAL Multi-Segment Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replay every WAL mutation after the latest checkpoint across all segment files and abort recovery without checkpointing when replay fails.

**Architecture:** `WALBuilder` sorts and decodes every valid WAL segment, clearing its pending-entry suffix whenever it encounters a checkpoint. `DBEngine::replay_wal` propagates the first contextual error, while `recover_from_wal` checkpoints only after replay and durable row flushing succeed.

**Tech Stack:** Rust 2024, Tokio async filesystem/tests, bincode framed WAL encoding, RRDB error types.

---

## File Map

- `src/engine/wal/manager/builder.rs`: ordered segment discovery and recovery-boundary calculation.
- `src/engine/wal/manager/mod.rs`: loader and corruption regression tests.
- `src/engine/mod.rs`: fail-fast replay behavior and focused replay test.
- `src/engine/wal/recovery.rs`: failed-recovery checkpoint regression test.

### Task 1: Multi-segment recovery loading

**Files:**
- Modify: `src/engine/wal/manager/mod.rs`
- Modify: `src/engine/wal/manager/builder.rs`

- [ ] **Step 1: Add the failing multi-segment test**

Add a test that writes an `Insert` payload `segment-1` to sequence 1 and a `Delete` payload `segment-2` to sequence 2 using `write_wal_file`. Build the manager and assert exactly these two payloads appear in `pending_entries()` in sequence order, and `current_sequence()` is 2.

```rust
let payloads: Vec<_> = wal_manager
    .pending_entries()
    .iter()
    .map(|entry| entry.data.clone().unwrap())
    .collect();
assert_eq!(payloads, vec![b"segment-1".to_vec(), b"segment-2".to_vec()]);
assert_eq!(wal_manager.current_sequence(), 2);
```

- [ ] **Step 2: Verify RED**

Run `cargo test engine::wal::manager::tests::test_build_loads_pending_entries_from_all_segments -- --exact`.

Expected: FAIL because the current builder loads only sequence 2.

- [ ] **Step 3: Add checkpoint-boundary coverage**

Write sequence 1 with `Insert("durable")` followed by `Checkpoint`, and sequence 2 with `Set("pending")`. Assert the manager exposes only the `Set` entry. This test documents that the last checkpoint applies across files.

- [ ] **Step 4: Add the failing corrupt-intermediate-segment test**

Write valid segments 1 and 3. Between them, create sequence 2 with a frame length of 8 but only two body bytes:

```rust
tokio::fs::write(
    wal_dir.join(format!("00000002.{}", config.wal_extension)),
    [8, 0, 0, 0, 1, 2],
)
.await
.unwrap();
```

Assert `WALBuilder::build` returns an error containing both `00000002` and `truncated wal frame body`. Run `cargo test engine::wal::manager::tests::test_build_rejects_corrupt_intermediate_segment -- --exact` and verify RED because the current loader ignores sequence 2.

- [ ] **Step 5: Implement ordered loading**

Replace the single `max_sequence`/`last_log_path` scan with `Vec<(usize, PathBuf)>`. Include only regular files with the configured extension and a hexadecimal stem, then call:

```rust
segments.sort_by_key(|(sequence, _)| *sequence);
```

For every segment in order:

```rust
let content = tokio::fs::read(&path).await.map_err(|error| {
    WALError::wrap(format!("failed to read log file {:?}: {}", path, error))
})?;
let used_bytes = used_wal_bytes(&content).map_err(|error| {
    WALError::wrap(format!("failed to inspect log file {:?}: {}", path, error))
})?;
let entries = decoder.decode(&content).map_err(|error| {
    WALError::wrap(format!("failed to decode log file {:?}: {}", path, error))
})?;

for entry in &entries {
    if matches!(entry.entry_type, EntryType::Checkpoint) {
        pending_entries.clear();
    } else {
        pending_entries.push(entry.clone());
    }
}
```

Remember the newest segment's decoded entries and `used_bytes`. Return `(1, [], 0)` when no segment exists. If the newest entry is a checkpoint, return `(max_sequence + 1, pending_entries, 0)`; otherwise return `(max_sequence, pending_entries, newest_offset)`. An empty newest segment therefore resumes at its own sequence and offset zero.

- [ ] **Step 6: Verify GREEN**

Run `cargo test engine::wal::manager::tests -- --nocapture`.

Expected: all builder tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/engine/wal/manager/builder.rs src/engine/wal/manager/mod.rs
git commit -m "fix: replay WAL across segment boundaries"
```

### Task 2: Abort replay on the first error

**Files:**
- Modify: `src/engine/mod.rs`

- [ ] **Step 1: Add a failing payload test**

Construct a default `DBEngine` and one `WALEntry` with `EntryType::Insert` and payload `vec![0xff]`. Call `replay_wal`, require `unwrap_err()`, and assert its display contains `entry 0` and `Insert`.

```rust
let entry = WALEntry {
    entry_type: EntryType::Insert,
    data: Some(vec![0xff]),
    timestamp: 1,
    transaction_id: None,
    is_continuation: false,
};
let error = engine.replay_wal(&[entry]).await.unwrap_err();
assert!(error.to_string().contains("entry 0"));
assert!(error.to_string().contains("Insert"));
```

- [ ] **Step 2: Verify RED**

Run `cargo test engine::wal_replay_tests::replay_wal_returns_contextual_error_for_invalid_payload -- --exact`.

Expected: FAIL because the current method logs the error and returns success.

- [ ] **Step 3: Implement fail-fast replay**

Enumerate the existing replay loop and preserve its `EntryType` match. Replace the warning-and-continue block with:

```rust
result.map_err(|error| {
    ExecuteError::wrap(format!(
        "WAL replay failed at entry {} ({:?}): {}",
        index, entry.entry_type, error
    ))
})?;
```

Update the method comment to state that any decode or application failure aborts recovery. Do not change transaction-entry handling in this task.

- [ ] **Step 4: Verify GREEN and existing index replay**

Run:

```bash
cargo test engine::wal_replay_tests -- --nocapture
cargo test engine::actions::ddl::create_index::tests -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/engine/mod.rs
git commit -m "fix: abort WAL replay on entry failure"
```

### Task 3: Preserve WAL when recovery fails

**Files:**
- Modify: `src/engine/wal/recovery.rs`

- [ ] **Step 1: Add a recovery-level regression test**

Create a unique directory below `target/test_wal_recovery`, build a manager, append an invalid `Insert` payload, and call `sync()`. Run `recover_from_wal` and assert it returns an error. Decode `00000001.<extension>` and assert it contains exactly the invalid entry and no `Checkpoint`.

```rust
assert!(engine.recover_from_wal(&mut wal_manager).await.is_err());
let entries = BincodeDecoder::new().decode(&content).unwrap();
assert_eq!(entries.len(), 1);
assert!(!entries
    .iter()
    .any(|entry| matches!(entry.entry_type, EntryType::Checkpoint)));
```

- [ ] **Step 2: Run the focused test**

Run `cargo test engine::wal::recovery::tests::failed_recovery_does_not_append_checkpoint -- --exact`.

Expected after Task 2: PASS. The existing `?` ordering already prevents `WALManager::flush` after replay failure, so no production change is expected in `recovery.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/engine/wal/recovery.rs
git commit -m "test: preserve WAL after replay failure"
```

### Task 4: Full verification

**Files:**
- Verify: `src/engine/wal/manager/builder.rs`
- Verify: `src/engine/wal/manager/mod.rs`
- Verify: `src/engine/mod.rs`
- Verify: `src/engine/wal/recovery.rs`

- [ ] **Step 1: Format and inspect**

Run `cargo fmt --check` and `git diff --check`.

Expected: both commands succeed.

- [ ] **Step 2: Run all tests**

Run `cargo test`.

Expected: all tests pass.

- [ ] **Step 3: Run Clippy**

Run `cargo clippy --all-targets --all-features -- -D warnings`.

Expected: no warnings or errors.

- [ ] **Step 4: Confirm scope**

Run `git status --short` and `git log -5 --oneline`.

Expected: only the intended WAL loader, replay, recovery-test, design, and plan changes are present.
