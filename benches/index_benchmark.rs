//! Benchmarks for the B-tree index system (issue #160, PR #191)
//!
//! Run with:
//!   cargo bench --bench index_benchmark
//!
//! Measures:
//!   1. BTreeIndex in-memory operations (insert, get, range, remove, update)
//!   2. IndexManager memory+disk operations (insert with flush, get, range, reload)
//!   3. Scaling: 1K, 10K, 50K, 100K entries

use std::time::{Duration, Instant};

use rrdb::engine::ast::types::TableName;
use rrdb::engine::index::IndexMeta;
use rrdb::engine::index::btree::BTreeIndex;
use rrdb::engine::index::manager::IndexManager;

// ─── Helpers ────────────────────────────────────────────────────────────

fn make_int_key(v: i64) -> String {
    format!("I:{:020}", v)
}

fn make_meta(name: &str, column: &str, unique: bool) -> IndexMeta {
    IndexMeta::new(
        name.to_string(),
        TableName {
            database_name: Some("benchdb".to_string()),
            table_name: "benchtable".to_string(),
        },
        column.to_string(),
        unique,
    )
}

fn fmt_duration(d: Duration) -> String {
    if d.as_secs() > 0 {
        format!("{:.2} s", d.as_secs_f64())
    } else if d.as_millis() > 0 {
        format!("{:.2} ms", d.as_secs_f64() * 1000.0)
    } else if d.as_micros() > 0 {
        format!("{:.2} us", d.as_secs_f64() * 1_000_000.0)
    } else {
        format!("{:.2} ns", d.as_secs_f64() * 1_000_000_000.0)
    }
}

fn fmt_throughput(n: usize, d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs > 0.0 {
        format!("{} ops/sec", (n as f64 / secs).floor() as u64)
    } else {
        "n/a".to_string()
    }
}

struct BenchResult {
    name: String,
    n: usize,
    total: Duration,
}

impl BenchResult {
    fn print(&self) {
        println!(
            "  {:<50} n={:>7}  total={:>12}  throughput={:>16}",
            &self.name,
            self.n,
            fmt_duration(self.total),
            fmt_throughput(self.n, self.total),
        );
    }
}

// ─── BTreeIndex In-Memory Benchmarks ────────────────────────────────────

fn bench_btree_insert(n: usize) -> BenchResult {
    let mut tree = BTreeIndex::new("id".to_string(), false);
    let start = Instant::now();
    for i in 0..n as i64 {
        let key = make_int_key(i);
        let path = format!("/r/{}", i);
        tree.insert(key, path).unwrap();
    }
    let total = start.elapsed();
    BenchResult {
        name: format!("BTreeIndex::insert (n={})", n),
        n,
        total,
    }
}

fn bench_btree_get(tree: &BTreeIndex, n: usize) -> BenchResult {
    let start = Instant::now();
    let mut found = 0;
    for i in 0..n as i64 {
        let key = make_int_key(i);
        if !tree.get(&key).is_empty() {
            found += 1;
        }
    }
    let total = start.elapsed();
    assert_eq!(found, n, "get should find all inserted keys");
    BenchResult {
        name: format!("BTreeIndex::get (exact lookup, n={})", n),
        n,
        total,
    }
}

fn bench_btree_get_miss(tree: &BTreeIndex, n: usize) -> BenchResult {
    let start = Instant::now();
    for i in 0..n as i64 {
        let key = make_int_key(i + 1_000_000);
        let _ = tree.get(&key);
    }
    let total = start.elapsed();
    BenchResult {
        name: format!("BTreeIndex::get (miss, n={})", n),
        n,
        total,
    }
}

fn bench_btree_range(tree: &BTreeIndex, n: usize, range_size: usize) -> BenchResult {
    let half = (n / 2) as i64;
    let start_key = make_int_key(half);
    let end_key = make_int_key(half + range_size as i64);
    let iterations = 1000;
    let start = Instant::now();
    let mut total_entries = 0;
    for _ in 0..iterations {
        let results = tree.range(Some(&start_key), Some(&end_key));
        total_entries += results.len();
    }
    let total = start.elapsed();
    assert!(total_entries > 0, "range should find entries");
    BenchResult {
        name: format!(
            "BTreeIndex::range ({} entries, {}x)",
            range_size, iterations
        ),
        n: iterations,
        total,
    }
}

fn bench_btree_scan_all(tree: &BTreeIndex, n: usize) -> BenchResult {
    let iterations = 50;
    let start = Instant::now();
    let mut total_entries = 0;
    for _ in 0..iterations {
        let entries = tree.scan_all();
        total_entries += entries.len();
    }
    let total = start.elapsed();
    assert_eq!(
        total_entries,
        n * iterations,
        "scan_all should return all entries"
    );
    BenchResult {
        name: format!("BTreeIndex::scan_all (n={}, {}x)", n, iterations),
        n: iterations,
        total,
    }
}

fn bench_btree_remove(n: usize) -> BenchResult {
    let mut tree = BTreeIndex::new("id".to_string(), false);
    for i in 0..n as i64 {
        tree.insert(make_int_key(i), format!("/r/{}", i)).unwrap();
    }

    let start = Instant::now();
    for i in 0..n as i64 {
        let key = make_int_key(i);
        let path = format!("/r/{}", i);
        tree.remove(&key, &path);
    }
    let total = start.elapsed();
    BenchResult {
        name: format!("BTreeIndex::remove (n={})", n),
        n,
        total,
    }
}

fn bench_btree_update(n: usize) -> BenchResult {
    let mut tree = BTreeIndex::new("id".to_string(), true);
    for i in 0..n as i64 {
        tree.insert(make_int_key(i), format!("/r/{}", i)).unwrap();
    }

    let start = Instant::now();
    for i in 0..n as i64 {
        let old_key = make_int_key(i);
        let new_key = make_int_key(i + 1_000_000);
        let path = format!("/r/{}", i);
        tree.update(&old_key, new_key, path).unwrap();
    }
    let total = start.elapsed();
    BenchResult {
        name: format!("BTreeIndex::update (n={})", n),
        n,
        total,
    }
}

// ─── IndexManager (memory + disk) Benchmarks ────────────────────────────

async fn bench_manager_insert(n: usize) -> BenchResult {
    let dir = std::env::temp_dir().join(format!("rrdb_bench_insert_{}_{}", n, std::process::id()));
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    let manager = IndexManager::new(dir.clone());
    let meta = make_meta("bench_idx", "id", false);
    manager.create_index(meta).await.unwrap();

    let start = Instant::now();
    for i in 0..n as i64 {
        let key = make_int_key(i);
        let path = format!("/r/{}", i);
        manager.insert("bench_idx", key, path).await.unwrap();
    }
    let total = start.elapsed();

    let _ = tokio::fs::remove_dir_all(&dir).await;
    BenchResult {
        name: format!("IndexManager::insert+flush (n={})", n),
        n,
        total,
    }
}

async fn bench_manager_get(manager: &IndexManager, n: usize) -> BenchResult {
    let start = Instant::now();
    let mut found = 0;
    for i in 0..n as i64 {
        let key = make_int_key(i);
        if !manager.get("bench_idx", &key).await.unwrap().is_empty() {
            found += 1;
        }
    }
    let total = start.elapsed();
    assert_eq!(found, n, "manager get should find all keys");
    BenchResult {
        name: format!("IndexManager::get (exact lookup, n={})", n),
        n,
        total,
    }
}

async fn bench_manager_range(manager: &IndexManager, n: usize, range_size: usize) -> BenchResult {
    let half = (n / 2) as i64;
    let start_key = make_int_key(half);
    let end_key = make_int_key(half + range_size as i64);
    let iterations = 1000;
    let start = Instant::now();
    let mut total_entries = 0;
    for _ in 0..iterations {
        let results = manager
            .range("bench_idx", Some(&start_key), Some(&end_key))
            .await
            .unwrap();
        total_entries += results.len();
    }
    let total = start.elapsed();
    assert!(total_entries > 0, "manager range should find entries");
    BenchResult {
        name: format!(
            "IndexManager::range ({} entries, {}x)",
            range_size, iterations
        ),
        n: iterations,
        total,
    }
}

async fn bench_manager_scan_all(manager: &IndexManager, n: usize) -> BenchResult {
    let iterations = 50;
    let start = Instant::now();
    let mut total_entries = 0;
    for _ in 0..iterations {
        let entries = manager.scan_all("bench_idx").await.unwrap();
        total_entries += entries.len();
    }
    let total = start.elapsed();
    assert_eq!(
        total_entries,
        n * iterations,
        "manager scan_all should return all"
    );
    BenchResult {
        name: format!("IndexManager::scan_all (n={}, {}x)", n, iterations),
        n: iterations,
        total,
    }
}

async fn bench_manager_persist_and_reload(n: usize) -> BenchResult {
    let dir = std::env::temp_dir().join(format!("rrdb_bench_reload_{}_{}", n, std::process::id()));
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.unwrap();

    // Timed: full persist + reload cycle (create, insert, reload from disk)
    let start = Instant::now();
    {
        // Phase 1: create and populate
        let manager = IndexManager::new(dir.clone());
        let meta = make_meta("bench_idx", "id", false);
        manager.create_index(meta).await.unwrap();
        for i in 0..n as i64 {
            let key = make_int_key(i);
            let path = format!("/r/{}", i);
            manager.insert("bench_idx", key, path).await.unwrap();
        }

        // Phase 2: reload from disk
        let manager2 = IndexManager::new(dir.clone());
        let db_dir = dir
            .join("benchdb")
            .join("tables")
            .join("benchtable")
            .join("index");
        manager2.load_all(&db_dir).await.unwrap();
        assert_eq!(
            manager2.len("bench_idx").await.unwrap(),
            n,
            "reload should restore all entries"
        );
    }
    let total = start.elapsed();

    let _ = tokio::fs::remove_dir_all(&dir).await;
    BenchResult {
        name: format!("IndexManager::persist+reload (n={})", n),
        n,
        total,
    }
}

// ─── Main ───────────────────────────────────────────────────────────────

fn main() {
    println!();
    println!("==============================================================");
    println!("    B-tree Index System Benchmarks (PR #191 / Issue #160)");
    println!("==============================================================");
    println!();

    let scales: &[usize] = &[1_000, 10_000, 50_000, 100_000];

    // ── BTreeIndex (in-memory) ──
    println!("--- BTreeIndex (in-memory only) ---");
    println!();
    for &n in scales {
        // Build a shared tree for read benchmarks
        let mut tree = BTreeIndex::new("id".to_string(), false);
        for i in 0..n as i64 {
            tree.insert(make_int_key(i), format!("/r/{}", i)).unwrap();
        }

        bench_btree_insert(n).print();
        bench_btree_get(&tree, n).print();
        bench_btree_get_miss(&tree, n).print();
        bench_btree_range(&tree, n, 100).print();
        bench_btree_range(&tree, n, 1_000).print();
        bench_btree_scan_all(&tree, n).print();
        // Remove and update are destructive, use separate trees
        bench_btree_remove(n).print();
        bench_btree_update(n).print();
        println!();
    }

    // ── IndexManager (memory + disk) ──
    println!("--- IndexManager (memory + disk dual-write) ---");
    println!();

    // IndexManager disk writes are O(n^2) due to full-file rewrite on every mutation.
    // 100K would take ~7 minutes. Cap at 50K for disk benchmarks.
    let disk_scales: &[usize] = &[1_000, 10_000, 50_000];

    let rt = tokio::runtime::Runtime::new().unwrap();

    for &n in disk_scales {
        rt.block_on(async {
            // Insert benchmark (creates its own dir)
            bench_manager_insert(n).await.print();

            // Set up a persistent manager for read benchmarks
            let dir =
                std::env::temp_dir().join(format!("rrdb_bench_read_{}_{}", n, std::process::id()));
            let _ = tokio::fs::remove_dir_all(&dir).await;
            tokio::fs::create_dir_all(&dir).await.unwrap();

            let manager = IndexManager::new(dir.clone());
            let meta = make_meta("bench_idx", "id", false);
            manager.create_index(meta).await.unwrap();
            for i in 0..n as i64 {
                let key = make_int_key(i);
                let path = format!("/r/{}", i);
                manager.insert("bench_idx", key, path).await.unwrap();
            }

            bench_manager_get(&manager, n).await.print();
            bench_manager_range(&manager, n, 100).await.print();
            bench_manager_range(&manager, n, 1_000).await.print();
            bench_manager_scan_all(&manager, n).await.print();
            bench_manager_persist_and_reload(n).await.print();

            let _ = tokio::fs::remove_dir_all(&dir).await;
            println!();
        });
    }

    // ── Summary Table ──
    println!("--- Summary: Insert Scaling ---");
    println!();
    println!(
        "  {:<30} {:>10} {:>12} {:>16}",
        "Operation", "Entries", "Total Time", "Ops/sec"
    );
    println!("  {}", "-".repeat(72));

    for &n in scales {
        let mut tree = BTreeIndex::new("id".to_string(), false);
        let start = Instant::now();
        for i in 0..n as i64 {
            tree.insert(make_int_key(i), format!("/r/{}", i)).unwrap();
        }
        let d = start.elapsed();
        println!(
            "  {:<30} {:>10} {:>12} {:>16}",
            "BTreeIndex::insert",
            n,
            fmt_duration(d),
            (n as f64 / d.as_secs_f64()).floor() as u64,
        );
    }
    println!();
    println!("Benchmarks complete.");
    println!();
}
