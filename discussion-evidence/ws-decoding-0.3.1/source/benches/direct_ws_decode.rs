use std::time::Duration;

use criterion::{
    BenchmarkGroup, Criterion, Throughput, black_box, criterion_group, criterion_main,
    measurement::WallTime,
};
use extrema_infra::direct_benchmark::*;

fn bench_pair(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    frame: &'static [u8],
    legacy: fn(&[u8]),
    preferred: fn(&[u8]),
) {
    group.throughput(Throughput::Bytes(frame.len() as u64));
    group.bench_function(format!("{name}/legacy_untagged"), |b| {
        b.iter(|| legacy(black_box(frame)));
    });
    group.bench_function(format!("{name}/preferred"), |b| {
        b.iter(|| preferred(black_box(frame)));
    });
}

fn verify_behavior() {
    let checks = behavior_report();
    let production = checks.iter().filter(|check| check.expected_equal).count();
    let boundaries = checks.len() - production;

    for check in &checks {
        assert_eq!(
            check.actual_equal, check.expected_equal,
            "unexpected behavior result for {}: {check:#?}",
            check.name
        );

        if !check.expected_equal {
            println!(
                "expected boundary: {}: {} -> {}; normalized_equal={}",
                check.name, check.legacy_variant, check.preferred_variant, check.normalized_equal
            );
        }
    }

    println!(
        "behavior preflight: {production} production/fallback cases equivalent; \
         {boundaries} documented ambiguous-shape differences confirmed"
    );
}

fn benchmark_decode_paths(c: &mut Criterion) {
    verify_behavior();

    let mut decode = c.benchmark_group("decode_only_hot");
    bench_pair(
        &mut decode,
        "binance_trade",
        BINANCE_TRADE,
        binance_trade_legacy_decode,
        binance_trade_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "binance_bbo",
        BINANCE_BBO,
        binance_bbo_legacy_decode,
        binance_bbo_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "binance_positions",
        BINANCE_POSITIONS,
        binance_positions_legacy_decode,
        binance_positions_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "binance_um_diff_depth",
        BINANCE_DIFF_DEPTH,
        binance_diff_depth_legacy_decode,
        binance_diff_depth_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "binance_cm_snapshot",
        BINANCE_CM_SNAPSHOT,
        binance_cm_snapshot_legacy_decode,
        binance_cm_snapshot_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "binance_spot_order",
        BINANCE_SPOT_ORDER,
        binance_spot_order_legacy_decode,
        binance_spot_order_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "gate_trade",
        GATE_TRADE,
        gate_trade_legacy_decode,
        gate_trade_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "gate_bbo",
        GATE_BBO,
        gate_bbo_legacy_decode,
        gate_bbo_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "gate_snapshot",
        GATE_SNAPSHOT,
        gate_snapshot_legacy_decode,
        gate_snapshot_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "gate_incremental",
        GATE_INCREMENTAL,
        gate_incremental_legacy_decode,
        gate_incremental_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "gate_spot_order",
        GATE_SPOT_ORDER,
        gate_spot_order_legacy_decode,
        gate_spot_order_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "hyperliquid_trade",
        HYPERLIQUID_TRADE,
        hyperliquid_trade_legacy_decode,
        hyperliquid_trade_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "hyperliquid_bbo",
        HYPERLIQUID_BBO,
        hyperliquid_bbo_legacy_decode,
        hyperliquid_bbo_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "hyperliquid_l2",
        HYPERLIQUID_L2,
        hyperliquid_l2_legacy_decode,
        hyperliquid_l2_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "hyperliquid_positions",
        HYPERLIQUID_POSITIONS,
        hyperliquid_positions_legacy_decode,
        hyperliquid_positions_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "hyperliquid_order",
        HYPERLIQUID_ORDER,
        hyperliquid_order_legacy_decode,
        hyperliquid_order_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "okx_trade",
        OKX_TRADE,
        okx_trade_legacy_decode,
        okx_trade_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "okx_bbo",
        OKX_BBO,
        okx_bbo_legacy_decode,
        okx_bbo_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "okx_snapshot",
        OKX_SNAPSHOT,
        okx_snapshot_legacy_decode,
        okx_snapshot_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "okx_heartbeat",
        OKX_HEARTBEAT,
        okx_heartbeat_legacy_decode,
        okx_heartbeat_preferred_decode,
    );
    bench_pair(
        &mut decode,
        "okx_order",
        OKX_ORDER,
        okx_order_legacy_decode,
        okx_order_preferred_decode,
    );
    decode.finish();

    let mut normalize = c.benchmark_group("decode_normalize_hot");
    bench_pair(
        &mut normalize,
        "binance_trade",
        BINANCE_TRADE,
        binance_trade_legacy_normalize,
        binance_trade_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "binance_bbo",
        BINANCE_BBO,
        binance_bbo_legacy_normalize,
        binance_bbo_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "binance_positions",
        BINANCE_POSITIONS,
        binance_positions_legacy_normalize,
        binance_positions_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "binance_um_diff_depth",
        BINANCE_DIFF_DEPTH,
        binance_diff_depth_legacy_normalize,
        binance_diff_depth_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "binance_cm_snapshot",
        BINANCE_CM_SNAPSHOT,
        binance_cm_snapshot_legacy_normalize,
        binance_cm_snapshot_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "binance_spot_order",
        BINANCE_SPOT_ORDER,
        binance_spot_order_legacy_normalize,
        binance_spot_order_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "gate_trade",
        GATE_TRADE,
        gate_trade_legacy_normalize,
        gate_trade_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "gate_bbo",
        GATE_BBO,
        gate_bbo_legacy_normalize,
        gate_bbo_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "gate_snapshot",
        GATE_SNAPSHOT,
        gate_snapshot_legacy_normalize,
        gate_snapshot_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "gate_incremental",
        GATE_INCREMENTAL,
        gate_incremental_legacy_normalize,
        gate_incremental_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "gate_spot_order",
        GATE_SPOT_ORDER,
        gate_spot_order_legacy_normalize,
        gate_spot_order_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "hyperliquid_trade",
        HYPERLIQUID_TRADE,
        hyperliquid_trade_legacy_normalize,
        hyperliquid_trade_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "hyperliquid_bbo",
        HYPERLIQUID_BBO,
        hyperliquid_bbo_legacy_normalize,
        hyperliquid_bbo_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "hyperliquid_l2",
        HYPERLIQUID_L2,
        hyperliquid_l2_legacy_normalize,
        hyperliquid_l2_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "hyperliquid_positions",
        HYPERLIQUID_POSITIONS,
        hyperliquid_positions_legacy_normalize,
        hyperliquid_positions_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "hyperliquid_order",
        HYPERLIQUID_ORDER,
        hyperliquid_order_legacy_normalize,
        hyperliquid_order_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "okx_trade",
        OKX_TRADE,
        okx_trade_legacy_normalize,
        okx_trade_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "okx_bbo",
        OKX_BBO,
        okx_bbo_legacy_normalize,
        okx_bbo_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "okx_snapshot",
        OKX_SNAPSHOT,
        okx_snapshot_legacy_normalize,
        okx_snapshot_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "okx_heartbeat",
        OKX_HEARTBEAT,
        okx_heartbeat_legacy_normalize,
        okx_heartbeat_preferred_normalize,
    );
    bench_pair(
        &mut normalize,
        "okx_order",
        OKX_ORDER,
        okx_order_legacy_normalize,
        okx_order_preferred_normalize,
    );
    normalize.finish();

    let mut fallback = c.benchmark_group("decode_only_fallback");
    bench_pair(
        &mut fallback,
        "binance_ack",
        BINANCE_ACK,
        binance_trade_legacy_decode,
        binance_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "binance_error",
        BINANCE_NESTED_ERROR,
        binance_trade_legacy_decode,
        binance_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "binance_positions_ack",
        BINANCE_ACK,
        binance_positions_legacy_decode,
        binance_positions_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "gate_ack",
        GATE_ACK,
        gate_trade_legacy_decode,
        gate_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "gate_error",
        GATE_ERROR,
        gate_trade_legacy_decode,
        gate_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "gate_single_ack",
        GATE_ACK,
        gate_bbo_legacy_decode,
        gate_bbo_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "hyperliquid_pong",
        HYPERLIQUID_PONG,
        hyperliquid_trade_legacy_decode,
        hyperliquid_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "hyperliquid_error",
        HYPERLIQUID_ERROR,
        hyperliquid_trade_legacy_decode,
        hyperliquid_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "hyperliquid_positions_pong",
        HYPERLIQUID_PONG,
        hyperliquid_positions_legacy_decode,
        hyperliquid_positions_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "hyperliquid_bbo_pong",
        HYPERLIQUID_PONG,
        hyperliquid_bbo_legacy_decode,
        hyperliquid_bbo_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "hyperliquid_l2_pong",
        HYPERLIQUID_PONG,
        hyperliquid_l2_legacy_decode,
        hyperliquid_l2_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "okx_ack",
        OKX_ACK,
        okx_trade_legacy_decode,
        okx_trade_preferred_decode,
    );
    bench_pair(
        &mut fallback,
        "okx_error",
        OKX_ERROR,
        okx_trade_legacy_decode,
        okx_trade_preferred_decode,
    );
    fallback.finish();
}

fn config() -> Criterion {
    Criterion::default()
        .without_plots()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(100)
        .nresamples(100_000)
        .noise_threshold(0.02)
        .significance_level(0.05)
}

criterion_group! {
    name = benches;
    config = config();
    targets = benchmark_decode_paths
}
criterion_main!(benches);
