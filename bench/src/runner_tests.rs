use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

struct Fake {
    fault: &'static str,
    statements: Mutex<Vec<String>>,
    active: AtomicUsize,
    peak: AtomicUsize,
    written: AtomicUsize,
}

impl Fake {
    fn new(fault: &'static str) -> Arc<Self> {
        Arc::new(Self {
            fault,
            statements: Mutex::new(vec![]),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            written: AtomicUsize::new(0),
        })
    }
}

struct Active<'a>(&'a AtomicUsize);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl db::Adapter for Fake {
    async fn execute(&self, sql: &str) -> std::result::Result<(), &'static str> {
        self.statements.lock().unwrap().push(sql.to_owned());
        if sql.starts_with("CREATE") {
            if self.fault == "create" {
                return Err("create failed");
            }
            if self.fault == "create-timeout" {
                std::future::pending::<()>().await;
            }
        }
        if sql.starts_with("DROP") {
            assert_eq!(
                self.active.load(Ordering::SeqCst),
                0,
                "cleanup before tasks joined"
            );
            if self.fault == "drop" {
                return Err("drop failed");
            }
            if self.fault == "drop-timeout" {
                std::future::pending::<()>().await;
            }
        }
        if sql.starts_with("INSERT") {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            let _guard = Active(&self.active);
            self.peak.fetch_max(active, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(1)).await;
            if sql.contains("VALUES (0,") {
                match self.fault {
                    "write" => return Err("write failed"),
                    "panic" => panic!("simulated worker panic"),
                    "write-timeout" => std::future::pending::<()>().await,
                    _ => {}
                }
            }
            self.written.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    async fn count(&self, sql: &str) -> std::result::Result<usize, &'static str> {
        self.statements.lock().unwrap().push(sql.to_owned());
        match self.fault {
            "read" => Err("read failed"),
            "read-timeout" => std::future::pending().await,
            "mismatch" => Ok(0),
            _ => Ok(self.written.load(Ordering::SeqCst)),
        }
    }
}

fn config(backend: &str) -> Config {
    Config::parse(
        [
            backend,
            "--rows",
            "12",
            "--workers",
            "3",
            "--output",
            "result.json",
        ]
        .map(str::to_owned)
        .to_vec(),
    )
    .unwrap()
}

async fn simulated(backend: &str, fake: Arc<Fake>) -> std::result::Result<Value, &'static str> {
    tokio::time::timeout(
        Duration::from_secs(2),
        run_benchmark(&config(backend), fake, Duration::from_millis(20)),
    )
    .await
    .expect("benchmark hung")
}

#[tokio::test]
async fn backends_share_exact_workload_and_bounded_joined_workers() {
    let mut workloads = vec![];
    let mut tables = vec![];
    for backend in ["rrdb", "postgres"] {
        let fake = Fake::new("");
        let result = simulated(backend, fake.clone()).await.unwrap();
        assert_eq!(result["backend"], backend);
        assert_eq!(result["observed_rows"], 12);
        assert_eq!(fake.active.load(Ordering::SeqCst), 0);
        assert!(fake.peak.load(Ordering::SeqCst) <= 3);
        assert!(fake.peak.load(Ordering::SeqCst) > 1);
        let statements = fake.statements.lock().unwrap();
        let table = statements[0].split_whitespace().nth(2).unwrap().to_string();
        assert_eq!(table.len(), 38);
        assert!(table.starts_with("bench_"));
        assert!(
            table
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        );
        assert_eq!(
            statements[0],
            format!("CREATE TABLE {table} (key INTEGER PRIMARY KEY, value VARCHAR(128))")
        );
        assert_eq!(
            statements[statements.len() - 2],
            format!("SELECT COUNT(1) FROM {table}")
        );
        assert_eq!(statements.last().unwrap(), &format!("DROP TABLE {table}"));
        let mut writes: Vec<_> = statements
            .iter()
            .filter(|s| s.starts_with("INSERT"))
            .map(|s| s.replace(&table, "TABLE"))
            .collect();
        writes.sort();
        assert_eq!(writes.len(), 12);
        for index in 0..12 {
            assert!(writes.contains(&format!(
                "INSERT INTO TABLE (key, value) VALUES ({index}, '{}')",
                format!("{index:08x}").repeat(16)
            )));
        }
        tables.push(table);
        workloads.push(writes);
    }
    assert_ne!(tables[0], tables[1]);
    assert_eq!(workloads[0], workloads[1]);
}

#[tokio::test]
async fn create_failure_or_timeout_never_drops_an_unowned_table() {
    for fault in ["create", "create-timeout"] {
        let fake = Fake::new(fault);
        assert!(simulated("rrdb", fake.clone()).await.is_err());
        let statements = fake.statements.lock().unwrap();
        assert_eq!(statements.len(), 1);
        assert!(statements[0].starts_with("CREATE TABLE bench_"));
        assert!(!statements[0].contains("IF NOT EXISTS"));
    }
}

#[tokio::test]
async fn write_errors_panics_and_timeouts_fail_closed_without_retries() {
    for fault in ["write", "panic", "write-timeout"] {
        let fake = Fake::new(fault);
        assert!(simulated("rrdb", fake.clone()).await.is_err(), "{fault}");
        assert_eq!(fake.active.load(Ordering::SeqCst), 0);
        let statements = fake.statements.lock().unwrap();
        let writes: Vec<_> = statements
            .iter()
            .filter(|s| s.starts_with("INSERT"))
            .collect();
        assert_eq!(
            writes.iter().filter(|s| s.contains("VALUES (0,")).count(),
            1
        );
        assert_eq!(
            writes
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            writes.len()
        );
        assert!(!statements.iter().any(|s| s.starts_with("SELECT")));
        assert_eq!(
            statements.iter().filter(|s| s.starts_with("DROP")).count(),
            1
        );
    }
}

#[tokio::test]
async fn readback_mismatch_errors_timeouts_and_cleanup_failures_fail_closed() {
    for fault in ["mismatch", "read", "read-timeout", "drop", "drop-timeout"] {
        let fake = Fake::new(fault);
        assert!(
            simulated("postgres", fake.clone()).await.is_err(),
            "{fault}"
        );
        assert_eq!(fake.written.load(Ordering::SeqCst), 12);
        let statements = fake.statements.lock().unwrap();
        assert_eq!(
            statements
                .iter()
                .filter(|s| s.starts_with("SELECT"))
                .count(),
            1
        );
        assert_eq!(
            statements.iter().filter(|s| s.starts_with("DROP")).count(),
            1
        );
    }
}
