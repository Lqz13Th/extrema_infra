# Extrema Infra 0.3.1 WebSocket decode matrix

Source: release `0.3.1` at `3760565`, with benchmark-only harness additions. The production exchange decoders and normalizers were not modified for this measurement.

Latency cells use the median of 3 independent Criterion process-run medians. Absolute deltas, changes, and speedups are paired within each process run before taking their median. Each function used 1 s warm-up, 3 s measurement, and 100 samples.

Matrix: 21 hot scenarios, two timed boundaries, 13 fallback scenarios, 110 Criterion functions per run.

Only `run-01`, `run-02`, and `run-03` contribute to this report. The retained `baseline-11-cases` directory is an earlier subset and is excluded.

Host: Apple M5 Pro (15 cores, 48 GB), macOS 26.5.2, Rust 1.97.1. Frames are static, production-shaped fixtures; this run did not capture a live exchange socket.

Behavior preflight: 48 expected-equivalent cases passed; two deliberately ambiguous synthetic shapes produced the documented channel-context selection differences.

## Coverage

| Exchange | Timed preferred-success scenarios | Known fixture gaps |
| --- | --- | --- |
| Binance | UM trades, BBO, positions, incremental depth; CM snapshot; Spot order | UM account orders, balance/position, candles, snapshot; CM BBO and incremental depth |
| Gate | Futures trades, BBO, snapshot, incremental book; Spot order | Futures account orders, positions, candles |
| Hyperliquid | Trades, BBO, L2 book, positions, order | None among current runner families |
| OKX | Trades, BBO, books snapshot, books empty-update heartbeat, order | Balance/position, positions, books5 snapshot, non-empty incremental book update |

## Hot-path exchange summary

The summary is unweighted across measured scenarios; it is not a production traffic mix.

| Boundary | Exchange | Scenarios | Geometric speedup | Median latency change | Scenario range |
| --- | --- | ---: | ---: | ---: | ---: |
| decode-only | Binance | 6 | 1.86x | -46.0% | -55.4% to -36.5% |
| decode-only | Gate | 5 | 1.88x | -50.1% | -54.7% to -32.1% |
| decode-only | Hyperliquid | 5 | 2.45x | -55.1% | -72.6% to -49.6% |
| decode-only | OKX | 5 | 1.63x | -37.6% | -42.8% to -36.5% |
| decode+normalize | Binance | 6 | 1.61x | -36.6% | -49.2% to -32.1% |
| decode+normalize | Gate | 5 | 1.65x | -41.3% | -45.7% to -28.9% |
| decode+normalize | Hyperliquid | 5 | 2.06x | -49.6% | -61.7% to -40.2% |
| decode+normalize | OKX | 5 | 1.48x | -31.6% | -35.8% to -30.4% |

## Hot-path detail

| Exchange | Class | Channel fixture | Bytes | Decode legacy | Decode preferred | Decode change | Decode+normalize legacy | Decode+normalize preferred | Decode+normalize change |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Binance | public data | UM BBO / bookTicker | 141 | 394.09 ns | 216.04 ns | -45.2% | 539.24 ns | 365.40 ns | -32.2% |
| Binance | public data | CM partial-depth snapshot | 191 | 606.25 ns | 319.65 ns | -46.9% | 754.77 ns | 474.67 ns | -37.1% |
| Binance | private data | UM account positions | 183 | 779.81 ns | 348.09 ns | -55.4% | 901.56 ns | 457.88 ns | -49.2% |
| Binance | private data | Spot account order | 181 | 602.07 ns | 382.35 ns | -36.5% | 655.27 ns | 445.04 ns | -32.1% |
| Binance | public data | UM aggregate trade | 132 | 381.17 ns | 209.34 ns | -45.1% | 490.37 ns | 303.38 ns | -38.1% |
| Binance | public data | UM incremental depth | 202 | 673.33 ns | 357.89 ns | -46.8% | 837.27 ns | 539.80 ns | -36.1% |
| Gate | public data | Futures BBO | 157 | 582.26 ns | 290.57 ns | -50.1% | 773.04 ns | 449.93 ns | -41.3% |
| Gate | public data | Futures incremental book | 178 | 684.70 ns | 311.99 ns | -54.4% | 835.03 ns | 452.90 ns | -45.7% |
| Gate | public data | Futures book snapshot | 193 | 819.61 ns | 371.65 ns | -54.7% | 965.32 ns | 532.26 ns | -44.9% |
| Gate | private data | Spot account order | 371 | 955.86 ns | 653.80 ns | -32.1% | 1.014 us | 717.93 ns | -28.9% |
| Gate | public data | Futures trades | 218 | 517.06 ns | 306.92 ns | -39.9% | 625.18 ns | 410.91 ns | -33.9% |
| Hyperliquid | public data | BBO | 136 | 1.089 us | 297.71 ns | -72.6% | 1.340 us | 522.42 ns | -61.7% |
| Hyperliquid | public data | L2 book (10 + 10 levels) | 759 | 5.014 us | 2.020 us | -59.7% | 5.338 us | 2.306 us | -56.4% |
| Hyperliquid | private data | Account order | 232 | 791.91 ns | 394.02 ns | -49.6% | 967.21 ns | 578.29 ns | -40.2% |
| Hyperliquid | private data | Account positions | 219 | 905.74 ns | 411.04 ns | -54.7% | 1.072 us | 538.71 ns | -49.6% |
| Hyperliquid | public data | Trades | 121 | 519.96 ns | 233.22 ns | -55.1% | 646.62 ns | 349.01 ns | -46.5% |
| OKX | public data | BBO / bbo-tbt | 176 | 729.66 ns | 465.96 ns | -36.5% | 888.50 ns | 618.13 ns | -30.4% |
| OKX | in-band heartbeat | Books heartbeat update | 143 | 480.30 ns | 276.29 ns | -42.8% | 577.26 ns | 369.92 ns | -35.8% |
| OKX | private data | Account order | 392 | 1.162 us | 735.75 ns | -36.7% | 1.263 us | 863.72 ns | -31.6% |
| OKX | public data | Books snapshot | 228 | 866.70 ns | 543.91 ns | -37.6% | 1.042 us | 721.52 ns | -30.7% |
| OKX | public data | Trades | 174 | 558.52 ns | 335.50 ns | -39.9% | 656.69 ns | 445.58 ns | -32.6% |

## Fallback decode-only detail

| Exchange | Fallback fixture | Bytes | Legacy | Preferred+fallback | Extra latency | Change |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Binance | Trade decoder: subscription ACK | 36 | 360.31 ns | 502.96 ns | +141.00 ns | +39.1% |
| Binance | Trade decoder: business error | 62 | 442.90 ns | 611.18 ns | +160.15 ns | +36.2% |
| Binance | Positions decoder: subscription ACK | 36 | 360.32 ns | 494.83 ns | +134.51 ns | +36.8% |
| Gate | Batch decoder: subscription ACK | 78 | 538.10 ns | 774.34 ns | +239.25 ns | +44.7% |
| Gate | Batch decoder: business error | 91 | 477.73 ns | 697.26 ns | +218.63 ns | +46.1% |
| Gate | Single decoder: subscription ACK | 78 | 545.86 ns | 752.27 ns | +209.07 ns | +38.5% |
| Hyperliquid | BBO decoder: pong | 18 | 176.12 ns | 317.17 ns | +139.49 ns | +79.2% |
| Hyperliquid | Trade decoder: business error | 52 | 643.77 ns | 839.95 ns | +196.18 ns | +30.7% |
| Hyperliquid | L2 decoder: pong | 18 | 176.72 ns | 316.96 ns | +140.23 ns | +79.4% |
| Hyperliquid | Trade decoder: pong | 18 | 177.55 ns | 321.85 ns | +144.30 ns | +81.3% |
| Hyperliquid | Positions decoder: pong | 18 | 178.62 ns | 320.00 ns | +140.65 ns | +79.1% |
| OKX | Trade decoder: subscription ACK | 73 | 335.56 ns | 559.23 ns | +225.05 ns | +67.9% |
| OKX | Trade decoder: business error | 104 | 421.97 ns | 686.69 ns | +265.06 ns | +62.9% |

## Scope

The timer starts with in-memory raw bytes and stops after either typed decode or existing `IntoWsData` normalization. It excludes socket/TLS I/O, Tokio routing, task fan-out, strategy callbacks, order submission, and venue latency.

Each pair ran legacy first and preferred second. No TRACE subscriber was installed, so fallback results include the disabled trace callsite but not trace recording or export. Criterion `Throughput::Bytes` was configured only to derive byte-rate from single-thread latency; this was not a concurrent throughput test.

