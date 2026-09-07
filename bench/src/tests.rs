use super::*;

fn config(args: &[&str]) -> std::result::Result<Config, &'static str> {
    Config::parse(args.iter().map(|arg| arg.to_string()).collect())
}

#[test]
fn cli_validates_backend_defaults_and_bounds() {
    for backend in ["rrdb", "postgres"] {
        let parsed = config(&[backend, "--output", "result.json"]).unwrap();
        assert_eq!(parsed.backend, backend);
        assert_eq!((parsed.rows, parsed.workers), (1000, 4));
        assert_eq!(parsed.output, std::path::PathBuf::from("result.json"));
    }
    let parsed = config(&[
        "postgres",
        "--workers",
        "64",
        "--rows",
        "1000000",
        "--output",
        "r.json",
    ])
    .unwrap();
    assert_eq!((parsed.rows, parsed.workers), (1_000_000, 64));
    assert!(
        config(&[
            "rrdb",
            "--rows",
            "1",
            "--workers",
            "1",
            "--output",
            "r.json"
        ])
        .is_ok()
    );
    for args in [
        vec![],
        vec!["mysql"],
        vec!["rrdb"],
        vec!["rrdb", "--output", ""],
        vec!["rrdb", "--output", "r.json", "--rows", "0"],
        vec!["rrdb", "--output", "r.json", "--rows", "1000001"],
        vec!["rrdb", "--output", "r.json", "--rows", "-1"],
        vec!["rrdb", "--output", "r.json", "--rows", "1.5"],
        vec![
            "rrdb",
            "--output",
            "r.json",
            "--rows",
            "999999999999999999999",
        ],
        vec!["rrdb", "--output", "r.json", "--workers", "0"],
        vec!["rrdb", "--output", "r.json", "--workers", "65"],
        vec!["rrdb", "--output", "r.json", "--rows", "3"],
        vec!["rrdb", "--output", "r.json", "--workers"],
        vec!["rrdb", "--output", "r.json", "--rows", "4", "--rows", "5"],
        vec!["rrdb", "--output", "r.json", "--url", "sensitive-input"],
        vec!["sensitive-input", "--output", "r.json"],
    ] {
        let error = config(&args).unwrap_err();
        assert!(!error.contains("sensitive-input"));
    }
}

#[test]
fn generated_workload_is_repeatable_alphanumeric_and_indexed() {
    let first = entry(0);
    assert_eq!(first.0, 0);
    assert_eq!(first.1, "00000000".repeat(16));
    assert_eq!(entry(15).1, "0000000f".repeat(16));
    for index in [0, 1, 15, 999, 999_999] {
        let (key, value) = entry(index);
        assert_eq!(key, index);
        assert_eq!(entry(index), (key, value.clone()));
        assert_eq!(value.len(), 128);
        assert!(value.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    }
    assert_ne!(entry(0), entry(1));
}

#[test]
fn metrics_keep_submillisecond_precision_and_nearest_rank_percentiles() {
    let cfg = config(&["postgres", "--rows", "4", "--output", "r.json"]).unwrap();
    let samples = [900, 100, 500, 300].map(std::time::Duration::from_micros);
    let result = metrics(&cfg, &samples, std::time::Duration::from_micros(2000), 4).unwrap();
    assert_eq!(result["backend"], "postgres");
    assert_eq!(result["successful_writes"], 4);
    assert_eq!(result["failed_writes"], 0);
    assert_eq!(result["observed_rows"], 4);
    assert_eq!(result["rows"], 4);
    assert_eq!(result["workers"], 4);
    assert_eq!(result["protocol"], "postgresql-simple-query");
    assert_eq!(result["elapsed_seconds"], 0.002);
    assert_eq!(result["throughput_writes_per_second"], 2000.0);
    for (key, value) in [
        ("min", 0.1),
        ("mean", 0.45),
        ("p50", 0.3),
        ("p95", 0.9),
        ("max", 0.9),
    ] {
        assert!((result["latency_ms"][key].as_f64().unwrap() - value).abs() < 1e-12);
    }
    assert!(metrics(&cfg, &[], std::time::Duration::from_secs(1), 0).is_err());
    assert!(metrics(&cfg, &samples, std::time::Duration::ZERO, 4).is_err());
    assert!(metrics(&cfg, &samples, std::time::Duration::from_secs(1), 3).is_err());
    assert!(metrics(&cfg, &samples[..3], std::time::Duration::from_secs(1), 4).is_err());
}
