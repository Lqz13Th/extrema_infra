# Extrema Infra 0.3.1 WebSocket decoder evidence

This bundle backs the four-exchange channel-aware decoder Discussion. The
production base is the published `extrema_infra 0.3.1` source at commit
`3760565`. Production exchange code was not modified for measurement.

## Contents

- `source/release-0.3.1-benchmark.patch`: tracked benchmark-only changes to
  `Cargo.toml` and `src/lib.rs`.
- `source/benches/direct_ws_decode.rs`: untracked Criterion entry point.
- `source/src/direct_benchmark.rs`: fixtures, behavior preflight, legacy
  full-envelope comparators, production preferred decoders, and normalization
  calls.
- `results/report.md`: generated human-readable report.
- `results/results.csv` and `results/results.json`: generated machine-readable
  summaries.
- `results/criterion-runs-01-03.tar.gz`: all retained Criterion output from the
  three formal process runs. It contains 330 formal estimates under the same
  110 benchmark keys per run.
- `results/summarize.mjs`: summarizer used to produce the report and tables.

The 50-case behavior corpus is embedded in `source/src/direct_benchmark.rs`.
It contains 48 expected-equivalent cases and two deliberately ambiguous
synthetic cases whose channel-context differences are documented in the
Discussion and report.

## Reconstruction

```bash
git clone https://github.com/Lqz13Th/extrema_infra.git
cd extrema_infra
git checkout 3760565
git apply /path/to/source/release-0.3.1-benchmark.patch
cp /path/to/source/benches/direct_ws_decode.rs benches/
cp /path/to/source/src/direct_benchmark.rs src/

cargo test --offline --features bench-internal direct_benchmark -- --nocapture
cargo bench --offline --bench direct_ws_decode
```

The published measurements used Rust 1.97.1, Criterion 0.5.1, one-second
warm-up, three-second measurement, 100 samples, and three independent complete
process runs on an Apple M5 Pro host with 15 logical CPUs and 48 GiB memory.

## Comparison boundary

The `legacy` side is a benchmark comparator reconstructed on the same 0.3.1
schemas and in the same benchmark binary:

```text
legacy:    raw bytes -> current full exchange envelope -> optional normalize
preferred: raw bytes -> production channel decoder      -> optional normalize
```

It is not an old release binary or a historical checkout. This design isolates
decoder selection from unrelated schema and normalization changes.

Fixtures are static, production-shaped examples. They are not claimed to be
verbatim live WebSocket captures. Socket/TLS I/O, Tokio routing, Strategy
callbacks, order handling, and venue latency are outside the timer.

