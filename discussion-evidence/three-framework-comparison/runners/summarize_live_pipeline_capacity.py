#!/usr/bin/env python3
"""Summarize normalized live-pipeline JSONL measurements as CSV."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Optional, Sequence


ROOT = Path(__file__).resolve().parents[2]
SHARED_TOPOLOGY = "shared_handler"
BATCH_SHARDED_TOPOLOGY = "batch_sharded"
WS5_STRATEGY20_TOPOLOGY = "ws5_strategy20"
BATCH_TRADE_LANES = 20
SIGNAL_HANDLERS_PER_BATCH = 5
TRADE_LANES_PER_SIGNAL_HANDLER = 4
ACCOUNT_OBSERVATIONS_PER_PRIVATE_FRAME = 5
WS5_PUBLIC_WS_TASKS = 20
WS5_SYMBOLS_PER_PUBLIC_WS = 5
WS5_SIGNAL_HANDLERS = 20
PRIVATE_INGRESS_CAPACITY = 8_192
DEFAULT_INPUTS = (
    ROOT / "outputs" / "live-pipeline-capacity-raw-2026-08-02-batch-sharded.jsonl",
)
DEFAULT_OVERRIDE_INPUTS: tuple[Path, ...] = ()
DEFAULT_OUTPUT = ROOT / "outputs" / "live-pipeline-capacity-summary-2026-08-02-batch-sharded.csv"

COMPATIBILITY_FIELDS = (
    "runner_schema_version",
    "runner_source_sha256",
    "host",
    "platform",
    "logical_cpus",
    "rustc",
    "cargo",
    "runtime_worker_threads",
    "instruments_per_task",
    "messages_per_instrument",
    "order_every",
    "reject_every",
    "case_key_fields",
    "topologies_by_framework",
    "batch_sharded_contract",
    "ws5_strategy20_contract",
    "repositories",
    "scope",
)

FIELD_ALIASES = {
    "account_ingress_capacity": (
        "account_ingress_capacity",
        "private_ingress_capacity",
    ),
    "account_topics": ("account_topics", "account_broadcast_topics"),
    "account_callbacks": ("account_callbacks", "account_observation_callbacks"),
    "account_handlers_per_fanout_domain": (
        "account_handlers_per_fanout_domain",
        "account_receivers_per_fanout_domain",
    ),
    "per_signal_trade_callbacks_min": (
        "per_signal_trade_callbacks_min",
        "signal_trade_callbacks_min",
    ),
    "per_signal_trade_callbacks_max": (
        "per_signal_trade_callbacks_max",
        "signal_trade_callbacks_max",
    ),
    "per_signal_private_callbacks_min": (
        "per_signal_private_callbacks_min",
        "signal_private_callbacks_min",
    ),
    "per_signal_private_callbacks_max": (
        "per_signal_private_callbacks_max",
        "signal_private_callbacks_max",
    ),
}

MODEL_FIELDS = (
    "signal_handler_model",
    "handler_kind",
    "batch_isolation_model",
    "private_fanout_model",
    "account_owner_selection_model",
    "public_ingress_model",
    "private_ingress_model",
    "overflow_model",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="*", type=Path, default=None)
    parser.add_argument("--override-inputs", nargs="*", type=Path, default=None)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def median(rows: list[dict[str, Any]], field: str) -> float:
    return statistics.median(float(row[field]) for row in rows)


def latency_median(rows: list[dict[str, Any]], path: str, field: str) -> int:
    return round(statistics.median(int(row[path][field]) for row in rows))


def field(record: dict[str, Any], name: str, default: Any = None) -> Any:
    for candidate in FIELD_ALIASES.get(name, (name,)):
        if candidate in record:
            return record[candidate]
    return default


def required_field(record: dict[str, Any], name: str, input_path: Path) -> Any:
    value = field(record, name)
    if value is None:
        raise RuntimeError(f"{input_path}: measurement missing required field: {name}")
    return value


def required_int(record: dict[str, Any], name: str, input_path: Path) -> int:
    return int(required_field(record, name, input_path))


def strict_nonnegative_int(
    record: dict[str, Any], name: str, input_path: Path
) -> int:
    value = required_field(record, name, input_path)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise RuntimeError(
            f"{input_path}: {name} must be a nonnegative JSON integer, got {value!r}"
        )
    return value


def runner_schema_version(metadata_record: dict[str, Any], input_path: Path) -> int:
    value = metadata_record.get("runner_schema_version", 1)
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise RuntimeError(
            f"{input_path}: runner_schema_version must be a positive JSON integer"
        )
    return value


def metadata_signal_compute_ns(
    metadata_record: dict[str, Any], input_path: Path
) -> int:
    schema_version = runner_schema_version(metadata_record, input_path)
    if schema_version >= 6:
        return strict_nonnegative_int(
            metadata_record, "signal_compute_ns", input_path
        )

    # Schema <=5 predates configurable signal work. Treat it as the legacy
    # zero-delay workload, but do not infer an unrecorded callback count.
    value = metadata_record.get("signal_compute_ns", 0)
    if isinstance(value, bool) or not isinstance(value, int) or value != 0:
        raise RuntimeError(
            f"{input_path}: schema {schema_version} only supports legacy "
            "signal_compute_ns=0"
        )
    return 0


def require_nonempty_models(record: dict[str, Any], input_path: Path) -> None:
    missing = [
        name
        for name in MODEL_FIELDS
        if not isinstance(field(record, name), str)
        or str(field(record, name)).strip() in {"", "None"}
    ]
    if missing:
        raise RuntimeError(f"{input_path}: missing topology model descriptors: {missing}")


def validate_batch_sharded_metadata_contract(
    record: dict[str, Any], input_path: Path
) -> None:
    contract = record.get("batch_sharded_contract")
    if not isinstance(contract, dict):
        raise RuntimeError(f"{input_path}: missing batch_sharded_contract metadata")
    expected = {
        "trade_lanes_per_batch": BATCH_TRADE_LANES,
        "cooperative_trade_ingress_tasks_per_lane": 1,
        "trade_ingress_yields_after_each_message": True,
        "bounded_raw_account_ingress_tasks_per_batch": 1,
        "private_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
        "signal_handlers_per_batch": SIGNAL_HANDLERS_PER_BATCH,
        "trade_lanes_per_signal_handler": TRADE_LANES_PER_SIGNAL_HANDLER,
        "account_callbacks_per_private_frame": (
            ACCOUNT_OBSERVATIONS_PER_PRIVATE_FRAME
        ),
        "canonical_account_callbacks_per_private_frame": 1,
        "required_loss_duplicate_route_cross_batch_errors": 0,
    }
    mismatches = {
        name: (contract.get(name), value)
        for name, value in expected.items()
        if contract.get(name) != value
    }
    physical = contract.get("physical_fanout_by_framework")
    if not isinstance(physical, dict) or set(physical) != {
        "nautilus_trader",
        "barter-rs",
        "extrema_infra",
    }:
        mismatches["physical_fanout_by_framework"] = (
            sorted(physical) if isinstance(physical, dict) else physical,
            ["barter-rs", "extrema_infra", "nautilus_trader"],
        )
    if mismatches:
        raise RuntimeError(
            f"{input_path}: invalid batch_sharded_contract metadata: {mismatches}"
        )


def validate_ws5_metadata_contract(
    record: dict[str, Any], input_path: Path
) -> None:
    contract = record.get("ws5_strategy20_contract")
    if not isinstance(contract, dict):
        raise RuntimeError(f"{input_path}: missing ws5_strategy20_contract metadata")
    expected = {
        "public_ws_tasks": WS5_PUBLIC_WS_TASKS,
        "symbols_per_public_ws": WS5_SYMBOLS_PER_PUBLIC_WS,
        "total_symbols": WS5_PUBLIC_WS_TASKS * WS5_SYMBOLS_PER_PUBLIC_WS,
        "signal_handlers": WS5_SIGNAL_HANDLERS,
        "symbols_per_signal_handler": WS5_SYMBOLS_PER_PUBLIC_WS,
        "account_ingress_tasks": 1,
        "account_fanout_domains": 1,
        "account_callbacks_per_private_frame": WS5_SIGNAL_HANDLERS,
        "canonical_account_callbacks_per_private_frame": 1,
        "required_loss_duplicate_route_cross_batch_errors": 0,
    }
    if runner_schema_version(record, input_path) >= 6:
        expected.update(
            {
                "signal_compute_calls_per_public_tick": 1,
                "signal_compute_model": (
                    "CPU-bound busy wait in the owning signal callback before "
                    "the order decision"
                ),
            }
        )
    mismatches = {
        name: (contract.get(name), value)
        for name, value in expected.items()
        if contract.get(name) != value
    }
    required_frameworks = {
        "nautilus_trader",
        "barter-rs",
        "extrema_infra",
    }
    for name in (
        "physical_signal_dispatch_by_framework",
        "physical_private_fanout_by_framework",
    ):
        physical = contract.get(name)
        if (
            not isinstance(physical, dict)
            or set(physical) != required_frameworks
            or any(not str(value).strip() for value in physical.values())
        ):
            mismatches[name] = (
                sorted(physical) if isinstance(physical, dict) else physical,
                sorted(required_frameworks),
            )
    if mismatches:
        raise RuntimeError(
            f"{input_path}: invalid ws5_strategy20_contract metadata: {mismatches}"
        )


def validate_metadata_contract(record: dict[str, Any], input_path: Path) -> None:
    topologies = record.get("topologies_by_framework", {})
    if not isinstance(topologies, dict):
        raise RuntimeError(f"{input_path}: topologies_by_framework must be an object")
    configured_topologies = {
        topology
        for framework_topologies in topologies.values()
        for topology in framework_topologies
    }
    if BATCH_SHARDED_TOPOLOGY in configured_topologies:
        validate_batch_sharded_metadata_contract(record, input_path)
    if WS5_STRATEGY20_TOPOLOGY in configured_topologies:
        validate_ws5_metadata_contract(record, input_path)


def validate_batch_sharded_measurement(
    record: dict[str, Any], input_path: Path
) -> None:
    framework = str(record["framework"])
    tasks = int(record["tasks"])
    messages = required_int(record, "messages_per_instrument", input_path)
    order_every = required_int(record, "order_every", input_path)
    reject_every = required_int(record, "reject_every", input_path)
    mode = str(record["mode"])
    scenario = str(record["scenario"])
    orders_per_batch = (
        BATCH_TRADE_LANES * messages
        if mode == "stress"
        else BATCH_TRADE_LANES * (messages // order_every)
    )
    private_frames_by_batch: list[int] = []
    for batch in range(tasks):
        if scenario == "success":
            rejects = 0
        else:
            first_order = batch * orders_per_batch
            first_reject_offset = (-first_order) % reject_every
            rejects = (
                0
                if first_reject_offset >= orders_per_batch
                else 1 + (orders_per_batch - first_reject_offset - 1) // reject_every
            )
        private_frames_by_batch.append(orders_per_batch * 2 - rejects)
    expected_private_frames = sum(private_frames_by_batch)
    expected_callbacks = (
        expected_private_frames * ACCOUNT_OBSERVATIONS_PER_PRIVATE_FRAME
    )

    expected_common = {
        "instruments_per_task": BATCH_TRADE_LANES,
        "private_frames_parsed": expected_private_frames,
        "batch_count": tasks,
        "producer_tasks": tasks * BATCH_TRADE_LANES,
        "trade_ingress_lanes": tasks * BATCH_TRADE_LANES,
        "signal_handlers": tasks * SIGNAL_HANDLERS_PER_BATCH,
        "trade_lanes_per_signal_handler": TRADE_LANES_PER_SIGNAL_HANDLER,
        "public_cooperative_yields": tasks * BATCH_TRADE_LANES * messages,
        "account_ingress_tasks": tasks,
        "account_ingress_lanes": tasks,
        "account_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
        "account_fanout_domains": tasks,
        "account_handlers_per_fanout_domain": SIGNAL_HANDLERS_PER_BATCH,
        "venue_handlers": tasks,
        "expected_account_callbacks": expected_callbacks,
        "account_observation_sources": expected_private_frames,
        "per_signal_trade_callbacks_min": (
            TRADE_LANES_PER_SIGNAL_HANDLER * messages
        ),
        "per_signal_trade_callbacks_max": (
            TRADE_LANES_PER_SIGNAL_HANDLER * messages
        ),
        "per_signal_private_callbacks_min": min(private_frames_by_batch),
        "per_signal_private_callbacks_max": max(private_frames_by_batch),
    }
    mismatches = {
        name: (required_int(record, name, input_path), expected)
        for name, expected in expected_common.items()
        if required_int(record, name, input_path) != expected
    }

    if framework == "extrema_infra":
        expected_physical = {
            "trade_task_keys": tasks * BATCH_TRADE_LANES,
            "account_broadcast_rings": tasks,
            "account_topics": 0,
            "account_receivers_per_ring": SIGNAL_HANDLERS_PER_BATCH,
            "account_owner_handlers": tasks,
            "order_runners": tasks,
        }
        expected_lag_observable = True
    elif framework == "barter-rs":
        expected_physical = {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": 0,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": tasks,
            "order_runners": 0,
        }
        expected_lag_observable = False
    elif framework == "nautilus_trader":
        expected_physical = {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": tasks * SIGNAL_HANDLERS_PER_BATCH,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": tasks * SIGNAL_HANDLERS_PER_BATCH,
            "order_runners": tasks,
        }
        expected_lag_observable = False
    else:
        raise RuntimeError(f"{input_path}: unknown framework: {framework}")

    mismatches.update(
        {
            name: (required_int(record, name, input_path), expected)
            for name, expected in expected_physical.items()
            if required_int(record, name, input_path) != expected
        }
    )
    lag_observable = bool(
        required_field(record, "broadcast_lag_observable", input_path)
    )
    if lag_observable != expected_lag_observable:
        mismatches["broadcast_lag_observable"] = (
            lag_observable,
            expected_lag_observable,
        )
    if mismatches:
        raise RuntimeError(
            f"{input_path}: {framework} batch_sharded topology mismatch: {mismatches}"
        )

    require_nonempty_models(record, input_path)
    account_callbacks = required_int(record, "account_callbacks", input_path)
    owner_callbacks = required_int(record, "native_owner_callbacks", input_path)
    non_owner_callbacks = required_int(
        record, "account_non_owner_callbacks", input_path
    )
    callback_errors = (
        abs(account_callbacks - expected_callbacks)
        + abs(owner_callbacks - expected_private_frames)
        + abs(
            non_owner_callbacks
            - expected_private_frames * (SIGNAL_HANDLERS_PER_BATCH - 1)
        )
        + abs(account_callbacks - owner_callbacks - non_owner_callbacks)
    )
    record["_account_callback_errors"] = callback_errors

    diagnostic_errors = sum(
        required_int(record, name, input_path)
        for name in (
            "drops",
            "duplicates",
            "route_errors",
            "missing",
            "ending_in_flight",
            "account_delivery_lost",
            "broadcast_lagged",
            "cross_batch_errors",
        )
    )
    if bool(record.get("conservation_ok")) and callback_errors + diagnostic_errors != 0:
        raise RuntimeError(
            f"{input_path}: conserved measurement reports "
            f"{callback_errors + diagnostic_errors} topology/delivery errors"
        )


def validate_ws5_measurement(record: dict[str, Any], input_path: Path) -> None:
    framework = str(record["framework"])
    tasks = int(record["tasks"])
    messages = required_int(record, "messages_per_instrument", input_path)
    order_every = required_int(record, "order_every", input_path)
    reject_every = required_int(record, "reject_every", input_path)
    mode = str(record["mode"])
    scenario = str(record["scenario"])
    if tasks != WS5_PUBLIC_WS_TASKS:
        raise RuntimeError(
            f"{input_path}: ws5_strategy20 requires tasks={WS5_PUBLIC_WS_TASKS}, "
            f"got {tasks}"
        )

    total_ticks = WS5_PUBLIC_WS_TASKS * WS5_SYMBOLS_PER_PUBLIC_WS * messages
    orders_per_public_ws = (
        WS5_SYMBOLS_PER_PUBLIC_WS * messages
        if mode == "stress"
        else WS5_SYMBOLS_PER_PUBLIC_WS * (messages // order_every)
    )
    expected_orders = WS5_PUBLIC_WS_TASKS * orders_per_public_ws
    expected_rejects = (
        0
        if scenario == "success"
        else (expected_orders + reject_every - 1) // reject_every
    )
    expected_private_frames = expected_orders * 2 - expected_rejects
    expected_callbacks = expected_private_frames * WS5_SIGNAL_HANDLERS
    expected_common = {
        "instruments_per_task": WS5_SYMBOLS_PER_PUBLIC_WS,
        "total_ticks": total_ticks,
        "private_frames_parsed": expected_private_frames,
        "batch_count": 1,
        "producer_tasks": WS5_PUBLIC_WS_TASKS,
        "public_ws_tasks": WS5_PUBLIC_WS_TASKS,
        "public_ws_tasks_per_fanout_domain": WS5_PUBLIC_WS_TASKS,
        "symbols_per_public_ws": WS5_SYMBOLS_PER_PUBLIC_WS,
        "symbols_per_signal_handler": WS5_SYMBOLS_PER_PUBLIC_WS,
        "trade_ingress_lanes": WS5_PUBLIC_WS_TASKS,
        "signal_handlers": WS5_SIGNAL_HANDLERS,
        "trade_lanes_per_signal_handler": 1,
        "public_cooperative_yields": total_ticks,
        "account_ingress_tasks": 1,
        "account_ingress_lanes": 1,
        "account_ingress_capacity": PRIVATE_INGRESS_CAPACITY,
        "account_fanout_domains": 1,
        "account_handlers_per_fanout_domain": WS5_SIGNAL_HANDLERS,
        "venue_handlers": 1,
        "expected_account_callbacks": expected_callbacks,
        "account_observation_sources": expected_private_frames,
        "per_signal_trade_callbacks_min": (
            WS5_SYMBOLS_PER_PUBLIC_WS * messages
        ),
        "per_signal_trade_callbacks_max": (
            WS5_SYMBOLS_PER_PUBLIC_WS * messages
        ),
        "per_signal_private_callbacks_min": expected_private_frames,
        "per_signal_private_callbacks_max": expected_private_frames,
    }
    mismatches = {
        name: (required_int(record, name, input_path), expected)
        for name, expected in expected_common.items()
        if required_int(record, name, input_path) != expected
    }

    if framework == "extrema_infra":
        expected_physical = {
            "trade_task_keys": WS5_PUBLIC_WS_TASKS,
            "account_broadcast_rings": 1,
            "account_topics": 0,
            "account_receivers_per_ring": WS5_SIGNAL_HANDLERS,
            "account_owner_handlers": 1,
            "order_runners": 1,
        }
        expected_lag_observable = True
    elif framework == "barter-rs":
        expected_physical = {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": 0,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": 1,
            "order_runners": 0,
        }
        expected_lag_observable = False
    elif framework == "nautilus_trader":
        expected_physical = {
            "trade_task_keys": 0,
            "account_broadcast_rings": 0,
            "account_topics": WS5_SIGNAL_HANDLERS,
            "account_receivers_per_ring": 0,
            "account_owner_handlers": WS5_SIGNAL_HANDLERS,
            "order_runners": 1,
        }
        expected_lag_observable = False
    else:
        raise RuntimeError(f"{input_path}: unknown framework: {framework}")

    mismatches.update(
        {
            name: (required_int(record, name, input_path), expected)
            for name, expected in expected_physical.items()
            if required_int(record, name, input_path) != expected
        }
    )
    lag_observable = bool(
        required_field(record, "broadcast_lag_observable", input_path)
    )
    if lag_observable != expected_lag_observable:
        mismatches["broadcast_lag_observable"] = (
            lag_observable,
            expected_lag_observable,
        )
    if mismatches:
        raise RuntimeError(
            f"{input_path}: {framework} ws5_strategy20 topology mismatch: "
            f"{mismatches}"
        )

    require_nonempty_models(record, input_path)
    account_callbacks = required_int(record, "account_callbacks", input_path)
    owner_callbacks = required_int(record, "native_owner_callbacks", input_path)
    non_owner_callbacks = required_int(
        record, "account_non_owner_callbacks", input_path
    )
    callback_errors = (
        abs(account_callbacks - expected_callbacks)
        + abs(owner_callbacks - expected_private_frames)
        + abs(
            non_owner_callbacks
            - expected_private_frames * (WS5_SIGNAL_HANDLERS - 1)
        )
        + abs(account_callbacks - owner_callbacks - non_owner_callbacks)
    )
    record["_account_callback_errors"] = callback_errors

    diagnostic_errors = sum(
        required_int(record, name, input_path)
        for name in (
            "drops",
            "duplicates",
            "route_errors",
            "missing",
            "ending_in_flight",
            "account_delivery_lost",
            "broadcast_lagged",
            "cross_batch_errors",
        )
    )
    if bool(record.get("conservation_ok")) and callback_errors + diagnostic_errors != 0:
        raise RuntimeError(
            f"{input_path}: conserved measurement reports "
            f"{callback_errors + diagnostic_errors} topology/delivery errors"
        )


def compatibility_metadata(record: dict[str, Any]) -> dict[str, Any]:
    values = {name: record.get(name) for name in COMPATIBILITY_FIELDS}
    if values["runner_schema_version"] is None:
        values["runner_schema_version"] = 1
        values["runner_source_sha256"] = "legacy"
    return values


def measurement_worker_threads(record: dict[str, Any]) -> Optional[int]:
    value = record.get("runtime_worker_threads")
    if value is None:
        value = record.get("raw", {}).get("runtime_worker_threads")
    return None if value is None else int(value)


def validate_case_record(
    record: dict[str, Any], metadata_record: dict[str, Any], input_path: Path
) -> None:
    schema_version = runner_schema_version(metadata_record, input_path)
    configured_compute_ns = metadata_signal_compute_ns(metadata_record, input_path)
    record["_runner_schema_version"] = schema_version
    record["_signal_compute_ns"] = configured_compute_ns

    mode = str(record["mode"])
    scenario = str(record["scenario"])
    tasks = int(record["tasks"])
    if "modes" in metadata_record and mode not in metadata_record["modes"]:
        raise RuntimeError(f"{input_path}: measurement metadata mismatch: mode")
    if scenario == "success":
        allowed_tasks = metadata_record.get("task_ladder")
    elif scenario == "business_error":
        allowed_tasks = metadata_record.get("business_error_tasks")
    else:
        raise RuntimeError(f"{input_path}: invalid measurement scenario: {scenario}")
    if allowed_tasks is not None and tasks not in allowed_tasks:
        raise RuntimeError(f"{input_path}: measurement metadata mismatch: tasks")
    topologies = metadata_record.get("topologies_by_framework")
    framework = str(record["framework"])
    topology = str(record["topology"])
    if topologies is not None and (
        framework not in topologies or topology not in topologies[framework]
    ):
        raise RuntimeError(
            f"{input_path}: measurement metadata mismatch: framework/topology"
        )

    if schema_version >= 6:
        record_compute_ns = strict_nonnegative_int(
            record, "signal_compute_ns", input_path
        )
        if record_compute_ns != configured_compute_ns:
            raise RuntimeError(
                f"{input_path}: measurement metadata mismatch: signal_compute_ns"
            )
    else:
        record_compute_ns = record.get("signal_compute_ns", 0)
        if (
            isinstance(record_compute_ns, bool)
            or not isinstance(record_compute_ns, int)
            or record_compute_ns != 0
        ):
            raise RuntimeError(
                f"{input_path}: schema {schema_version} measurement must use "
                "legacy signal_compute_ns=0"
            )

    if record.get("record_type") != "measurement":
        return

    if schema_version >= 6:
        signal_compute_calls = strict_nonnegative_int(
            record, "signal_compute_calls", input_path
        )
        total_ticks = strict_nonnegative_int(record, "total_ticks", input_path)
        if signal_compute_calls != total_ticks:
            raise RuntimeError(
                f"{input_path}: signal_compute_calls={signal_compute_calls} "
                f"does not equal total_ticks={total_ticks}"
            )
    for name in (
        "instruments_per_task",
        "messages_per_instrument",
        "order_every",
        "reject_every",
    ):
        if name in metadata_record and int(record[name]) != int(metadata_record[name]):
            raise RuntimeError(f"{input_path}: measurement metadata mismatch: {name}")
    worker_threads = measurement_worker_threads(record)
    if (
        worker_threads is not None
        and "runtime_worker_threads" in metadata_record
        and worker_threads != int(metadata_record["runtime_worker_threads"])
    ):
        raise RuntimeError(
            f"{input_path}: measurement metadata mismatch: runtime_worker_threads"
        )

    if topology == BATCH_SHARDED_TOPOLOGY:
        validate_batch_sharded_measurement(record, input_path)
    elif topology == WS5_STRATEGY20_TOPOLOGY:
        validate_ws5_measurement(record, input_path)


def load_groups(
    paths: Sequence[Path],
    expected_metadata: Optional[dict[str, Any]] = None,
) -> tuple[
    dict[tuple[str, str, str, str, int, int, int], list[dict[str, Any]]],
    Optional[dict[str, Any]],
]:
    groups: dict[
        tuple[str, str, str, str, int, int, int], list[dict[str, Any]]
    ] = defaultdict(list)
    for input_path in paths:
        records = [
            json.loads(line)
            for line in input_path.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        metadata_records = [
            record for record in records if record.get("record_type") == "metadata"
        ]
        if len(metadata_records) != 1:
            raise RuntimeError(f"{input_path}: expected exactly one metadata record")
        metadata_record = metadata_records[0]
        validate_metadata_contract(metadata_record, input_path)
        schema_version = runner_schema_version(metadata_record, input_path)
        signal_compute_ns = metadata_signal_compute_ns(metadata_record, input_path)
        current_metadata = compatibility_metadata(metadata_record)
        if expected_metadata is None:
            expected_metadata = current_metadata
        else:
            mismatches = [
                name
                for name in COMPATIBILITY_FIELDS
                if expected_metadata.get(name) != current_metadata.get(name)
            ]
            if mismatches:
                raise RuntimeError(
                    f"{input_path}: incompatible benchmark metadata: {mismatches}"
                )

        for record in records:
            if record.get("record_type") not in {"measurement", "failure"}:
                continue
            # Measurements produced before topology became a case dimension
            # all used the common shared-handler workload.
            record.setdefault("topology", SHARED_TOPOLOGY)
            validate_case_record(record, metadata_record, input_path)
            key = (
                record["framework"],
                record["topology"],
                record["mode"],
                record["scenario"],
                int(record["tasks"]),
                schema_version,
                signal_compute_ns,
            )
            groups[key].append(record)
    return groups, expected_metadata


def stable_value(rows: list[dict[str, Any]], name: str) -> Any:
    values = [field(row, name) for row in rows]
    if any(value != values[0] for value in values[1:]):
        raise RuntimeError(f"case has inconsistent {name}: {values}")
    return values[0]


def failure_reason(record: dict[str, Any]) -> str:
    for name in ("error", "reason", "message", "detail"):
        value = record.get(name)
        if value:
            reason = str(value).replace("\n", " ")
            error_type = str(record.get("error_type", "")).strip()
            if error_type:
                reason = f"{error_type}: {reason}"
            return reason if len(reason) <= 2_000 else f"{reason[:1_997]}..."
    return "measurement conservation_ok=false"


def expected_case_counts(
    metadata: Optional[dict[str, Any]], mode: str, scenario: str, tasks: int
) -> tuple[Optional[int], Optional[int], Optional[int]]:
    if metadata is None:
        return None, None, None
    required = (
        "instruments_per_task",
        "messages_per_instrument",
        "order_every",
        "reject_every",
    )
    if any(metadata.get(name) is None for name in required):
        return None, None, None
    instruments = int(metadata["instruments_per_task"])
    messages = int(metadata["messages_per_instrument"])
    total_ticks = tasks * instruments * messages
    orders_per_batch = (
        instruments * messages
        if mode == "stress"
        else instruments * (messages // int(metadata["order_every"]))
    )
    expected_orders = tasks * orders_per_batch
    expected_business_errors = (
        0
        if scenario == "success"
        else (expected_orders + int(metadata["reject_every"]) - 1)
        // int(metadata["reject_every"])
    )
    return total_ticks, expected_orders, expected_business_errors


def main() -> int:
    args = parse_args()
    use_defaults = not args.inputs and args.override_inputs is None
    inputs = DEFAULT_INPUTS if not args.inputs else args.inputs
    override_inputs = DEFAULT_OVERRIDE_INPUTS if use_defaults else (args.override_inputs or ())
    groups, input_metadata = load_groups(inputs)
    override_groups, _ = load_groups(override_inputs, input_metadata)
    groups.update(override_groups)

    fieldnames = (
        "framework",
        "topology",
        "mode",
        "scenario",
        "tasks",
        "runner_schema_version",
        "signal_compute_ns",
        "signal_compute_calls",
        "batch_count",
        "runtime_worker_threads",
        "instruments_per_task",
        "total_instruments",
        "messages_per_instrument",
        "total_ticks",
        "expected_orders",
        "expected_business_errors",
        "producer_tasks",
        "trade_ingress_lanes",
        "trade_task_keys",
        "public_cooperative_yields",
        "signal_handlers",
        "trade_lanes_per_signal_handler",
        "handler_kind",
        "account_ingress_tasks",
        "account_ingress_lanes",
        "account_ingress_capacity",
        "venue_handlers",
        "order_runners",
        "account_broadcast_rings",
        "account_topics",
        "account_fanout_domains",
        "account_handlers_per_fanout_domain",
        "account_receivers_per_ring",
        "broadcast_lag_observable",
        "account_owner_handlers",
        "trade_broadcast_capacity",
        "order_execution_broadcast_capacity",
        "account_order_broadcast_capacity",
        "expected_account_callbacks",
        "account_callbacks",
        "account_observation_sources",
        "native_owner_callbacks",
        "account_non_owner_callbacks",
        "per_signal_trade_callbacks_min",
        "per_signal_trade_callbacks_max",
        "per_signal_private_callbacks_min",
        "per_signal_private_callbacks_max",
        "signal_handler_model",
        "batch_isolation_model",
        "private_fanout_model",
        "account_owner_selection_model",
        "public_ingress_model",
        "private_ingress_model",
        "overflow_model",
        "measured_runs",
        "failure_count",
        "failed_runs",
        "failure_reasons",
        "throughput_median_ticks_s",
        "throughput_min_ticks_s",
        "throughput_max_ticks_s",
        "throughput_cv_percent",
        "public_decode_p50_median_ns",
        "public_decode_p99_median_ns",
        "tick_to_order_p50_median_ns",
        "tick_to_order_p99_median_ns",
        "tick_to_ack_p99_median_ns",
        "tick_to_fill_p50_median_ns",
        "tick_to_fill_p99_median_ns",
        "max_in_flight_median",
        "task_completed_min",
        "task_completed_max",
        "private_bridge",
        "conservation_failures",
        "account_callback_errors",
        "account_delivery_lost",
        "broadcast_lagged",
        "cross_batch_errors",
        "errors",
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=fieldnames)
        writer.writeheader()
        for (
            framework,
            topology,
            mode,
            scenario,
            tasks,
            schema_version,
            signal_compute_ns,
        ), records in sorted(groups.items()):
            rows = [
                record
                for record in records
                if record.get("record_type") == "measurement"
                and bool(record.get("conservation_ok"))
            ]
            failures = [
                record
                for record in records
                if record.get("record_type") == "failure"
                or (
                    record.get("record_type") == "measurement"
                    and not bool(record.get("conservation_ok"))
                )
            ]
            throughputs = [float(row["throughput_ticks_s"]) for row in rows]
            cv = 0.0
            if len(throughputs) > 1 and statistics.mean(throughputs) != 0:
                cv = statistics.stdev(throughputs) / statistics.mean(throughputs) * 100
            measured_records = [
                record
                for record in records
                if record.get("record_type") == "measurement"
            ]
            diagnostic_fields = (
                "drops",
                "duplicates",
                "route_errors",
                "missing",
                "ending_in_flight",
                "account_delivery_lost",
                "broadcast_lagged",
                "cross_batch_errors",
            )
            diagnostic_totals = {
                name: sum(int(field(row, name, 0) or 0) for row in measured_records)
                for name in diagnostic_fields
            }
            account_callback_errors = sum(
                int(row.get("_account_callback_errors", 0)) for row in measured_records
            )
            errors = sum(diagnostic_totals.values()) + account_callback_errors
            first = rows[0] if rows else None
            expected_ticks, calculated_orders, calculated_business_errors = (
                expected_case_counts(input_metadata, mode, scenario, tasks)
            )

            def stable(name: str) -> Any:
                return stable_value(rows, name) if rows else None

            def performance(value: Any) -> Any:
                return value if rows else None

            failed_runs = ",".join(
                str(record.get("run", "case")) for record in failures
            )
            failure_reasons = " | ".join(
                sorted({failure_reason(record) for record in failures})
            )
            writer.writerow(
                {
                    "framework": framework,
                    "topology": topology,
                    "mode": mode,
                    "scenario": scenario,
                    "tasks": tasks,
                    "runner_schema_version": schema_version,
                    "signal_compute_ns": signal_compute_ns,
                    "signal_compute_calls": stable("signal_compute_calls"),
                    "batch_count": stable("batch_count"),
                    "runtime_worker_threads": (
                        measurement_worker_threads(first)
                        if first is not None
                        else (input_metadata or {}).get("runtime_worker_threads")
                    ),
                    "instruments_per_task": (
                        first["instruments_per_task"]
                        if first is not None
                        else (input_metadata or {}).get("instruments_per_task")
                    ),
                    "total_instruments": (
                        tasks * int(first["instruments_per_task"])
                        if first is not None
                        else (
                            tasks * int(input_metadata["instruments_per_task"])
                            if input_metadata
                            and input_metadata.get("instruments_per_task") is not None
                            else None
                        )
                    ),
                    "messages_per_instrument": (
                        first["messages_per_instrument"]
                        if first is not None
                        else (input_metadata or {}).get("messages_per_instrument")
                    ),
                    "total_ticks": stable("total_ticks") if rows else expected_ticks,
                    "expected_orders": (
                        stable("expected_orders") if rows else calculated_orders
                    ),
                    "expected_business_errors": (
                        stable("business_errors")
                        if rows
                        else calculated_business_errors
                    ),
                    "producer_tasks": stable("producer_tasks"),
                    "trade_ingress_lanes": stable("trade_ingress_lanes"),
                    "trade_task_keys": stable("trade_task_keys"),
                    "public_cooperative_yields": stable(
                        "public_cooperative_yields"
                    ),
                    "signal_handlers": stable("signal_handlers"),
                    "trade_lanes_per_signal_handler": stable(
                        "trade_lanes_per_signal_handler"
                    ),
                    "handler_kind": stable("handler_kind"),
                    "account_ingress_tasks": stable("account_ingress_tasks"),
                    "account_ingress_lanes": stable("account_ingress_lanes"),
                    "account_ingress_capacity": stable(
                        "account_ingress_capacity"
                    ),
                    "venue_handlers": stable("venue_handlers"),
                    "order_runners": stable("order_runners"),
                    "account_broadcast_rings": stable("account_broadcast_rings"),
                    "account_topics": stable("account_topics"),
                    "account_fanout_domains": stable("account_fanout_domains"),
                    "account_handlers_per_fanout_domain": stable(
                        "account_handlers_per_fanout_domain"
                    ),
                    "account_receivers_per_ring": stable(
                        "account_receivers_per_ring"
                    ),
                    "broadcast_lag_observable": stable("broadcast_lag_observable"),
                    "account_owner_handlers": stable("account_owner_handlers"),
                    "trade_broadcast_capacity": stable("trade_broadcast_capacity"),
                    "order_execution_broadcast_capacity": stable(
                        "order_execution_broadcast_capacity"
                    ),
                    "account_order_broadcast_capacity": stable(
                        "account_order_broadcast_capacity"
                    ),
                    "expected_account_callbacks": stable(
                        "expected_account_callbacks"
                    ),
                    "account_callbacks": stable("account_callbacks"),
                    "account_observation_sources": stable(
                        "account_observation_sources"
                    ),
                    "native_owner_callbacks": stable("native_owner_callbacks"),
                    "account_non_owner_callbacks": stable(
                        "account_non_owner_callbacks"
                    ),
                    "per_signal_trade_callbacks_min": stable(
                        "per_signal_trade_callbacks_min"
                    ),
                    "per_signal_trade_callbacks_max": stable(
                        "per_signal_trade_callbacks_max"
                    ),
                    "per_signal_private_callbacks_min": stable(
                        "per_signal_private_callbacks_min"
                    ),
                    "per_signal_private_callbacks_max": stable(
                        "per_signal_private_callbacks_max"
                    ),
                    "signal_handler_model": stable("signal_handler_model"),
                    "batch_isolation_model": stable("batch_isolation_model"),
                    "private_fanout_model": stable("private_fanout_model"),
                    "account_owner_selection_model": stable(
                        "account_owner_selection_model"
                    ),
                    "public_ingress_model": stable("public_ingress_model"),
                    "private_ingress_model": stable("private_ingress_model"),
                    "overflow_model": stable("overflow_model"),
                    "measured_runs": len(rows),
                    "failure_count": len(failures),
                    "failed_runs": failed_runs,
                    "failure_reasons": failure_reasons,
                    "throughput_median_ticks_s": performance(
                        round(statistics.median(throughputs), 3) if rows else None
                    ),
                    "throughput_min_ticks_s": performance(
                        round(min(throughputs), 3) if rows else None
                    ),
                    "throughput_max_ticks_s": performance(
                        round(max(throughputs), 3) if rows else None
                    ),
                    "throughput_cv_percent": performance(round(cv, 3)),
                    "public_decode_p50_median_ns": performance(
                        latency_median(rows, "public_decode", "p50_ns")
                        if rows
                        else None
                    ),
                    "public_decode_p99_median_ns": performance(
                        latency_median(rows, "public_decode", "p99_ns")
                        if rows
                        else None
                    ),
                    "tick_to_order_p50_median_ns": performance(
                        latency_median(rows, "tick_to_order", "p50_ns")
                        if rows
                        else None
                    ),
                    "tick_to_order_p99_median_ns": performance(
                        latency_median(rows, "tick_to_order", "p99_ns")
                        if rows
                        else None
                    ),
                    "tick_to_ack_p99_median_ns": performance(
                        latency_median(rows, "tick_to_ack", "p99_ns")
                        if rows
                        else None
                    ),
                    "tick_to_fill_p50_median_ns": performance(
                        latency_median(rows, "tick_to_fill", "p50_ns")
                        if rows
                        else None
                    ),
                    "tick_to_fill_p99_median_ns": performance(
                        latency_median(rows, "tick_to_fill", "p99_ns")
                        if rows
                        else None
                    ),
                    "max_in_flight_median": performance(
                        round(median(rows, "max_in_flight")) if rows else None
                    ),
                    "task_completed_min": performance(
                        min(int(row["task_completed_min"]) for row in rows)
                        if rows
                        else None
                    ),
                    "task_completed_max": performance(
                        max(int(row["task_completed_max"]) for row in rows)
                        if rows
                        else None
                    ),
                    "private_bridge": stable("private_bridge"),
                    "conservation_failures": sum(
                        record.get("record_type") == "measurement"
                        and not bool(record.get("conservation_ok"))
                        for record in records
                    ),
                    "account_callback_errors": (
                        account_callback_errors if measured_records else None
                    ),
                    "account_delivery_lost": (
                        diagnostic_totals["account_delivery_lost"]
                        if measured_records
                        else None
                    ),
                    "broadcast_lagged": (
                        diagnostic_totals["broadcast_lagged"]
                        if measured_records
                        else None
                    ),
                    "cross_batch_errors": (
                        diagnostic_totals["cross_batch_errors"]
                        if measured_records
                        else None
                    ),
                    "errors": errors if measured_records else None,
                }
            )
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
