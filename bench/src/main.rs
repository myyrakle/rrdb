use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;

mod db;
#[cfg(test)]
mod runner_tests;
#[cfg(test)]
mod tests;

// Errors are fixed categories: never expose server messages, input or connection URLs.
type Result<T> = std::result::Result<T, &'static str>;
const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct Config {
    backend: String,
    rows: usize,
    workers: usize,
    output: PathBuf,
}

impl Config {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut args = args.into_iter();
        let backend = args.next().ok_or("backend required: rrdb or postgres")?;
        if !matches!(backend.as_str(), "rrdb" | "postgres") {
            return Err("backend must be rrdb or postgres");
        }
        let (mut rows, mut workers, mut output) = (None, None, None);
        while let Some(flag) = args.next() {
            let value = args.next().ok_or("option value required")?;
            match flag.as_str() {
                "--rows" if rows.is_none() => {
                    rows = Some(value.parse::<usize>().map_err(|_| "invalid rows")?)
                }
                "--workers" if workers.is_none() => {
                    workers = Some(value.parse::<usize>().map_err(|_| "invalid workers")?)
                }
                "--output" if output.is_none() && !value.is_empty() => {
                    output = Some(PathBuf::from(value))
                }
                _ => return Err("unknown, duplicate, or invalid option"),
            }
        }
        let (rows, workers) = (rows.unwrap_or(1000), workers.unwrap_or(4));
        if !(1..=1_000_000).contains(&rows) || !(1..=64).contains(&workers) || workers > rows {
            return Err("require rows 1..=1000000 and workers 1..=64, workers <= rows");
        }
        Ok(Self {
            backend,
            rows,
            workers,
            output: output.ok_or("--output PATH required")?,
        })
    }
}

fn entry(index: usize) -> (usize, String) {
    (index, format!("{index:08x}").repeat(16))
}

async fn bounded<T>(
    limit: Duration,
    future: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout(limit, future)
        .await
        .map_err(|_| "database operation timed out")?
}

async fn run_benchmark(
    config: &Config,
    db: Arc<dyn db::Adapter>,
    limit: Duration,
) -> Result<Value> {
    // No IF NOT EXISTS: only an acknowledged CREATE grants permission to DROP.
    let table = format!("bench_{}", uuid::Uuid::new_v4().simple());
    bounded(
        limit,
        db.execute(&format!(
            "CREATE TABLE {table} (key INTEGER PRIMARY KEY, value VARCHAR(128))"
        )),
    )
    .await?;
    let measured = measure(config, db.clone(), &table, limit).await;
    let cleanup = bounded(limit, db.execute(&format!("DROP TABLE {table}"))).await;
    let report = measured?;
    cleanup?;
    Ok(report)
}

async fn measure(
    config: &Config,
    db: Arc<dyn db::Adapter>,
    table: &str,
    limit: Duration,
) -> Result<Value> {
    let mut tasks = JoinSet::new();
    let mut samples = Vec::with_capacity(config.rows);
    let (mut next, mut failed) = (0, false);
    let start = Instant::now();
    loop {
        while !failed && next < config.rows && tasks.len() < config.workers {
            let (key, value) = entry(next);
            // Only generated integer keys, ASCII hex values and a UUID identifier reach SQL.
            let sql = format!("INSERT INTO {table} (key, value) VALUES ({key}, '{value}')");
            let db = db.clone();
            tasks.spawn(async move {
                let start = Instant::now();
                bounded(limit, db.execute(&sql)).await?;
                Ok::<_, &'static str>(start.elapsed())
            });
            next += 1;
        }
        match tasks.join_next().await {
            Some(Ok(Ok(latency))) => samples.push(latency),
            Some(_) => failed = true, // Stop scheduling; drain every in-flight task, including panics.
            None => break,
        }
    }
    let elapsed = start.elapsed();
    if failed {
        return Err("write failed, timed out, or worker panicked; benchmark rejected");
    }
    let observed = bounded(limit, db.count(&format!("SELECT COUNT(1) FROM {table}"))).await?;
    metrics(config, &samples, elapsed, observed)
}

fn metrics(
    config: &Config,
    samples: &[Duration],
    elapsed: Duration,
    observed: usize,
) -> Result<Value> {
    if samples.is_empty()
        || samples.len() != config.rows
        || observed != config.rows
        || elapsed.is_zero()
    {
        return Err("write/readback count mismatch or empty measurement");
    }
    let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let n = ms.len();
    Ok(json!({
        "backend": config.backend, "rows": config.rows, "workers": config.workers,
        "successful_writes": n, "failed_writes": 0, "observed_rows": observed,
        "elapsed_seconds": elapsed.as_secs_f64(),
        "throughput_writes_per_second": n as f64 / elapsed.as_secs_f64(),
        "latency_ms": {"min": ms[0], "mean": ms.iter().sum::<f64>() / n as f64,
            "p50": ms[n.div_ceil(2) - 1], "p95": ms[(n * 95).div_ceil(100) - 1], "max": ms[n - 1]},
        "protocol": "postgresql-simple-query",
        "measurement": "acknowledged single-row inserts; excludes connect, create, count and drop",
        "latency_scope": "per-write client request including pool acquisition; successful writes only",
        "percentile_method": "nearest-rank",
        "schema": "key INTEGER PRIMARY KEY, value VARCHAR(128)", "value_bytes": 128,
        "connect_timeout_seconds": DEADLINE.as_secs(), "query_timeout_seconds": DEADLINE.as_secs()
    }))
}

async fn execute_cli() -> Result<()> {
    let config = Config::parse(std::env::args().skip(1).collect())?;
    let url = std::env::var("BENCH_DATABASE_URL").map_err(|_| "BENCH_DATABASE_URL required")?;
    let db = bounded(DEADLINE, db::Pg::connect(&url, config.workers)).await?;
    let report = run_benchmark(&config, Arc::new(db), DEADLINE).await?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config.output)
        .map_err(|_| "cannot create output file (must not already exist)")?;
    serde_json::to_writer_pretty(file, &report).map_err(|_| "cannot write JSON output")?;
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match execute_cli().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("benchmark: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
