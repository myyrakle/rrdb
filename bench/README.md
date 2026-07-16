# Databases write performance benchmark

- This is a write throughput load test for each database.
- We continuously insert 10 million records and measure how long it takes, latency, and other metrics.

## Test Environment

- OS: Linux (Arch Linux)
- CPU: Ryzen 9 7900 (docker limit 4 core)
- RAM: docker limit 8 GB
- Disk: SSD - SK hynix Gold P31 M.2 NVMe 2280

## Benchmark Table

| DB             | Duration | TPS    | Avegate Latency | Min Latency | Max Latency | Disk Usage |
| -------------- | -------- | ------ | --------------- | ----------- | ----------- | ---------- |
| PostgreSQL     | 526 s    | 18861  | 526 ms          | 7 ms        | 2483 ms     | 3.7 GB     |
| MySQL          | 3228 s   | 3143   | 3143 ms         | 53 ms       | 180705 ms   | 7.6 GB     |
| MariaDB        | 2194 s   | 4751   | 2098 ms         | 3 ms        | 13098 ms    | 3.8 GB     |
| MongoDB        | 376 s    | 26524  | 370 ms          | 201 ms      | 1288 ms     | 3.1 GB     |
| CassandraDB    | 158 s    | 63130  | 156 ms          | 78 ms       | 1388 ms     | 2.3 GB     |
| ScyllaDB       | 172 s    | 57848  | 156 ms          | 1 ms        | 1098 ms     | 6.2 GB     |
| InfluxDB (v2)  | 1428 s   | 7001   | 1426 ms         | 1 ms        | 3484ms      | 1.6 GB     |
| TimescaleDB    | 978 s    | 10224  | 976 ms          | 348 ms      | 30283 ms    | 12 GB      |
| CouchDB        | 3800 s   | 2631   | 189 ms          | 3 ms        | 2307 ms     | 28 GB      |
| YugabyteDB     | 3179 s   | 3145   | 3177 ms         | 623 ms      | 7064 ms     | 2 GB       |
| CockroachDB    | 3919 s   | 2551   | 3917 ms         | 156 ms      | 16015 ms    | 3.1 GB     |
| etcd           | 1367 s   | 7310   | 1366 ms         | 2 ms        | 2394 ms     | 2.6 GB     |
| nats Jetstream | 83 s     | 120415 | 74 ms           | 7 ms        | 312 ms      | 1.9 GB     |
| TiDB           | 562 s    | 17788  | 561 ms          | 2 ms        | 1626 ms     | 3.1 GB     |
| TiKV           | 1235 s   | 8093   | 1234 ms         | 4 ms        | 2586 ms     | 24 GB      |
| Clickhouse     | 1666 s   | 2032   | 306 ms          | 8 ms        | 9053 ms     | 2.4 GB     |
| Elasticsearch  | 525 s    | 19037  | 47 ms           | 2 ms        | 4295 ms     | 5.2 GB     |
| Opensearch     | 642 s    | 15556  | 59 ms           | 1 ms        | 4079 ms     | 5.3 GB     |
