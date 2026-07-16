use std::sync::atomic::AtomicU64;

pub mod db;

#[derive(Clone)]
struct WriteEntry {
    key: String,
    value: String,
}

#[tokio::main]
async fn main() {
    // parse args
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <db_type>", args[0]);
        eprintln!(
            "db_type: postgres, rrdb, mysql, mariadb, mongodb, scylla, cassandra, influxdb_v2, influxdb_v3, timescaledb, couchdb, yugabytedb, cockroachdb, clickhouse, elasticsearch, opensearch, etcd, nats, ydb, tidb, tikv, barus, fake"
        );
        std::process::exit(1);
    }

    let db_arg = &args[1];
    println!("Using database: {}", db_arg);

    let db = db::new_database(db_arg)
        .await
        .expect("Failed to create database");

    db.ping().await.expect("Failed to ping database");

    db.setup().await.expect("Failed to setup database");

    let csv_text = std::fs::read_to_string("dataset.csv").unwrap();

    let worker_count = db.worker_count();

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<WriteEntry>(worker_count);

    // producer
    tokio::spawn(async move {
        for (i, line) in csv_text.lines().enumerate() {
            if i >= 1000000 {
                break;
            }

            if i % 10000 == 0 {
                println!("Writing {} lines", i);
            }

            let mut parts = line.splitn(2, ',');
            let key = parts.next().unwrap();
            let value = parts.next().unwrap();

            let entry = WriteEntry {
                key: key.to_string(),
                value: value.to_string(),
            };

            sender.send(entry).await.expect("Failed to send entry");
        }
    });

    let retry_count = 10;
    let retry_delay_ms = 100;

    let _fail_count = std::sync::Arc::new(AtomicU64::new(0));
    let _success_count = std::sync::Arc::new(AtomicU64::new(0));
    let _max_latency_ms = std::sync::Arc::new(AtomicU64::new(0));
    let _min_latency_ms = std::sync::Arc::new(AtomicU64::new(u64::MAX));
    let _total_latency_ms = std::sync::Arc::new(AtomicU64::new(0));

    // consumer
    let fail_count = _fail_count.clone();
    let success_count = _success_count.clone();
    let max_latency_ms = _max_latency_ms.clone();
    let min_latency_ms = _min_latency_ms.clone();
    let total_latency_ms = _total_latency_ms.clone();

    let start = std::time::Instant::now();

    tokio::spawn(async move {
        let request_count = std::sync::Arc::new(AtomicU64::new(0));
        let done_count = std::sync::Arc::new(AtomicU64::new(0));
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(worker_count));

        while let Some(entry) = receiver.recv().await {
            // recv 후 바로 세마포어 획득 - 이 지점에서 블록됨
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            let db = db.clone();
            let done_count = done_count.clone();
            let fail_count = fail_count.clone();
            let max_latency_ms = max_latency_ms.clone();
            let min_latency_ms = min_latency_ms.clone();
            let total_latency_ms = total_latency_ms.clone();
            let success_count = success_count.clone();

            tokio::spawn(async move {
                // permit을 spawn 내부로 이동
                let _permit = permit;

                for _ in 0..retry_count {
                    let write_start = std::time::Instant::now();

                    match db.write(&entry.key, &entry.value).await {
                        Ok(_) => {
                            let write_duration = write_start.elapsed();
                            max_latency_ms.fetch_max(
                                write_duration.as_millis() as u64,
                                std::sync::atomic::Ordering::SeqCst,
                            );
                            min_latency_ms.fetch_min(
                                write_duration.as_millis() as u64,
                                std::sync::atomic::Ordering::SeqCst,
                            );
                            total_latency_ms.fetch_add(
                                write_duration.as_millis() as u64,
                                std::sync::atomic::Ordering::SeqCst,
                            );
                            success_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                            break;
                        }
                        Err(_e) => {
                            // eprintln!("Write error: {:?}, retrying...", e);
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms))
                                .await;
                            fail_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                }

                done_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // _permit이 drop되면서 자동으로 세마포어 해제
            });
        }
        let request_count = request_count.load(std::sync::atomic::Ordering::SeqCst);
        println!("@ All requests sent: {}", request_count);

        // 모든 작업이 끝날 때까지 대기
        while done_count.load(std::sync::atomic::Ordering::SeqCst) < request_count {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();

    let duration = start.elapsed();

    println!("@ All writes completed in {:?}", duration);
    println!(
        "@ Fail count: {}, Success count: {}",
        _fail_count.load(std::sync::atomic::Ordering::SeqCst),
        _success_count.load(std::sync::atomic::Ordering::SeqCst)
    );
    let total = _total_latency_ms.load(std::sync::atomic::Ordering::SeqCst);
    let max = _max_latency_ms.load(std::sync::atomic::Ordering::SeqCst);
    let min = _min_latency_ms.load(std::sync::atomic::Ordering::SeqCst);
    let success = _success_count.load(std::sync::atomic::Ordering::SeqCst);
    let avg = total as f64 / success as f64;
    let tps = success as f64 / duration.as_secs_f64();

    println!("@ Max latency: {} ms", max);
    println!("@ Min latency: {} ms", min);
    println!("@ Avg latency: {:.2} ms", avg);
    println!("@ Throughput: {:.2} writes/sec(TPS)", tps);
}
