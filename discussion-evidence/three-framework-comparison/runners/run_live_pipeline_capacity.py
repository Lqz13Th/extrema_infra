#!/usr/bin/env python3
"""Run and normalize the three zero-network live-pipeline benchmarks."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = (
    ROOT / "outputs" / "live-pipeline-capacity-raw-2026-08-02-batch-sharded.jsonl"
)
RESULT_PREFIX = "LIVE_PIPELINE_RESULT "
SHARED_TOPOLOGY = "shared_handler"
BATCH_SHARDED_TOPOLOGY = "batch_sharded"
WS5_STRATEGY20_TOPOLOGY = "ws5_strategy20"
BATCH_TRADE_LANES = 20
SIGNAL_HANDLERS_PER_BATCH = 5
TRADE_LANES_PER_SIGNAL_HANDLER = 4
WS5_PUBLIC_WS_TASKS = 20
WS5_SYMBOLS_PER_PUBLIC_WS = 5
WS5_SIGNAL_HANDLERS = 20
REQUIRED_WORKER_THREADS = 16
PRIVATE_INGRESS_CAPACITY = 8192
RUNNER_SCHEMA_VERSION = 6


@dataclass(frozen=True)
class Framework:
    name: str
    repo: Path
    command: tuple[str, ...]
    source_files: tuple[str, ...]


FRAMEWORKS = (
    Framework(
        "nautilus_trader",
        ROOT / "work" / "repos" / "nautilus_trader",
        (
            "cargo",
            "bench",
            "--offline",
            "--locked",
            "-p",
            "nautilus-binance",
            "--bench",
            "live_pipeline",
        ),
        (
            "crates/adapters/binance/benches/live_pipeline/main.rs",
            "crates/adapters/binance/benches/live_pipeline/support.rs",
            "crates/adapters/binance/Cargo.toml",
        ),
    ),
    Framework(
        "barter-rs",
        ROOT / "work" / "repos" / "barter-rs",
        ("cargo", "bench", "--offline", "-p", "barter", "--bench", "live_pipeline"),
        (
            "barter/benches/live_pipeline.rs",
            "barter/tests/live_pipeline_bench.rs",
            "barter/Cargo.toml",
        ),
    ),
    Framework(
        "extrema_infra",
        ROOT / "work" / "repos" / "extrema_infra",
        (
            "cargo",
            "bench",
            "--offline",
            "--features",
            "bench-internal",
            "--bench",
            "live_pipeline",
        ),
        (
            "src/live_pipeline_benchmark.rs",
            "benches/live_pipeline.rs",
            "src/arch/market_assets/exchange/binance/benchmark.rs",
            "src/arch/market_assets/exchange/binance/schemas/um_futures_ws/account_order.rs",
            "src/arch/strategy_base/handler/handler_core.rs",
            "src/arch/strategy_base/handler/task_channel.rs",
            "src/arch/task_execution/alt_runner.rs",
            "Cargo.toml",
        ),
    ),
)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def nonnegative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("value must be nonnegative")
    return parsed


def parse_int_list(value: str) -> list[int]:
    try:
        values = [positive_int(item.strip()) for item in value.split(",")]
    except (ValueError, argparse.ArgumentTypeError) as error:
        raise argparse.ArgumentTypeError(str(error)) from error
    if not values or len(values) != len(set(values)):
        raise argparse.ArgumentTypeError("list must be non-empty and contain no duplicates")
    return values


def parse_modes(value: str) -> list[str]:
    modes = [item.strip() for item in value.split(",")]
    if not modes or len(modes) != len(set(modes)):
        raise argparse.ArgumentTypeError("mode list must be non-empty and unique")
    invalid = set(modes) - {"stress", "mixed_live"}
    if invalid:
        raise argparse.ArgumentTypeError(f"invalid modes: {sorted(invalid)}")
    return modes


def parse_topologies(value: str) -> list[str]:
    topologies = [item.strip() for item in value.split(",")]
    if not topologies or len(topologies) != len(set(topologies)):
        raise argparse.ArgumentTypeError("topology list must be non-empty and unique")
    invalid = set(topologies) - {
        SHARED_TOPOLOGY,
        BATCH_SHARDED_TOPOLOGY,
        WS5_STRATEGY20_TOPOLOGY,
    }
    if invalid:
        raise argparse.ArgumentTypeError(f"invalid topologies: {sorted(invalid)}")
    return topologies


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--tasks", type=parse_int_list, default=parse_int_list("1,2,5,10,16,32,64")
    )
    parser.add_argument("--modes", type=parse_modes, default=parse_modes("stress,mixed_live"))
    parser.add_argument(
        "--topologies",
        type=parse_topologies,
        default=parse_topologies(BATCH_SHARDED_TOPOLOGY),
        help="topology cases run identically for all three frameworks",
    )
    parser.add_argument(
        "--business-error-tasks", type=parse_int_list, default=parse_int_list("5")
    )
    parser.add_argument("--skip-business-error", action="store_true")
    parser.add_argument("--instruments-per-task", type=positive_int, default=20)
    parser.add_argument("--messages-per-instrument", type=positive_int, default=256)
    parser.add_argument("--order-every", type=positive_int, default=20)
    parser.add_argument("--reject-every", type=positive_int, default=10)
    parser.add_argument("--signal-compute-ns", type=nonnegative_int, default=0)
    parser.add_argument("--worker-threads", type=positive_int, default=16)
    parser.add_argument("--warmup-runs", type=positive_int, default=1)
    parser.add_argument("--measured-runs", type=positive_int, default=5)
    parser.add_argument("--command-timeout-seconds", type=positive_int, default=900)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--overwrite", action="store_true")
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    if args.overwrite and args.resume:
        parser.error("--overwrite and --resume are mutually exclusive")
    if (
        BATCH_SHARDED_TOPOLOGY in args.topologies
        and args.instruments_per_task != BATCH_TRADE_LANES
    ):
        parser.error(
            f"batch_sharded requires --instruments-per-task={BATCH_TRADE_LANES}"
        )
    if WS5_STRATEGY20_TOPOLOGY in args.topologies:
        if args.instruments_per_task != WS5_SYMBOLS_PER_PUBLIC_WS:
            parser.error(
                "ws5_strategy20 requires "
                f"--instruments-per-task={WS5_SYMBOLS_PER_PUBLIC_WS}"
            )
        invalid_tasks = set(args.tasks) - {WS5_PUBLIC_WS_TASKS}
        invalid_error_tasks = (
            set() if args.skip_business_error else set(args.business_error_tasks)
        ) - {WS5_PUBLIC_WS_TASKS}
        if invalid_tasks or invalid_error_tasks:
            parser.error(
                "ws5_strategy20 requires --tasks=20 and "
                "--business-error-tasks=20"
            )
    if (
        BATCH_SHARDED_TOPOLOGY in args.topologies
        and WS5_STRATEGY20_TOPOLOGY in args.topologies
    ):
        parser.error(
            "batch_sharded and ws5_strategy20 require different "
            "--instruments-per-task values; run them as separate datasets"
        )
    if args.worker_threads != REQUIRED_WORKER_THREADS:
        parser.error(
            f"--worker-threads must be {REQUIRED_WORKER_THREADS}; "
            "the current Barter benchmark has a fixed 16-thread runtime"
        )
    if (
        not args.skip_business_error
        and "mixed_live" in args.modes
        and args.messages_per_instrument < args.order_every
    ):
        parser.error(
            "mixed_live business_error cases require "
            "--messages-per-instrument >= --order-every so the error path is exercised"
        )
    return args


def command_output(command: list[str], cwd: Path) -> str:
    return subprocess.run(
        command,
        cwd=cwd,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def source_digest(framework: Framework) -> str:
    digest = hashlib.sha256()
    for relative in framework.source_files:
        path = framework.repo / relative
        digest.update(relative.encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def runner_digest() -> str:
    return hashlib.sha256(Path(__file__).resolve().read_bytes()).hexdigest()


def metadata(args: argparse.Namespace) -> dict[str, Any]:
    repositories = {}
    for framework in FRAMEWORKS:
        repositories[framework.name] = {
            "commit": command_output(["git", "rev-parse", "HEAD"], framework.repo),
            "dirty": bool(command_output(["git", "status", "--porcelain"], framework.repo)),
            "benchmark_source_sha256": source_digest(framework),
        }
    return {
        "record_type": "metadata",
        "runner_schema_version": RUNNER_SCHEMA_VERSION,
        "runner_source_sha256": runner_digest(),
        "created_at_epoch_seconds": time.time(),
        "host": platform.node(),
        "platform": platform.platform(),
        "logical_cpus": os.cpu_count(),
        "rustc": command_output(["rustc", "--version"], ROOT),
        "cargo": command_output(["cargo", "--version"], ROOT),
        "runtime_worker_threads": args.worker_threads,
        "signal_compute_ns": args.signal_compute_ns,
        "task_ladder": args.tasks,
        "modes": args.modes,
        "case_key_fields": ["framework", "topology", "mode", "scenario", "tasks"],
        "topologies_by_framework": {
            framework.name: args.topologies for framework in FRAMEWORKS
        },
        "batch_sharded_contract": {
            "trade_lanes_per_batch": BATCH_TRADE_LANES,
            "cooperative_trade_ingress_tasks_per_lane": 1,
            "trade_ingress_yields_after_each_message": True,
            "bounded_raw_account_ingress_tasks_per_batch": 1,
            "private_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
            "signal_handlers_per_batch": SIGNAL_HANDLERS_PER_BATCH,
            "trade_lanes_per_signal_handler": TRADE_LANES_PER_SIGNAL_HANDLER,
            "account_callbacks_per_private_frame": SIGNAL_HANDLERS_PER_BATCH,
            "canonical_account_callbacks_per_private_frame": 1,
            "required_loss_duplicate_route_cross_batch_errors": 0,
            "physical_fanout_by_framework": {
                "nautilus_trader": "one owner native callback plus four non-owner CustomData callbacks",
                "barter-rs": "five concrete logical handlers inside one native composite Strategy callback",
                "extrema_infra": "one Tokio broadcast ring with five native Strategy task receivers",
            },
        },
        "ws5_strategy20_contract": {
            "public_ws_tasks": WS5_PUBLIC_WS_TASKS,
            "symbols_per_public_ws": WS5_SYMBOLS_PER_PUBLIC_WS,
            "total_symbols": WS5_PUBLIC_WS_TASKS * WS5_SYMBOLS_PER_PUBLIC_WS,
            "signal_handlers": WS5_SIGNAL_HANDLERS,
            "symbols_per_signal_handler": WS5_SYMBOLS_PER_PUBLIC_WS,
            "account_ingress_tasks": 1,
            "account_fanout_domains": 1,
            "account_callbacks_per_private_frame": WS5_SIGNAL_HANDLERS,
            "canonical_account_callbacks_per_private_frame": 1,
            "signal_compute_calls_per_public_tick": 1,
            "signal_compute_model": "CPU-bound busy wait in the owning signal callback before the order decision",
            "required_loss_duplicate_route_cross_batch_errors": 0,
            "physical_signal_dispatch_by_framework": {
                "nautilus_trader": "twenty real Strategies synchronously dispatched by one LiveNode engine thread",
                "barter-rs": "twenty logical handlers synchronously dispatched inside one native composite Strategy and Engine",
                "extrema_infra": "twenty native Strategy tasks scheduled on one multi-thread Tokio runtime",
            },
            "physical_private_fanout_by_framework": {
                "nautilus_trader": "one owner native callback plus nineteen non-owner CustomData callbacks",
                "barter-rs": "twenty logical observations inside one native composite Strategy callback",
                "extrema_infra": "one Tokio broadcast ring with twenty native Strategy task receivers",
            },
        },
        "business_error_tasks": [] if args.skip_business_error else args.business_error_tasks,
        "instruments_per_task": args.instruments_per_task,
        "messages_per_instrument": args.messages_per_instrument,
        "order_every": args.order_every,
        "reject_every": args.reject_every,
        "warmup_runs": args.warmup_runs,
        "measured_runs": args.measured_runs,
        "repositories": repositories,
        "scope": {
            "included": "prebuilt raw public decode, configured signal computation, and native terminal private callback",
            "excluded": "TLS, socket IO, real venue latency, reconnect/backoff, sleeps",
        },
    }


def framework_topologies(framework: Framework, args: argparse.Namespace) -> tuple[str, ...]:
    del framework
    return tuple(args.topologies)


def expected_counts(
    mode: str,
    scenario: str,
    tasks: int,
    instruments_per_task: int,
    messages_per_instrument: int,
    order_every: int,
    reject_every: int,
) -> dict[str, int]:
    ticks = tasks * instruments_per_task * messages_per_instrument
    orders_per_task = (
        instruments_per_task * messages_per_instrument
        if mode == "stress"
        else instruments_per_task * (messages_per_instrument // order_every)
    )
    orders = tasks * orders_per_task
    rejects = 0 if scenario == "success" else (orders + reject_every - 1) // reject_every
    successes = orders - rejects
    return {
        "ticks": ticks,
        "orders": orders,
        "rejects": rejects,
        "successes": successes,
        "private_frames": successes * 2 + rejects,
        "orders_per_task": orders_per_task,
        "per_task_ticks": instruments_per_task * messages_per_instrument,
    }


def expected_private_frames_by_task(
    scenario: str,
    tasks: int,
    orders_per_task: int,
    reject_every: int,
) -> list[int]:
    if scenario == "success":
        return [orders_per_task * 2] * tasks
    frames = []
    for task in range(tasks):
        first_ordinal = task * orders_per_task
        first_reject_offset = (-first_ordinal) % reject_every
        rejects = (
            0
            if first_reject_offset >= orders_per_task
            else 1 + (orders_per_task - first_reject_offset - 1) // reject_every
        )
        frames.append((orders_per_task - rejects) * 2 + rejects)
    return frames


def expected_topology(
    framework: str,
    topology: str,
    tasks: int,
    total_ticks: int,
    messages_per_instrument: int,
    private_frames: int,
    private_frames_by_task: list[int],
) -> dict[str, Any]:
    if topology == SHARED_TOPOLOGY:
        common = {
            "batch_count": 1,
            "producer_tasks": tasks,
            "signal_handlers": 1,
            "venue_handlers": 1,
            "account_owner_handlers": 1,
            "account_handlers_per_fanout_domain": 1,
            "expected_account_callbacks": private_frames,
            "per_signal_trade_callbacks_min": total_ticks,
            "per_signal_trade_callbacks_max": total_ticks,
            "native_owner_callbacks": private_frames,
            "account_non_owner_callbacks": 0,
            "public_cooperative_yields": total_ticks,
        }
        if framework == "extrema_infra":
            return common | {
                "trade_task_keys": tasks,
                "trade_ingress_lanes": tasks,
                "account_ingress_tasks": tasks,
                "account_ingress_lanes": tasks,
                "account_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
                "account_broadcast_rings": tasks,
                "account_topics": 0,
                "account_fanout_domains": tasks,
                "account_receivers_per_ring": 1,
                "order_runners": tasks,
                "trade_lanes_per_signal_handler": tasks,
                "per_signal_private_callbacks_min": 0,
                "per_signal_private_callbacks_max": 0,
            }
        total_instruments = tasks * BATCH_TRADE_LANES
        return common | {
            "trade_task_keys": 0,
            "trade_ingress_lanes": (
                total_instruments if framework == "barter-rs" else tasks
            ),
            "account_ingress_tasks": 0,
            "account_ingress_lanes": 0 if framework == "barter-rs" else 1,
            "account_ingress_capacity": 0,
            "account_broadcast_rings": 0,
            "account_topics": 0,
            "account_fanout_domains": 1,
            "account_receivers_per_ring": 0,
            "order_runners": 0,
            "trade_lanes_per_signal_handler": total_instruments,
            "per_signal_private_callbacks_min": private_frames,
            "per_signal_private_callbacks_max": private_frames,
        }

    if topology == WS5_STRATEGY20_TOPOLOGY:
        physical = {
            "nautilus_trader": {
                "trade_task_keys": 0,
                "account_broadcast_rings": 0,
                "account_topics": WS5_SIGNAL_HANDLERS,
                "account_receivers_per_ring": 0,
                "account_owner_handlers": WS5_SIGNAL_HANDLERS,
                "order_runners": 1,
            },
            "barter-rs": {
                "trade_task_keys": 0,
                "account_broadcast_rings": 0,
                "account_topics": 0,
                "account_receivers_per_ring": 0,
                "account_owner_handlers": 1,
                "order_runners": 0,
            },
            "extrema_infra": {
                "trade_task_keys": WS5_PUBLIC_WS_TASKS,
                "account_broadcast_rings": 1,
                "account_topics": 0,
                "account_receivers_per_ring": WS5_SIGNAL_HANDLERS,
                "account_owner_handlers": 1,
                "order_runners": 1,
            },
        }[framework]
        return physical | {
            "batch_count": 1,
            "producer_tasks": WS5_PUBLIC_WS_TASKS,
            "public_ws_tasks": WS5_PUBLIC_WS_TASKS,
            "public_ws_tasks_per_fanout_domain": WS5_PUBLIC_WS_TASKS,
            "symbols_per_public_ws": WS5_SYMBOLS_PER_PUBLIC_WS,
            "symbols_per_signal_handler": WS5_SYMBOLS_PER_PUBLIC_WS,
            "trade_ingress_lanes": WS5_PUBLIC_WS_TASKS,
            "signal_handlers": WS5_SIGNAL_HANDLERS,
            "venue_handlers": 1,
            "account_ingress_tasks": 1,
            "account_ingress_lanes": 1,
            "account_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
            "account_fanout_domains": 1,
            "account_handlers_per_fanout_domain": WS5_SIGNAL_HANDLERS,
            "trade_lanes_per_signal_handler": 1,
            "expected_account_callbacks": private_frames * WS5_SIGNAL_HANDLERS,
            "native_owner_callbacks": private_frames,
            "account_non_owner_callbacks": private_frames * (WS5_SIGNAL_HANDLERS - 1),
            "public_cooperative_yields": total_ticks,
            "per_signal_trade_callbacks_min": (
                WS5_SYMBOLS_PER_PUBLIC_WS * messages_per_instrument
            ),
            "per_signal_trade_callbacks_max": (
                WS5_SYMBOLS_PER_PUBLIC_WS * messages_per_instrument
            ),
            "per_signal_private_callbacks_min": private_frames,
            "per_signal_private_callbacks_max": private_frames,
        }

    physical = {
        "nautilus_trader": {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": tasks * SIGNAL_HANDLERS_PER_BATCH,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": tasks * SIGNAL_HANDLERS_PER_BATCH,
            "order_runners": tasks,
        },
        "barter-rs": {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": 0,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": tasks,
            "order_runners": 0,
        },
        "extrema_infra": {
            "trade_task_keys": tasks * BATCH_TRADE_LANES,
            "account_broadcast_rings": tasks,
            "account_topics": 0,
            "account_receivers_per_ring": SIGNAL_HANDLERS_PER_BATCH,
            "account_owner_handlers": tasks,
            "order_runners": tasks,
        },
    }[framework]
    return physical | {
        "batch_count": tasks,
        "producer_tasks": tasks * BATCH_TRADE_LANES,
        "trade_ingress_lanes": tasks * BATCH_TRADE_LANES,
        "signal_handlers": tasks * SIGNAL_HANDLERS_PER_BATCH,
        "venue_handlers": tasks,
        "account_ingress_tasks": tasks,
        "account_ingress_lanes": tasks,
        "account_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
        "account_fanout_domains": tasks,
        "account_handlers_per_fanout_domain": SIGNAL_HANDLERS_PER_BATCH,
        "trade_lanes_per_signal_handler": TRADE_LANES_PER_SIGNAL_HANDLER,
        "expected_account_callbacks": private_frames * SIGNAL_HANDLERS_PER_BATCH,
        "native_owner_callbacks": private_frames,
        "account_non_owner_callbacks": private_frames * (SIGNAL_HANDLERS_PER_BATCH - 1),
        "public_cooperative_yields": total_ticks,
        "per_signal_trade_callbacks_min": (
            TRADE_LANES_PER_SIGNAL_HANDLER * messages_per_instrument
        ),
        "per_signal_trade_callbacks_max": (
            TRADE_LANES_PER_SIGNAL_HANDLER * messages_per_instrument
        ),
        "per_signal_private_callbacks_min": min(private_frames_by_task),
        "per_signal_private_callbacks_max": max(private_frames_by_task),
    }


def latency(
    raw: dict[str, Any], framework: str, name: str, expected_samples: int
) -> dict[str, int]:
    if framework == "nautilus_trader":
        value = raw[f"{name}_ns"]
        return {
            "samples": int(value.get("samples", expected_samples)),
            "p50_ns": int(value["p50"]),
            "p95_ns": int(value["p95"]),
            "p99_ns": int(value["p99"]),
            "max_ns": int(value["max"]),
        }
    if framework == "barter-rs":
        value = raw[name]
        return {
            key: int(value[key])
            for key in ("samples", "p50_ns", "p95_ns", "p99_ns", "max_ns")
        }
    return {
        "samples": int(raw[f"{name}_samples"]),
        "p50_ns": int(raw[f"{name}_p50_ns"]),
        "p95_ns": int(raw[f"{name}_p95_ns"]),
        "p99_ns": int(raw[f"{name}_p99_ns"]),
        "max_ns": int(raw[f"{name}_max_ns"]),
    }


def field(raw: dict[str, Any], *names: str, default: Any = None) -> Any:
    for name in names:
        if name in raw:
            return raw[name]
    return default


def normalize(
    framework: str,
    raw: dict[str, Any],
    topology: str,
    mode: str,
    scenario: str,
    tasks: int,
    args: argparse.Namespace,
) -> dict[str, Any]:
    expected = expected_counts(
        mode,
        scenario,
        tasks,
        args.instruments_per_task,
        args.messages_per_instrument,
        args.order_every,
        args.reject_every,
    )
    public = latency(raw, framework, "public_decode", expected["ticks"])
    order = latency(raw, framework, "tick_to_order", expected["orders"])
    ack = latency(raw, framework, "tick_to_ack", expected["successes"])
    fill = latency(raw, framework, "tick_to_fill", expected["successes"])
    private_frames_by_task = expected_private_frames_by_task(
        scenario,
        tasks,
        expected["orders_per_task"],
        args.reject_every,
    )
    topology_expected = expected_topology(
        framework,
        topology,
        tasks,
        expected["ticks"],
        args.messages_per_instrument,
        expected["private_frames"],
        private_frames_by_task,
    )

    def optional_int(*names: str) -> int | None:
        value = field(raw, *names)
        return None if value is None else int(value)

    counts: dict[str, Any] = {
        "total_ticks": int(raw["total_ticks"]),
        "expected_orders": int(raw["expected_orders"]),
        "expected_success_orders": int(
            field(
                raw,
                "expected_success_orders",
                "expected_successful_orders",
                "expected_fills",
                default=expected["successes"],
            )
        ),
        "public_parsed": int(raw["public_parsed"]),
        "trade_callbacks": int(raw["trade_callbacks"]),
        "signal_compute_ns": int(raw["signal_compute_ns"]),
        "signal_compute_calls": int(raw["signal_compute_calls"]),
        "order_endpoint_calls": int(field(raw, "order_endpoint_calls", "orders_submitted")),
        "private_frames_parsed": int(raw["private_frames_parsed"]),
        "acks": int(raw["acks"]),
        "fills": int(raw["fills"]),
        "business_errors": int(raw["business_errors"]),
        "drops": int(raw["drops"]),
        "duplicates": int(raw["duplicates"]),
        "route_errors": int(raw["route_errors"]),
        "cross_batch_errors": int(raw["cross_batch_errors"]),
        "missing": int(field(raw, "missing", "missing_correlations", default=0)),
        "account_delivery_lost": int(field(raw, "account_delivery_lost", default=0)),
        "ending_in_flight": int(raw["ending_in_flight"]),
        "max_in_flight": int(raw["max_in_flight"]),
        "task_sent_min": int(field(raw, "task_sent_min", "per_task_sent_min")),
        "task_sent_max": int(field(raw, "task_sent_max", "per_task_sent_max")),
        "task_completed_min": int(
            field(raw, "task_completed_min", "per_task_completed_min")
        ),
        "task_completed_max": int(
            field(raw, "task_completed_max", "per_task_completed_max")
        ),
        "runtime_worker_threads": int(raw["runtime_worker_threads"]),
        "physical_sockets": optional_int("physical_sockets"),
        "batch_count": int(field(raw, "batch_count", "engine_count")),
        "producer_tasks": int(raw["producer_tasks"]),
        "public_ws_tasks": int(
            field(raw, "public_ws_tasks", default=raw["producer_tasks"])
        ),
        "public_ws_tasks_per_fanout_domain": int(
            field(raw, "public_ws_tasks_per_fanout_domain", default=0)
        ),
        "symbols_per_public_ws": int(
            field(raw, "symbols_per_public_ws", default=0)
        ),
        "symbols_per_signal_handler": int(
            field(raw, "symbols_per_signal_handler", "instruments_per_handler", default=0)
        ),
        "trade_task_keys": int(raw["trade_task_keys"]),
        "trade_ingress_lanes": int(
            field(raw, "trade_ingress_lanes", "trade_lanes")
        ),
        "public_cooperative_yields": int(raw["public_cooperative_yields"]),
        "signal_handlers": int(raw["signal_handlers"]),
        "venue_handlers": int(raw["venue_handlers"]),
        "account_ingress_tasks": int(raw["account_ingress_tasks"]),
        "account_ingress_lanes": int(raw["account_ingress_lanes"]),
        "account_ingress_capacity": int(
            field(raw, "account_ingress_capacity", "private_ingress_capacity")
        ),
        "account_broadcast_rings": int(raw["account_broadcast_rings"]),
        "account_topics": int(
            field(raw, "account_topics", "account_broadcast_topics")
        ),
        "account_fanout_domains": int(raw["account_fanout_domains"]),
        "account_receivers_per_ring": int(raw["account_receivers_per_ring"]),
        "account_handlers_per_fanout_domain": int(
            field(
                raw,
                "account_handlers_per_fanout_domain",
                "account_receivers_per_fanout_domain",
            )
        ),
        "account_owner_handlers": int(raw["account_owner_handlers"]),
        "order_runners": int(raw["order_runners"]),
        "trade_lanes_per_signal_handler": int(
            raw["trade_lanes_per_signal_handler"]
        ),
        "expected_account_callbacks": int(
            field(raw, "expected_account_callbacks", "expected_private_fanout_callbacks")
        ),
        # This normalized field always means logical strategy observations, not a
        # framework-specific native account callback counter.
        "account_callbacks": int(
            field(raw, "account_observation_callbacks", "private_fanout_callbacks")
        ),
        "account_observation_callbacks": int(
            field(raw, "account_observation_callbacks", "private_fanout_callbacks")
        ),
        "account_observation_sources": int(
            field(raw, "account_observation_sources", default=raw["private_frames_parsed"])
        ),
        "native_owner_callbacks": int(raw["native_owner_callbacks"]),
        "account_non_owner_callbacks": int(raw["account_non_owner_callbacks"]),
        "broadcast_lagged": int(field(raw, "broadcast_lagged", default=0)),
        "per_signal_trade_callbacks_min": optional_int(
            "per_signal_trade_callbacks_min", "signal_trade_callbacks_min"
        ),
        "per_signal_trade_callbacks_max": optional_int(
            "per_signal_trade_callbacks_max", "signal_trade_callbacks_max"
        ),
        "per_signal_private_callbacks_min": optional_int(
            "per_signal_private_callbacks_min", "signal_private_callbacks_min"
        ),
        "per_signal_private_callbacks_max": optional_int(
            "per_signal_private_callbacks_max", "signal_private_callbacks_max"
        ),
        "trade_broadcast_capacity": optional_int("trade_broadcast_capacity"),
        "order_execution_broadcast_capacity": optional_int(
            "order_execution_broadcast_capacity"
        ),
        "account_order_broadcast_capacity": optional_int(
            "account_order_broadcast_capacity"
        ),
    }
    topology_failures = [
        f"{name}: got={counts[name]!r} expected={value!r}"
        for name, value in topology_expected.items()
        if counts.get(name) is not None and counts.get(name) != value
    ]
    for name, value in topology_expected.items():
        if counts.get(name) is None:
            topology_failures.append(f"{name}: missing, expected={value!r}")

    public_ingress_model = str(raw["public_ingress_model"])
    private_ingress_model = str(raw["private_ingress_model"])
    signal_handler_model = str(raw["signal_handler_model"])
    batch_isolation_model = str(raw["batch_isolation_model"])
    private_fanout_model = str(raw["private_fanout_model"])
    overflow_model = str(raw["overflow_model"])
    handler_kind = str(field(raw, "handler_kind", "trade_handler_kind"))
    account_owner_selection_model = str(raw["account_owner_selection_model"])
    model_failure = any(
        not value
        for value in (
            public_ingress_model,
            private_ingress_model,
            signal_handler_model,
            batch_isolation_model,
            private_fanout_model,
            overflow_model,
            handler_kind,
            account_owner_selection_model,
        )
    )
    expected_per_task_ticks = expected["per_task_ticks"]
    failures = (
        bool(topology_failures)
        or model_failure
        or str(raw["framework"]) != framework
        or str(raw["topology"]) != topology
        or str(raw["mode"]) != mode
        or str(raw["scenario"]) != scenario
        or int(raw["tasks"]) != tasks
        or int(raw["instruments_per_task"]) != args.instruments_per_task
        or int(raw["messages_per_instrument"]) != args.messages_per_instrument
        or int(raw["order_every"]) != args.order_every
        or int(field(raw, "reject_every", default=args.reject_every)) != args.reject_every
        or counts["signal_compute_ns"] != args.signal_compute_ns
        or counts["signal_compute_calls"] != expected["ticks"]
        or counts["runtime_worker_threads"] != args.worker_threads
        or counts["total_ticks"] != expected["ticks"]
        or counts["expected_orders"] != expected["orders"]
        or counts["expected_success_orders"] != expected["successes"]
        or counts["public_parsed"] != expected["ticks"]
        or counts["trade_callbacks"] != expected["ticks"]
        or counts["order_endpoint_calls"] != expected["orders"]
        or counts["private_frames_parsed"] != expected["private_frames"]
        or counts["acks"] != expected["successes"]
        or counts["fills"] != expected["successes"]
        or counts["business_errors"] != expected["rejects"]
        or counts["drops"] != 0
        or counts["duplicates"] != 0
        or counts["route_errors"] != 0
        or counts["cross_batch_errors"] != 0
        or counts["missing"] != 0
        or counts["account_delivery_lost"] != 0
        or counts["broadcast_lagged"] != 0
        or counts["ending_in_flight"] != 0
        or counts["task_sent_min"] != expected_per_task_ticks
        or counts["task_sent_max"] != expected_per_task_ticks
        or counts["task_completed_min"] != expected_per_task_ticks
        or counts["task_completed_max"] != expected_per_task_ticks
        or public["samples"] != expected["ticks"]
        or order["samples"] != expected["orders"]
        or ack["samples"] != expected["successes"]
        or fill["samples"] != expected["successes"]
    )
    elapsed_ns = int(field(raw, "elapsed_ns", "wall_time_ns", "wall_elapsed_ns"))
    latency_exceeds_wall = any(
        item["max_ns"] > elapsed_ns for item in (public, order, ack, fill)
    )
    failures = failures or latency_exceeds_wall or elapsed_ns <= 0
    result = {
        "record_type": "measurement",
        "framework": framework,
        "topology": topology,
        "mode": mode,
        "scenario": scenario,
        "tasks": tasks,
        "instruments_per_task": args.instruments_per_task,
        "messages_per_instrument": args.messages_per_instrument,
        "order_every": args.order_every,
        "reject_every": args.reject_every,
        "signal_compute_ns": args.signal_compute_ns,
        "run": int(raw["run"]),
        **counts,
        "elapsed_ns": elapsed_ns,
        "throughput_ticks_s": float(raw["wall_throughput_ticks_s"]),
        "public_decode": public,
        "tick_to_order": order,
        "tick_to_ack": ack,
        "tick_to_fill": fill,
        "public_ingress_model": public_ingress_model,
        "private_ingress_model": private_ingress_model,
        "signal_handler_model": signal_handler_model,
        "handler_kind": handler_kind,
        "batch_isolation_model": batch_isolation_model,
        "account_owner_selection_model": account_owner_selection_model,
        "private_fanout_model": private_fanout_model,
        "overflow_model": overflow_model,
        "broadcast_lag_observable": bool(
            field(raw, "broadcast_lag_observable", default=False)
        ),
        "private_bridge": bool(
            field(
                raw,
                "private_bridge",
                "benchmark_local_decode_bridge",
                default=False,
            )
        ),
        "topology_failures": topology_failures,
        "latency_exceeds_wall": latency_exceeds_wall,
        "conservation_ok": not failures,
        "raw": raw,
    }
    return result


def run_framework(
    framework: Framework,
    topology: str,
    mode: str,
    scenario: str,
    tasks: int,
    args: argparse.Namespace,
) -> list[dict[str, Any]]:
    run_env = os.environ.copy()
    run_env.pop("LIVE_TOPOLOGY", None)
    run_env.update(
        {
            "LIVE_TASKS": str(tasks),
            "LIVE_INSTRUMENTS_PER_TASK": str(args.instruments_per_task),
            "LIVE_MESSAGES_PER_INSTRUMENT": str(args.messages_per_instrument),
            "LIVE_MODE": mode,
            "LIVE_ORDER_EVERY": str(args.order_every),
            "LIVE_REJECT_EVERY": str(args.reject_every),
            "LIVE_SIGNAL_COMPUTE_NS": str(args.signal_compute_ns),
            "LIVE_WORKER_THREADS": str(args.worker_threads),
            "LIVE_WARMUP_RUNS": str(args.warmup_runs),
            "LIVE_MEASURED_RUNS": str(args.measured_runs),
            "LIVE_SCENARIO": scenario,
        }
    )
    run_env["LIVE_TOPOLOGY"] = topology
    label = f"{framework.name} topology={topology} {mode}/{scenario} tasks={tasks}"
    print(f"START {label}", flush=True)
    started = time.monotonic()
    try:
        completed = subprocess.run(
            framework.command,
            cwd=framework.repo,
            env=run_env,
            text=True,
            capture_output=True,
            timeout=args.command_timeout_seconds,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"{label} timed out after {error.timeout}s") from error
    if completed.returncode != 0:
        print(completed.stdout, file=sys.stderr)
        print(completed.stderr, file=sys.stderr)
        detail = (completed.stderr.strip() or completed.stdout.strip())[-4_000:]
        raise RuntimeError(
            f"{label} failed with exit code {completed.returncode}; output_tail={detail}"
        )
    raw_results = [
        json.loads(line[len(RESULT_PREFIX) :])
        for line in completed.stdout.splitlines()
        if line.startswith(RESULT_PREFIX)
    ]
    if len(raw_results) != args.measured_runs:
        print(completed.stdout, file=sys.stderr)
        raise RuntimeError(
            f"{label} emitted {len(raw_results)} results, expected {args.measured_runs}"
        )
    normalized = [
        normalize(framework.name, raw, topology, mode, scenario, tasks, args)
        for raw in raw_results
    ]
    for result in normalized:
        if not result["conservation_ok"]:
            raise RuntimeError(f"{label} failed normalized conservation: {result}")
    median = sorted(item["throughput_ticks_s"] for item in normalized)[
        len(normalized) // 2
    ]
    print(
        f"DONE  {label} wall={time.monotonic() - started:.2f}s "
        f"median={median:,.0f} tick/s",
        flush=True,
    )
    return normalized


def append_json_line(path: Path, record: dict[str, Any]) -> None:
    with path.open("a", encoding="utf-8") as output:
        output.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
        output.write("\n")
        output.flush()


def failure_record(
    framework: Framework,
    topology: str,
    mode: str,
    scenario: str,
    tasks: int,
    args: argparse.Namespace,
    error: Exception,
) -> dict[str, Any]:
    return {
        "record_type": "failure",
        "framework": framework.name,
        "topology": topology,
        "mode": mode,
        "scenario": scenario,
        "tasks": tasks,
        "instruments_per_task": args.instruments_per_task,
        "messages_per_instrument": args.messages_per_instrument,
        "order_every": args.order_every,
        "reject_every": args.reject_every,
        "signal_compute_ns": args.signal_compute_ns,
        "runtime_worker_threads": args.worker_threads,
        "failed_at_epoch_seconds": time.time(),
        "error_type": type(error).__name__,
        "error": str(error),
    }


def rotated_frameworks(index: int) -> tuple[Framework, ...]:
    offset = index % len(FRAMEWORKS)
    return FRAMEWORKS[offset:] + FRAMEWORKS[:offset]


def resume_case(record: dict[str, Any]) -> tuple[str, str, str, str, int]:
    return (
        str(record["framework"]),
        str(record["topology"]),
        str(record["mode"]),
        str(record["scenario"]),
        int(record["tasks"]),
    )


def validate_resume_record(record: dict[str, Any], metadata_record: dict[str, Any]) -> None:
    case = resume_case(record)
    framework, topology, mode, scenario, tasks = case
    topologies = metadata_record["topologies_by_framework"]
    if framework not in topologies or topology not in topologies[framework]:
        raise RuntimeError(f"resume output contains an unexpected framework/topology: {case}")
    if mode not in metadata_record["modes"]:
        raise RuntimeError(f"resume output contains an unexpected mode: {case}")
    if scenario == "success":
        allowed_tasks = metadata_record["task_ladder"]
    elif scenario == "business_error":
        allowed_tasks = metadata_record["business_error_tasks"]
    else:
        raise RuntimeError(f"resume output contains an unexpected scenario: {case}")
    if tasks not in allowed_tasks:
        raise RuntimeError(f"resume output contains an unexpected task count: {case}")

    for field_name in (
        "instruments_per_task",
        "messages_per_instrument",
        "order_every",
        "reject_every",
        "signal_compute_ns",
        "runtime_worker_threads",
    ):
        if int(record[field_name]) != int(metadata_record[field_name]):
            raise RuntimeError(
                f"resume measurement metadata mismatch for {case}: {field_name}"
            )
    if record.get("record_type") == "failure":
        if not str(record.get("error", "")):
            raise RuntimeError(f"resume output contains an invalid failure record: {case}")
    elif record.get("conservation_ok") is not True:
        raise RuntimeError(f"resume output contains a failed measurement: {case}")


def load_resume_state(
    path: Path,
    current_metadata: dict[str, Any],
    measured_runs: int,
) -> tuple[
    set[tuple[str, str, str, str, int]],
    set[tuple[str, str, str, str, int]],
]:
    records = [
        json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()
    ]
    metadata_records = [
        record for record in records if record.get("record_type") == "metadata"
    ]
    if len(metadata_records) != 1:
        raise RuntimeError("resume output must contain exactly one metadata record")
    previous = metadata_records[0]
    stable_fields = (
        "runner_schema_version",
        "runner_source_sha256",
        "host",
        "platform",
        "logical_cpus",
        "rustc",
        "cargo",
        "runtime_worker_threads",
        "task_ladder",
        "modes",
        "case_key_fields",
        "topologies_by_framework",
        "batch_sharded_contract",
        "ws5_strategy20_contract",
        "business_error_tasks",
        "instruments_per_task",
        "messages_per_instrument",
        "order_every",
        "reject_every",
        "signal_compute_ns",
        "warmup_runs",
        "measured_runs",
        "repositories",
        "scope",
    )
    mismatches = [
        name
        for name in stable_fields
        if previous.get(name) != current_metadata.get(name)
    ]
    if mismatches:
        raise RuntimeError(f"resume metadata mismatch: {mismatches}")

    runs_by_case: dict[tuple[str, str, str, str, int], list[int]] = {}
    failed_cases: set[tuple[str, str, str, str, int]] = set()
    for record in records:
        if record.get("record_type") not in {"measurement", "failure"}:
            continue
        validate_resume_record(record, previous)
        key = resume_case(record)
        if record["record_type"] == "failure":
            failed_cases.add(key)
        else:
            runs_by_case.setdefault(key, []).append(int(record["run"]))

    expected_runs = set(range(1, measured_runs + 1))
    invalid_runs = {
        key: runs
        for key, runs in runs_by_case.items()
        if len(runs) != measured_runs or set(runs) != expected_runs
    }
    if invalid_runs:
        raise RuntimeError(f"resume output contains partial or duplicate cases: {invalid_runs}")
    overlap = set(runs_by_case) & failed_cases
    if overlap:
        raise RuntimeError(f"resume output contains both success and failure for cases: {overlap}")
    return set(runs_by_case) | failed_cases, failed_cases


def main() -> int:
    args = parse_args()
    args.output = args.output.resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    current_metadata = metadata(args)
    completed_cases: set[tuple[str, str, str, str, int]] = set()
    previous_failed_cases: set[tuple[str, str, str, str, int]] = set()
    if args.output.exists():
        if args.resume:
            completed_cases, previous_failed_cases = load_resume_state(
                args.output, current_metadata, args.measured_runs
            )
        elif not args.overwrite:
            raise FileExistsError(f"output already exists: {args.output}")
        else:
            args.output.unlink()
    if not args.output.exists():
        append_json_line(args.output, current_metadata)

    failure_count = len(previous_failed_cases)

    def execute_case(
        framework: Framework,
        topology: str,
        mode: str,
        scenario: str,
        tasks: int,
    ) -> None:
        nonlocal failure_count
        try:
            results = run_framework(
                framework, topology, mode, scenario, tasks, args
            )
        except Exception as error:  # Continue the capacity scan after an overloaded case.
            failure_count += 1
            append_json_line(
                args.output,
                failure_record(
                    framework, topology, mode, scenario, tasks, args, error
                ),
            )
            print(
                f"FAIL  {framework.name} topology={topology} "
                f"{mode}/{scenario} tasks={tasks}: {error}",
                file=sys.stderr,
                flush=True,
            )
            return
        for result in results:
            append_json_line(args.output, result)

    case_index = 0
    for mode in args.modes:
        for tasks in args.tasks:
            for framework in rotated_frameworks(case_index):
                for topology in framework_topologies(framework, args):
                    case = (framework.name, topology, mode, "success", tasks)
                    if case in completed_cases:
                        print(
                            f"SKIP  {framework.name} topology={topology} "
                            f"{mode}/success tasks={tasks}",
                            flush=True,
                        )
                        continue
                    execute_case(framework, topology, mode, "success", tasks)
            case_index += 1
    if not args.skip_business_error:
        for mode in args.modes:
            for tasks in args.business_error_tasks:
                for framework in rotated_frameworks(case_index):
                    for topology in framework_topologies(framework, args):
                        case = (framework.name, topology, mode, "business_error", tasks)
                        if case in completed_cases:
                            print(
                                f"SKIP  {framework.name} topology={topology} "
                                f"{mode}/business_error tasks={tasks}",
                                flush=True,
                            )
                            continue
                        execute_case(
                            framework, topology, mode, "business_error", tasks
                        )
                case_index += 1
    print(args.output)
    if failure_count:
        print(f"completed with {failure_count} failed cases", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
