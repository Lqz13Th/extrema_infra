# Three-framework benchmark evidence

This bundle backs the Extrema Infra, Barter-rs, and NautilusTrader comparison
Discussion. It publishes the actual dirty benchmark source states rather than
only their base commits and partial source hashes.

## Source provenance

| Repository | Public base commit | Reconstruction material |
| --- | --- | --- |
| Extrema Infra | `0384457670dd4d4db6729ec1af83aa3c902b71a6` | `patches/extrema_infra-0384457.patch` plus `source-archives/extrema-infra-untracked.tar.gz` |
| Barter-rs | `33e56188e2095781331f85aa3d7f88e251eec65a` | `patches/barter-rs-33e5618.patch` plus `source-archives/barter-rs-untracked.tar.gz` |
| NautilusTrader | `0975556b07d17adb9ad0508763730e0cc91bd58c` | `patches/nautilus-trader-0975556.patch` plus `source-archives/nautilus-trader-untracked.tar.gz` |

These are older dirty benchmark bases. They are not Extrema Infra 0.3.1 or the
current NautilusTrader `develop` product snapshot. Product capability review
and performance provenance are intentionally separate in the Discussion.

For each repository, check out the public base, apply its tracked patch, then
extract its untracked archive at the repository root:

```bash
git checkout <base-commit>
git apply /path/to/patches/<repository>.patch
tar -xzf /path/to/source-archives/<repository>-untracked.tar.gz
```

The patches include every tracked modification present in the benchmark
worktrees. The archives include every untracked benchmark source used by the
published parser, tick-to-trade, and zero-network live-shaped runners. Cargo
build output is excluded.

## Runner schemas and retained results

- Schema 5 zero-compute formal runs:
  `live-pipeline-native-ws5-strategy20-stress-m64-2026-08-05.jsonl` and
  `live-pipeline-native-ws5-strategy20-mixed-m256-2026-08-05.jsonl`.
  Each has two warm-ups and seven measured repetitions.
- Schema 5 overload probe:
  `live-pipeline-native-ws5-strategy20-overload-stress-m256-2026-08-05.jsonl`.
- Schema 6 matched-configuration compute comparison: the four `compute0` and
  `compute100us` JSONL files plus their summary CSVs. These are independent
  batches, not interleaved or per-run paired samples.
- Standalone Binance parser/normalizer context:
  `binance-public-ws-native-2026-08-05.jsonl`, containing all three Criterion
  process-run estimates and metadata.
- Human-readable retained reports:
  `native-ws5-strategy20-benchmark-2026-08-05.md` and
  `native-ws5-strategy20-compute100us-benchmark-2026-08-05.md`.

`runners/run_live_pipeline_capacity.py` records the workload contract,
conservation gates, runtime configuration, selected source digests, and command
execution. `runners/LIVE_PIPELINE_SPEC.md` describes the common logical
topology. The co-located three-channel Criterion harness is under
`runners/binance_public_ws_crossbench/`.

## Interpretation boundary

The zero-network live-shaped workload uses 20 logical public ingress tasks,
five symbols per ingress, 20 signal handlers, one raw AccountOrder ingress, an
in-memory benchmark venue, raw ACK/FILL/reject decoding, and 20 logical private
observations per frame. It excludes physical WebSockets, TLS/kernel I/O, real
venue latency, reconnect, authentication, and persistence.

All runners configure an outer 16-worker Tokio runtime, but effective Strategy
callback concurrency is framework-native: Extrema has independent Strategy
tasks, while the tested Barter Engine and Nautilus LiveNode dispatch Strategy
callbacks serially. The bundle measures those scheduling/domain paths; it does
not claim equal CPU instruction counts or a universal framework ranking.

Latency rows in the Discussion use the median of the seven run-level
percentiles, not a percentile pooled across every event from all runs. Barter's
schema 5 stress throughput was bimodal with a 77.963% CV and is retained as an
unstable observation, not a stable service-rate ranking.

