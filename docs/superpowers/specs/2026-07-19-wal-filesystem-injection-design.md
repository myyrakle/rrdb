# WAL Filesystem Injection Design

## Goal

Remove direct operating-system filesystem access from `WALBuilder` so WAL discovery and loading can be tested through RRDB's shared `FileSystem` abstraction.

## Scope

This change covers read-only filesystem operations needed by WAL loading. WAL segment writing remains in `WALManager`, where mmap and file synchronization require concrete `std::fs::File` handles and are outside this review item.

## Common Filesystem API

Extend `FileSystem` with two asynchronous methods:

- `read_dir(&self, path: &str) -> io::Result<Vec<FileSystemEntry>>`
- `read(&self, path: &Path) -> io::Result<Vec<u8>>`

`FileSystemEntry` contains an owned `PathBuf` and an `is_file` boolean. This keeps Tokio-specific types and metadata access inside `RealFileSystem` while giving mock implementations a simple owned value. `RealFileSystem` collects the complete Tokio directory stream and propagates iteration and file-type errors.

## WALBuilder Construction

`WALBuilder` stores `Arc<dyn FileSystem + Send + Sync>`.

`WALBuilder::new(config)` remains the production convenience constructor and installs `RealFileSystem`. `WALBuilder::with_file_system(config, file_system)` provides explicit dependency injection for callers and tests. Keeping `new` avoids mechanical changes to every existing call site while still making the dependency replaceable.

## WAL Loading Flow

`load_data` calls `self.file_system.read_dir(&config.wal_directory)` and applies the existing regular-file, extension, hexadecimal sequence, sorting, and checkpoint-boundary logic to the returned entries. Each selected segment is loaded with `self.file_system.read(&path)` while preserving the existing path-aware error messages.

No `tokio::fs` or `std::fs` access remains in `builder.rs`.

## Testing

Add mock-driven tests that:

- Return two segment paths and their framed contents through `MockFileSystem`, then assert both pending entries are loaded in sequence order.
- Return a directory-read error and assert `WALBuilder::build` propagates it.
- Return a segment-read error and assert the error retains the affected path.

Existing real-filesystem WAL manager and recovery tests remain integration coverage for `RealFileSystem`.

## Error Handling

Common filesystem methods return `io::Error`. `WALBuilder` maps these into the existing `WALError` messages. Directory iteration errors are no longer hidden inside the concrete implementation, and segment read errors continue to identify the segment path.
