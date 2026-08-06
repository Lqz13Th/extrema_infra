#!/usr/bin/env python3
"""Run the co-located Binance public WebSocket channel microbenchmarks."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import subprocess
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / "work" / "benchmarks" / "binance_public_ws_crossbench"
DEFAULT_OUTPUT = ROOT / "outputs" / "binance-public-ws-native-2026-08-05.jsonl"
CHANNELS = (
    "trade_decode_native_normalize",
    "book_ticker_decode_native_normalize",
    "depth_frame_decode_native_normalize",
)
FRAMEWORKS = ("nautilus", "barter", "extrema")


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repeats", type=positive_int, default=3)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--overwrite", action="store_true")
    return parser.parse_args()


def command_output(command: list[str], cwd: Path = ROOT) -> str:
    return subprocess.run(
        command,
        cwd=cwd,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def source_digest() -> str:
    digest = hashlib.sha256()
    paths = (
        PROJECT / "Cargo.toml",
        PROJECT / "Cargo.lock",
        PROJECT / "benches" / "channels.rs",
        ROOT
        / "work"
        / "repos"
        / "extrema_infra"
        / "src"
        / "arch"
        / "market_assets"
        / "exchange"
        / "binance"
        / "benchmark.rs",
    )
    for path in paths:
        digest.update(str(path.relative_to(ROOT)).encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def append(path: Path, record: dict[str, Any]) -> None:
    with path.open("a", encoding="utf-8") as output:
        output.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
        output.write("\n")


def estimates(channel: str, framework: str) -> dict[str, Any]:
    path = (
        PROJECT
        / "target"
        / "criterion"
        / channel
        / framework
        / "new"
        / "estimates.json"
    )
    value = json.loads(path.read_text(encoding="utf-8"))
    median = value["median"]
    mean = value["mean"]
    return {
        "median_ns": float(median["point_estimate"]),
        "median_ci_lower_ns": float(median["confidence_interval"]["lower_bound"]),
        "median_ci_upper_ns": float(median["confidence_interval"]["upper_bound"]),
        "mean_ns": float(mean["point_estimate"]),
    }


def rotated(values: tuple[str, ...], offset: int) -> tuple[str, ...]:
    index = offset % len(values)
    return values[index:] + values[:index]


def main() -> int:
    args = parse_args()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        if not args.overwrite:
            raise FileExistsError(f"output already exists: {output}")
        output.unlink()

    repositories = {}
    for name, relative in (
        ("nautilus", "work/repos/nautilus_trader"),
        ("barter", "work/repos/barter-rs"),
        ("extrema", "work/repos/extrema_infra"),
    ):
        repo = ROOT / relative
        repositories[name] = {
            "commit": command_output(["git", "rev-parse", "HEAD"], repo),
            "dirty": bool(command_output(["git", "status", "--porcelain"], repo)),
        }

    append(
        output,
        {
            "record_type": "metadata",
            "schema_version": 1,
            "created_at_epoch_seconds": time.time(),
            "host": platform.node(),
            "platform": platform.platform(),
            "rustc": command_output(["rustc", "--version"]),
            "cargo": command_output(["cargo", "--version"]),
            "criterion": "0.8.2",
            "repeats": args.repeats,
            "source_sha256": source_digest(),
            "repositories": repositories,
            "scope": "raw payload to production DTO decode and native normalize",
            "excluded": "WebSocket framing, socket/TLS, framework routing, strategy callback, order/private path",
            "wire_equivalence": {
                "trade_decode_native_normalize": "same logical trade values; Extrema/Nautilus aggTrade and Barter trade production wire events",
                "book_ticker_decode_native_normalize": "byte-identical Binance USD-M bookTicker payload",
                "depth_frame_decode_native_normalize": "byte-identical Binance USD-M depthUpdate payload; excludes Barter snapshot/sequencer and local-book apply",
            },
        },
    )

    case_index = 0
    for repeat in range(1, args.repeats + 1):
        for channel in rotated(CHANNELS, repeat - 1):
            for framework in rotated(FRAMEWORKS, case_index):
                benchmark_id = f"{channel}/{framework}"
                started = time.monotonic()
                completed = subprocess.run(
                    (
                        "cargo",
                        "bench",
                        "--offline",
                        "--manifest-path",
                        str(PROJECT / "Cargo.toml"),
                        "--bench",
                        "channels",
                        "--",
                        benchmark_id,
                    ),
                    cwd=ROOT,
                    text=True,
                    capture_output=True,
                )
                if completed.returncode != 0:
                    detail = (completed.stderr or completed.stdout)[-4_000:]
                    raise RuntimeError(f"{benchmark_id} failed: {detail}")
                result = estimates(channel, framework)
                append(
                    output,
                    {
                        "record_type": "measurement",
                        "repeat": repeat,
                        "channel": channel,
                        "framework": framework,
                        "wall_seconds": time.monotonic() - started,
                        **result,
                    },
                )
                print(
                    f"DONE repeat={repeat} {benchmark_id} "
                    f"median={result['median_ns']:.2f} ns",
                    flush=True,
                )
                case_index += 1
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
