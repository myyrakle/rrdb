# WAL Multi-Segment Replay Design

## Goal

Prevent recovery data loss by replaying every WAL mutation after the most recent checkpoint, even when those mutations span multiple segment files, and stop recovery without checkpointing when any replay entry fails.

## Scope

This change covers multi-segment WAL discovery and replay failure propagation. Transaction-aware replay and enforcement of the `wal_enabled` configuration remain outside this change.

## WAL Loading

`WALBuilder` discovers every regular file whose extension matches `wal_extension` and whose stem is a valid hexadecimal segment sequence. It sorts those files by numeric sequence and reads them in ascending order.

Every discovered segment is read and decoded with the existing framed WAL decoder. A read error, truncated frame, or invalid encoded entry fails the build. Recovery must not silently skip an unreadable segment because a later segment can depend on mutations recorded in it.

The builder combines decoded entries in sequence order and retains only entries after the last `Checkpoint`. A checkpoint therefore establishes the recovery boundary across segment files rather than only within the newest file.

The writer resumes the highest existing segment when its last decoded entry is not a checkpoint, using that segment's measured frame offset. If the newest segment ends in a checkpoint, it starts at the following sequence with offset zero. An empty highest segment is treated as the current writable segment at offset zero.

## Replay Failure Semantics

`DBEngine::replay_wal` processes pending entries in order. Payload deserialization failures and mutation execution failures are returned immediately. Recovery does not continue past the failed entry because later operations may depend on it.

`recover_from_wal` preserves its ordering:

1. Replay all pending WAL entries.
2. Flush replay-created row buffers durably.
3. Write and sync the recovery checkpoint.

An error in either of the first two steps returns before `WALManager::flush`, leaving the WAL recovery boundary intact for diagnosis and a later retry. A successful recovery checkpoints the replayed entries as before.

## Error Handling

Errors identify the segment path when file reading or decoding fails. Replay errors identify the entry type and its position in the pending sequence while retaining the underlying error text. Startup propagates these errors and does not accept client connections with partially recovered state.

## Testing

Tests follow a red-green sequence and cover:

- Loading pending entries from two uncheckpointed segments in sequence order.
- Excluding entries at and before the last checkpoint when the checkpoint is in an earlier segment.
- Rejecting a corrupt intermediate segment even when a later valid segment exists.
- Returning an error for an invalid replay payload instead of skipping it.
- Verifying that failed recovery does not append a checkpoint.

Existing single-segment loading, WAL append, rotation, durability flush, and replay tests remain regression coverage. The focused WAL tests run first, followed by the complete test suite and Clippy.
