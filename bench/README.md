# Automated write benchmark

Compare RRDB and PostgreSQL using the **same deterministic single-row INSERT
workload and PostgreSQL simple-query client**. This is a small write benchmark,
not a general database ranking or a durability-equivalent benchmark.

## Run the comparison

Requirements: stable Rust, Python 3.10+, and a running Docker daemon. Run from
the repository root on Linux or macOS:

```sh
cargo build --release --bin rrdb
cargo build --release --manifest-path bench/Cargo.toml --bin main
python3 bench/compare.py --rows 1000 --workers 4 --repeats 3
```

The script creates a temporary RRDB data directory and a disposable
`postgres:16-alpine` container. Both ports are bound to loopback, with dynamic
port allocation. The container uses trust authentication **only for this local
test database**. It does not connect to, reset, or reuse existing databases.
RRDB processes, the PostgreSQL container, its anonymous volumes, and temporary
RRDB data are cleaned up on completion, failure, timeout, or ordinary signal
cancellation. Force-killing the orchestrator or losing the Docker daemon can
prevent cleanup; remaining containers are named `rrdb-bench-<unique ID>`.

Options:

- `--rows`: total writes per database per trial, 1–1,000,000 (default 1,000).
- `--workers`: concurrent writes, 1–64 and no greater than rows (default 4).
- `--repeats`: trials per database, 1–10 (default 3). The database order alternates.
- `--output`: **new**, non-existing output directory; default
  `bench/results/<unique ID>`. Existing directories are rejected to avoid stale results.
- `--postgres-image`: alternative PostgreSQL image/tag or digest. The default is
  a floating tag; the actual image ID and server version are recorded.
- `--rrdb-bin`, `--bench-bin`: override the two prebuilt release binary paths.

A quicker correctness smoke test:

```sh
python3 bench/compare.py --rows 20 --workers 2 --repeats 1
```

## Results and failure semantics

Each trial writes `<trial>-<backend>.json` and a client log. Results include:

- Backend, requested rows and workers, successful/failed writes, observed row count.
- Total measured seconds and successful writes per second.
- Per-write client latency in milliseconds: min, mean, p50, p95, max
  (nearest-rank percentiles; sub-millisecond values are preserved).
- Workload schema, payload size, protocol and measurement scope.

`environment.json` records revision, dirty-tree status, OS/architecture, CPU
count, compiler version, parameters, PostgreSQL version and Docker image ID.
`comparison.json` is emitted **only after every trial passes**. Server and client
logs remain in the output directory on failure; a failing client or mismatched
row count makes the script exit nonzero, never a successful comparison.

The root project currently ignores `Cargo.lock`. Preserve the generated root and
bench lockfiles alongside results when reproducing measurements; dependencies
can otherwise change between runs.

## Measurement boundaries

- The workload uses integer keys `0..rows` and deterministic 128-byte hexadecimal
  text values. Every trial owns a newly created uniquely named table with
  `key INTEGER PRIMARY KEY, value VARCHAR(128)`.
- Both targets use the same adapter, SQL, worker limit and simple-query protocol.
  The measured operations are one INSERT per request, with no application-level
  retries or explicit multi-statement transactions.
- Schema creation, connection setup, final row-count verification and table drop
  are outside the write measurement. Readback checks acknowledged visibility,
  **not recovery after restart or crash**.
- This is a write-only, closed-loop client workload, not a read/mixed workload,
  saturation search, cold-cache test, or sustained throughput guarantee. There is
  no dedicated warmup. Repeated trials reuse server processes and may warm caches.
- RRDB WAL is enabled and PostgreSQL uses its default settings. An acknowledged
  RRDB write and a PostgreSQL commit do **not** imply equal fsync, transaction,
  isolation, recovery or durability guarantees.
- RRDB runs natively, PostgreSQL in Docker. On macOS this also involves a Linux VM.
  Container/VM networking and filesystem differences affect results. Shared CI
  hosts add noise, so results must not be treated as proof one database is faster.

## Run one backend manually

Use **only a dedicated test database**. The client creates a unique table and
only drops that table if creation succeeded. It never drops a pre-existing table.
Specify the connection via `BENCH_DATABASE_URL`, not a command-line argument;
do not put real credentials in shell history or CI logs.

```sh
BENCH_DATABASE_URL=postgres://rrdb@127.0.0.1:5432/rrdb \
  bench/target/release/main postgres --rows 1000 --workers 4 --output /tmp/pg-run.json
```

Use `rrdb` instead of `postgres` for the backend label. Both use the same adapter;
the URL selects the server. The output file must not already exist. Connection
and query errors are sanitized; measured writes are not retried.

The old CSV generator remains available as `cargo run --manifest-path
bench/Cargo.toml --bin gen`; its output is not used by the comparison.

## Automation and tests

`.github/workflows/benchmark.yml` runs on relevant pull requests, pushes to
`master`, and manual dispatch. It checks the bench package and Python runner,
builds release binaries, compares 1,000 rows / 4 workers / 3 trials, and uploads
JSON results, server/client logs, and dependency lockfiles (including on failure).
It has read-only repository permissions and does not post comments or commit
results. Functional failures fail the job; relative performance does not.

```sh
cargo test --manifest-path bench/Cargo.toml
cargo clippy --manifest-path bench/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path bench/Cargo.toml -- --check
python3 -m unittest discover -s bench -p 'test_*.py'
```
