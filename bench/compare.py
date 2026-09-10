"""Run identical write workloads against disposable RRDB and PostgreSQL instances."""

import argparse
import json
import math
import os
import platform
import signal
import socket
import subprocess
import tempfile
import time
import uuid
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def bounded_integer(maximum):
    def parse(value):
        number = int(value)
        if not 1 <= number <= maximum:
            raise argparse.ArgumentTypeError(f"must be between 1 and {maximum}")
        return number

    return parse


def arguments(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=bounded_integer(1_000_000), default=1000)
    parser.add_argument("--workers", type=bounded_integer(64), default=4)
    parser.add_argument("--repeats", type=bounded_integer(10), default=3)
    parser.add_argument(
        "--output", type=Path, default=ROOT / "bench/results" / uuid.uuid4().hex
    )
    parser.add_argument("--rrdb-bin", type=Path, default=ROOT / "target/release/rrdb")
    parser.add_argument(
        "--bench-bin", type=Path, default=ROOT / "bench/target/release/main"
    )
    parser.add_argument("--postgres-image", default="postgres:16-alpine")
    options = parser.parse_args(argv)
    if options.workers > options.rows:
        parser.error("workers cannot exceed rows")
    return options


def command(args, **kwargs):
    return subprocess.check_output(args, text=True, timeout=30, **kwargs).strip()


@contextmanager
def managed_process(args, log_path, env=None):
    # These are direct native binaries, not shells spawning unowned descendants.
    with log_path.open("w") as log:
        process = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, env=env)
        try:
            yield process
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
            process.wait(timeout=5)


def run_logged(args, log_path, timeout, env=None):
    with managed_process(args, log_path, env) as process:
        status = process.wait(timeout=timeout)
        if status:
            raise subprocess.CalledProcessError(status, args)


def wait_for_port(process, port, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("RRDB exited before becoming ready; see rrdb.log")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                return
        except OSError:
            time.sleep(0.1)
    raise TimeoutError("RRDB readiness timed out")


def available_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


@contextmanager
def postgres(image, output):
    # Only the disposable local container uses trust auth; no published remote endpoint.
    owner = uuid.uuid4().hex
    name = f"rrdb-bench-{owner}"
    label = f"rrdb.benchmark.run={owner}"
    container = None
    try:
        container = command(
            [
                "docker",
                "create",
                "--name",
                name,
                "--label",
                label,
                "--publish",
                "127.0.0.1::5432",
                "--env",
                "POSTGRES_HOST_AUTH_METHOD=trust",
                "--env",
                "POSTGRES_USER=rrdb",
                "--env",
                "POSTGRES_DB=rrdb",
                image,
            ]
        )
        command(["docker", "start", container])
        deadline = time.monotonic() + 60
        while True:
            ready = subprocess.run(
                [
                    "docker",
                    "exec",
                    container,
                    "pg_isready",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "rrdb",
                    "-d",
                    "rrdb",
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=10,
                check=False,
            )
            if ready.returncode == 0:
                break
            if time.monotonic() >= deadline:
                raise TimeoutError("PostgreSQL readiness timed out")
            time.sleep(0.2)
        mapping = command(["docker", "port", container, "5432/tcp"])
        port = int(mapping.rsplit(":", 1)[1])
        metadata = {
            "image": image,
            "image_id": command(
                ["docker", "inspect", "--format", "{{.Image}}", container]
            ),
            "version": command(["docker", "exec", container, "postgres", "--version"]),
        }
        yield f"postgres://rrdb@127.0.0.1:{port}/rrdb", metadata
    finally:
        # CREATE may have reached Docker even when its CLI response was lost.
        # Match both name and run label; never remove a foreign name collision.
        if not container:
            container = command(
                [
                    "docker",
                    "ps",
                    "--all",
                    "--quiet",
                    "--no-trunc",
                    "--filter",
                    f"name=^/{name}$",
                    "--filter",
                    f"label={label}",
                ]
            )
        if container:
            try:
                with (output / "postgres.log").open("w") as log:
                    subprocess.run(
                        ["docker", "logs", container],
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=10,
                        check=False,
                    )
            finally:
                command(["docker", "rm", "--force", "--volumes", container])


def validate_result(result, backend, rows, workers):
    expected = {
        "backend": backend,
        "rows": rows,
        "workers": workers,
        "successful_writes": rows,
        "failed_writes": 0,
        "observed_rows": rows,
    }
    for field, value in expected.items():
        if result.get(field) != value:
            raise ValueError(f"{backend}: result mismatch for {field}")
    for field in ("elapsed_seconds", "throughput_writes_per_second"):
        value = result[field]
        if (
            not isinstance(value, (int, float))
            or not math.isfinite(value)
            or value <= 0
        ):
            raise ValueError(f"{backend}: invalid metric {field}")


def compare(options):
    rrdb_bin = options.rrdb_bin.resolve(strict=True)
    bench_bin = options.bench_bin.resolve(strict=True)
    output = options.output.resolve()
    # Refuse an existing directory, so stale results can never pass a new run.
    output.mkdir(parents=True, exist_ok=False)
    provenance = {
        "started_at": datetime.now(timezone.utc).isoformat(),
        "revision": command(["git", "-C", str(ROOT), "rev-parse", "HEAD"]),
        "dirty_tree": bool(command(["git", "-C", str(ROOT), "status", "--porcelain"])),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "logical_cpus": os.cpu_count(),
        "rows": options.rows,
        "workers": options.workers,
        "repeats": options.repeats,
        "rustc": command(["rustc", "--version"]),
        "note": "RRDB native; PostgreSQL Docker (VM on macOS). Not a durability-equivalent comparison.",
    }
    (output / "environment.json").write_text(json.dumps(provenance, indent=2) + "\n")
    results = []
    with tempfile.TemporaryDirectory(prefix="rrdb-bench-") as tmp:
        base = Path(tmp)
        run_logged(
            [str(rrdb_bin), "init", "--base-path", str(base)],
            output / "init.log",
            timeout=30,
        )
        port = available_port()
        # init creates rrdb database/catalog; customize only this owned temporary config.
        config = (
            f'host = "127.0.0.1"\nport = {port}\n'
            f"data_directory = {json.dumps(str(base / 'data'))}\n"
            f"wal_directory = {json.dumps(str(base / 'wal'))}\n"
            'wal_enabled = true\nwal_segment_size = 16777216\nwal_extension = "log"\n'
        )
        (base / "rrdb.config").write_text(config)
        with managed_process(
            [str(rrdb_bin), "run", "--base-path", str(base)],
            output / "rrdb.log",
            {**os.environ, "RUST_LOG": "warn"},
        ) as server:
            wait_for_port(server, port)
            with postgres(options.postgres_image, output) as (pg_url, pg_metadata):
                provenance["postgres"] = pg_metadata
                (output / "environment.json").write_text(
                    json.dumps(provenance, indent=2) + "\n"
                )
                urls = {
                    "rrdb": f"postgres://rrdb@127.0.0.1:{port}/rrdb",
                    "postgres": pg_url,
                }
                for trial in range(options.repeats):
                    order = (
                        ("rrdb", "postgres") if trial % 2 == 0 else ("postgres", "rrdb")
                    )
                    for backend in order:
                        report = output / f"{trial + 1}-{backend}.json"
                        run_logged(
                            [
                                str(bench_bin),
                                backend,
                                "--rows",
                                str(options.rows),
                                "--workers",
                                str(options.workers),
                                "--output",
                                str(report),
                            ],
                            output / f"{trial + 1}-{backend}.log",
                            timeout=600,
                            env={**os.environ, "BENCH_DATABASE_URL": urls[backend]},
                        )
                        result = json.loads(report.read_text())
                        validate_result(result, backend, options.rows, options.workers)
                        results.append({"trial": trial + 1, **result})
    (output / "comparison.json").write_text(
        json.dumps({"environment": provenance, "results": results}, indent=2) + "\n"
    )
    print(f"Verified {len(results)} benchmark runs; results: {output}")


def main():
    # Turn cancellation into normal context-manager unwinding and bounded cleanup.
    def interrupted(signum, _frame):
        signal.signal(signum, signal.SIG_IGN)
        raise SystemExit(128 + signum)

    for signum in (signal.SIGTERM, signal.SIGHUP):
        if signal.getsignal(signum) != signal.SIG_IGN:
            signal.signal(signum, interrupted)
    compare(arguments())


if __name__ == "__main__":
    main()
