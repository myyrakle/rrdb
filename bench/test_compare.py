import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import compare


class ComparisonTests(unittest.TestCase):
    def test_invalid_options_fail_before_starting_resources(self):
        for args in (
            ["--rows", "0"],
            ["--workers", "0"],
            ["--repeats", "0"],
            ["--rows", "1000001"],
            ["--workers", "65"],
            ["--repeats", "11"],
            ["--rows", "1", "--workers", "2"],
        ):
            with (
                self.subTest(args=args),
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                compare.arguments(args)

    def test_equal_workload_is_required_for_both_backends(self):
        result = {
            "backend": "rrdb",
            "rows": 10,
            "workers": 2,
            "successful_writes": 10,
            "failed_writes": 0,
            "observed_rows": 10,
            "elapsed_seconds": 0.5,
            "throughput_writes_per_second": 20.0,
        }
        compare.validate_result(result, "rrdb", 10, 2)
        for field, value in (
            ("backend", "postgres"),
            ("rows", 9),
            ("workers", 3),
            ("successful_writes", 9),
            ("failed_writes", 1),
            ("observed_rows", 9),
            ("elapsed_seconds", 0),
            ("throughput_writes_per_second", float("nan")),
        ):
            with self.subTest(field=field), self.assertRaises(ValueError):
                compare.validate_result({**result, field: value}, "rrdb", 10, 2)

    def test_managed_process_is_reaped_after_body_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            with (
                self.assertRaisesRegex(RuntimeError, "body failure"),
                compare.managed_process(
                    [sys.executable, "-c", "import time; time.sleep(60)"],
                    Path(tmp) / "child.log",
                ) as process,
            ):
                self.assertIsNone(process.poll())
                raise RuntimeError("body failure")
            self.assertIsNotNone(process.returncode)

    def test_readiness_detects_early_exit(self):
        with subprocess.Popen([sys.executable, "-c", "pass"]) as process:
            process.wait(timeout=5)
            with self.assertRaisesRegex(RuntimeError, "exited"):
                compare.wait_for_port(process, 1, timeout=0.1)

    def test_failed_benchmark_exit_cannot_produce_success(self):
        with (
            tempfile.TemporaryDirectory() as tmp,
            self.assertRaises(subprocess.CalledProcessError),
        ):
            compare.run_logged(
                [sys.executable, "-c", "raise SystemExit(7)"],
                Path(tmp) / "failure.log",
                timeout=5,
            )

    def test_postgres_is_cleaned_when_start_fails(self):
        calls = []

        def command(args, **kwargs):
            calls.append(args)
            if args[:2] == ["docker", "create"]:
                return "owned-container-id"
            if args[:2] == ["docker", "start"]:
                raise RuntimeError("start failure")
            return ""

        with (
            tempfile.TemporaryDirectory() as tmp,
            patch.object(compare, "command", command),
            patch.object(compare.subprocess, "run"),
            self.assertRaisesRegex(RuntimeError, "start failure"),
            compare.postgres("postgres:16-alpine", Path(tmp)),
        ):
            self.fail("must not enter body")
        self.assertIn(
            ["docker", "rm", "--force", "--volumes", "owned-container-id"], calls
        )

    def test_container_create_response_loss_cleans_only_owned_resources(self):
        for failure in (
            subprocess.TimeoutExpired("docker create", 30),
            SystemExit(143),
        ):
            calls = []
            allocated = set()

            def command(
                args, calls=calls, allocated=allocated, failure=failure, **kwargs
            ):
                calls.append(args)
                if args[:2] == ["docker", "create"]:
                    allocated.add("owned-id")
                    raise failure
                if args[:2] == ["docker", "ps"]:
                    create = calls[0]
                    name = create[create.index("--name") + 1]
                    owner = create[create.index("--label") + 1]
                    self.assertIn(f"name=^/{name}$", args)
                    self.assertIn(f"label={owner}", args)
                    return "owned-id"
                if args[:2] == ["docker", "rm"]:
                    allocated.remove(args[-1])
                return ""

            with (
                self.subTest(failure=type(failure).__name__),
                tempfile.TemporaryDirectory() as tmp,
                patch.object(compare, "command", command),
                patch.object(compare.subprocess, "run"),
                self.assertRaises(type(failure)),
                compare.postgres("postgres:16-alpine", Path(tmp)),
            ):
                self.fail("must not enter body")
            self.assertEqual(allocated, set())

    def test_failed_create_does_not_remove_an_unowned_name_collision(self):
        calls = []

        def command(args, **kwargs):
            calls.append(args)
            if args[:2] == ["docker", "create"]:
                raise RuntimeError("name collision")
            if args[:2] == ["docker", "ps"]:
                # The colliding container lacks this run's ownership label.
                return ""
            self.fail(f"unexpected cleanup command: {args[:2]}")

        with (
            tempfile.TemporaryDirectory() as tmp,
            patch.object(compare, "command", command),
            self.assertRaisesRegex(RuntimeError, "name collision"),
            compare.postgres("postgres:16-alpine", Path(tmp)),
        ):
            self.fail("must not enter body")
        self.assertFalse(any(args[:2] == ["docker", "rm"] for args in calls))


if __name__ == "__main__":
    unittest.main()
