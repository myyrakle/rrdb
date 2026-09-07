# Indexed row reads by frame offset (#221)

## Contract

The existing B-tree row IDs and row/WAL formats are unchanged. A rebuildable in-memory directory maps logical row slots to frame-body offsets. A cold directory walks five-byte frame headers and skips bodies. Warm point/range lookups request only matched bodies, in index order, without cloning all cached rows.

Buffered appends and dirty decoded rows take precedence over disk. Successful append flushes extend a warm directory; rewrites and failed writes invalidate it. DROP removes pending, decoded, unsynced and offset state under the storage lock. Physical rename invalidates derived offsets under old/new namespaces even when the later config update fails; it does not repair existing rename semantics.

One directory retains up to 16 MiB of offset entries. The limit is a cache bound, not a table-size limit: an uncached suffix is located by header traversal. Appending to an incomplete directory preserves its contiguous prefix. Cold construction remains O(number of frames); suffix lookups and switching tables can require additional walks. Each uncached match restarts at the retained prefix boundary: selecting the first K suffix rows therefore performs K*(K+1)/2 header reads. The benchmark below stays below the cap and does not establish above-cap range scalability.

Frame bounds are checked before payload allocation. Query memory is reserved before directory growth, payload allocation, decode estimates and selected-row clones. Cold statistics count live headers rather than decoding the row segment. Index metadata/statistics work is not eliminated.

## Verification

Tests cover point/range read requests, unrelated invalid payloads, malformed/truncated frames, stale pointers, memory limits, pending rows, growth/shrink rewrites, tombstones, append, reopen, namespace reuse, failure invalidation and WAL-before-apply replay. A separate model test performs 96 SQL mutations and 24 engine reopens, comparing indexed/full reads to a BTreeMap reference.

The original applied-SQL-tail crash fixture also fails on the base revision: already-persisted index changes can conflict with replay while buffered row changes are lost. This PR does not fix that existing WAL/index crash window, general flush-retry durability, or existing decoded/pending/index/config rename defects. Passing WAL-before-apply tests is not a claim that all crash windows are safe.

## Reproducible benchmark

```sh
cargo test --release --lib offset_read_benchmark -- --ignored --nocapture
```

The benchmark asserts results for 1,000 and 10,000 rows, point queries and 64-row ranges. It invokes row-read methods directly, not SQL planning. Cold means decoded/offset caches are reset; **OS caches are not evicted**. Warm offset runs retain offsets but no decoded-row cache. Full scans materialize all rows and then filter the same locations.

Example run: macOS ARM64, Rust 1.97.1, release build; 10,000 rows, 10,960,000-byte segment. Means are illustrative, not latency assertions:

| Case | Mean per lookup | Row-segment bytes per lookup |
| --- | ---: | ---: |
| Full scan, cold decoded cache, point | 12,292.9 us | 10,960,000 expected |
| Offset point, cold directory | 125,430.6 us | 51,091 requested |
| Offset point, warm directory | 58.8 us | 1,091 requested |
| Offset range, cold directory | 121,151.1 us | 119,824 requested |
| Offset range, warm directory | 864.2 us | 69,824 requested |
| Full scan, cold decoded cache, range | 2,731.7 us | 10,960,000 expected |
| Full scan, warm decoded cache, range | 1,138.4 us | 0 expected |

Requested bytes are observed application-level random-read lengths, not physical device I/O. Full-scan expected bytes are derived from file length and cache resets because its existing direct reader bypasses the random-access decorator. Index-file I/O is not counted. Output counters aggregate all repetitions; divide by `repeats` for per-lookup values.

Cold per-header asynchronous seeks can be substantially slower than sequential materialization despite requesting fewer bytes. Small warm ranges can also lose to an already-decoded full cache. This is not a universal speedup claim. Cost constants remain unchanged: the existing formula describes matched-row random access, while cold-directory cost and cache state remain limitations rather than unmeasured reasons to lower costs.
