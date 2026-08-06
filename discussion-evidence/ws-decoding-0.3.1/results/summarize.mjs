#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname));
const runNames = process.argv.slice(2);

if (runNames.length === 0) {
  throw new Error("usage: node summarize.mjs run-01 [run-02 ...]");
}

const groups = new Set([
  "decode_only_hot",
  "decode_normalize_hot",
  "decode_only_fallback",
]);

const scenarioLabels = {
  binance_trade: "UM aggregate trade",
  binance_bbo: "UM BBO / bookTicker",
  binance_positions: "UM account positions",
  binance_um_diff_depth: "UM incremental depth",
  binance_cm_snapshot: "CM partial-depth snapshot",
  binance_spot_order: "Spot account order",
  gate_trade: "Futures trades",
  gate_bbo: "Futures BBO",
  gate_snapshot: "Futures book snapshot",
  gate_incremental: "Futures incremental book",
  gate_spot_order: "Spot account order",
  hyperliquid_trade: "Trades",
  hyperliquid_bbo: "BBO",
  hyperliquid_l2: "L2 book (10 + 10 levels)",
  hyperliquid_positions: "Account positions",
  hyperliquid_order: "Account order",
  okx_trade: "Trades",
  okx_bbo: "BBO / bbo-tbt",
  okx_snapshot: "Books snapshot",
  okx_heartbeat: "Books heartbeat update",
  okx_order: "Account order",
  binance_ack: "Trade decoder: subscription ACK",
  binance_error: "Trade decoder: business error",
  binance_positions_ack: "Positions decoder: subscription ACK",
  gate_ack: "Batch decoder: subscription ACK",
  gate_error: "Batch decoder: business error",
  gate_single_ack: "Single decoder: subscription ACK",
  hyperliquid_pong: "Trade decoder: pong",
  hyperliquid_error: "Trade decoder: business error",
  hyperliquid_positions_pong: "Positions decoder: pong",
  hyperliquid_bbo_pong: "BBO decoder: pong",
  hyperliquid_l2_pong: "L2 decoder: pong",
  okx_ack: "Trade decoder: subscription ACK",
  okx_error: "Trade decoder: business error",
};

const privateScenarios = new Set([
  "binance_positions",
  "binance_spot_order",
  "gate_spot_order",
  "hyperliquid_positions",
  "hyperliquid_order",
  "okx_order",
]);

const exchangeLabels = {
  binance: "Binance",
  gate: "Gate",
  hyperliquid: "Hyperliquid",
  okx: "OKX",
};

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
}

function round(value, digits = 2) {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

function readRun(runName) {
  const runDir = path.join(root, runName);
  if (!fs.existsSync(runDir)) {
    throw new Error(`missing run directory: ${runDir}`);
  }

  const values = new Map();
  for (const group of fs.readdirSync(runDir)) {
    if (!groups.has(group)) continue;
    const groupDir = path.join(runDir, group);
    for (const entry of fs.readdirSync(groupDir)) {
      const newDir = path.join(groupDir, entry, "new");
      const benchmarkPath = path.join(newDir, "benchmark.json");
      const estimatesPath = path.join(newDir, "estimates.json");
      if (!fs.existsSync(benchmarkPath) || !fs.existsSync(estimatesPath)) continue;

      const benchmark = JSON.parse(fs.readFileSync(benchmarkPath, "utf8"));
      const estimates = JSON.parse(fs.readFileSync(estimatesPath, "utf8"));
      const parts = benchmark.function_id.split("/");
      const variant = parts.pop();
      const scenario = parts.join("/");
      const key = `${group}/${scenario}`;
      const value = values.get(key) ?? {
        group,
        scenario,
        bytes: benchmark.throughput.Bytes,
      };
      value[variant] = estimates.median.point_estimate;
      values.set(key, value);
    }
  }

  for (const [key, value] of values) {
    if (value.legacy_untagged === undefined || value.preferred === undefined) {
      throw new Error(`${runName}: incomplete pair ${key}`);
    }
  }
  return values;
}

const runs = runNames.map((runName) => ({ runName, values: readRun(runName) }));
const expectedKeys = [...runs[0].values.keys()].sort();
for (const run of runs.slice(1)) {
  const keys = [...run.values.keys()].sort();
  if (JSON.stringify(keys) !== JSON.stringify(expectedKeys)) {
    throw new Error(`${run.runName}: benchmark set differs from ${runNames[0]}`);
  }
}

const pairs = expectedKeys.map((key) => {
  const first = runs[0].values.get(key);
  const exchange = first.scenario.split("_")[0];
  const runValues = runs.map(({ runName, values }) => {
    const value = values.get(key);
    const deltaPct = (value.preferred / value.legacy_untagged - 1) * 100;
    return {
      run: runName,
      legacyNs: value.legacy_untagged,
      preferredNs: value.preferred,
      deltaPct,
      speedup: value.legacy_untagged / value.preferred,
    };
  });
  const legacyNs = median(runValues.map((value) => value.legacyNs));
  const preferredNs = median(runValues.map((value) => value.preferredNs));
  return {
    group: first.group,
    traffic: first.group.endsWith("fallback") ? "fallback" : "hot",
    boundary:
      first.group === "decode_normalize_hot" ? "decode+normalize" : "decode-only",
    exchange,
    exchangeLabel: exchangeLabels[exchange],
    channelClass:
      first.scenario === "okx_heartbeat"
        ? "in-band heartbeat"
        : privateScenarios.has(first.scenario)
          ? "private data"
          : first.group.endsWith("fallback")
            ? "fallback/control"
            : "public data",
    scenario: first.scenario,
    scenarioLabel: scenarioLabels[first.scenario] ?? first.scenario,
    bytes: first.bytes,
    runs: runValues,
    legacyNs,
    preferredNs,
    deltaPct: median(runValues.map((value) => value.deltaPct)),
    speedup: median(runValues.map((value) => value.speedup)),
    absoluteDeltaNs: median(
      runValues.map((value) => value.preferredNs - value.legacyNs),
    ),
  };
});

const hotPairs = pairs.filter((pair) => pair.traffic === "hot");
const exchangeSummaries = [];
for (const boundary of ["decode-only", "decode+normalize"]) {
  for (const exchange of Object.keys(exchangeLabels)) {
    const selected = hotPairs.filter(
      (pair) => pair.boundary === boundary && pair.exchange === exchange,
    );
    const geometricSpeedup = Math.exp(
      selected.reduce((sum, pair) => sum + Math.log(pair.speedup), 0) / selected.length,
    );
    const deltas = selected.map((pair) => pair.deltaPct);
    exchangeSummaries.push({
      boundary,
      exchange,
      exchangeLabel: exchangeLabels[exchange],
      scenarios: selected.length,
      medianScenarioDeltaPct: median(deltas),
      minScenarioDeltaPct: Math.min(...deltas),
      maxScenarioDeltaPct: Math.max(...deltas),
      geometricSpeedup,
    });
  }
}

const output = {
  generatedAt: new Date().toISOString(),
  source: {
    repository: "extrema_infra",
    version: "0.3.1",
    commit: "3760565",
    benchmark: "direct_ws_decode",
  },
  environment: {
    host: "Apple M5 Pro (15 cores, 48 GB)",
    os: "macOS 26.5.2 (25F84)",
    rustc: "1.97.1 (8bab26f4f 2026-07-14)",
    cargo: "1.97.1 (c980f4866 2026-06-30)",
  },
  method: {
    runNames,
    processRuns: runs.length,
    statistic: "median of independent Criterion process-run medians",
    pairStatistic: "median of within-process legacy/preferred ratios and deltas",
    warmupSeconds: 1,
    measurementSeconds: 3,
    samplesPerBenchmark: 100,
    criterionResamples: 100000,
    pairOrder: "legacy_untagged then preferred",
    traceSubscriber: false,
    command: "cargo bench --offline --features bench-internal --bench direct_ws_decode",
    excludedDirectories: ["baseline-11-cases"],
  },
  behavior: {
    expectedEquivalentCases: 48,
    documentedSyntheticAmbiguities: 2,
  },
  counts: {
    criterionFunctionsPerRun: expectedKeys.length * 2,
    benchmarkPairsPerRun: expectedKeys.length,
    hotScenarios: new Set(hotPairs.map((pair) => pair.scenario)).size,
    hotPairs: hotPairs.length,
    fallbackScenarios: pairs.filter((pair) => pair.traffic === "fallback").length,
  },
  exchangeSummaries,
  pairs,
};

fs.writeFileSync(path.join(root, "results.json"), `${JSON.stringify(output, null, 2)}\n`);

const csvHeader = [
  "group",
  "traffic",
  "boundary",
  "exchange",
  "channel_class",
  "scenario",
  "bytes",
  ...runNames.flatMap((runName) => [
    `${runName}_legacy_ns`,
    `${runName}_preferred_ns`,
    `${runName}_delta_pct`,
  ]),
  "legacy_median_ns",
  "preferred_median_ns",
  "absolute_delta_ns",
  "delta_pct",
  "speedup",
];
const csvRows = pairs.map((pair) => [
  pair.group,
  pair.traffic,
  pair.boundary,
  pair.exchange,
  pair.channelClass,
  pair.scenario,
  pair.bytes,
  ...pair.runs.flatMap((run) => [
    round(run.legacyNs, 6),
    round(run.preferredNs, 6),
    round(run.deltaPct, 6),
  ]),
  round(pair.legacyNs, 6),
  round(pair.preferredNs, 6),
  round(pair.absoluteDeltaNs, 6),
  round(pair.deltaPct, 6),
  round(pair.speedup, 6),
]);
fs.writeFileSync(
  path.join(root, "results.csv"),
  [csvHeader, ...csvRows].map((row) => row.join(",")).join("\n") + "\n",
);

function ns(value) {
  return value >= 1000 ? `${(value / 1000).toFixed(3)} us` : `${value.toFixed(2)} ns`;
}

function pct(value) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}%`;
}

const markdown = [];
markdown.push("# Extrema Infra 0.3.1 WebSocket decode matrix");
markdown.push("");
markdown.push(
  "Source: release `0.3.1` at `3760565`, with benchmark-only harness additions. The production " +
    "exchange decoders and normalizers were not modified for this measurement.",
);
markdown.push("");
markdown.push(
  `Latency cells use the median of ${runs.length} independent Criterion process-run medians. ` +
    "Absolute deltas, changes, and speedups are paired within each process run before taking their median. " +
    "Each function used 1 s warm-up, 3 s measurement, and 100 samples.",
);
markdown.push("");
markdown.push(
  `Matrix: ${output.counts.hotScenarios} hot scenarios, two timed boundaries, ` +
    `${output.counts.fallbackScenarios} fallback scenarios, ` +
    `${output.counts.criterionFunctionsPerRun} Criterion functions per run.`,
);
markdown.push("");
markdown.push(
  "Only `run-01`, `run-02`, and `run-03` contribute to this report. The retained " +
    "`baseline-11-cases` directory is an earlier subset and is excluded.",
);
markdown.push("");
markdown.push(
  "Host: Apple M5 Pro (15 cores, 48 GB), macOS 26.5.2, Rust 1.97.1. Frames are static, " +
    "production-shaped fixtures; this run did not capture a live exchange socket.",
);
markdown.push("");
markdown.push("Behavior preflight: 48 expected-equivalent cases passed; two deliberately ambiguous synthetic shapes produced the documented channel-context selection differences.");
markdown.push("");
markdown.push("## Coverage");
markdown.push("");
markdown.push("| Exchange | Timed preferred-success scenarios | Known fixture gaps |");
markdown.push("| --- | --- | --- |");
markdown.push("| Binance | UM trades, BBO, positions, incremental depth; CM snapshot; Spot order | UM account orders, balance/position, candles, snapshot; CM BBO and incremental depth |");
markdown.push("| Gate | Futures trades, BBO, snapshot, incremental book; Spot order | Futures account orders, positions, candles |");
markdown.push("| Hyperliquid | Trades, BBO, L2 book, positions, order | None among current runner families |");
markdown.push("| OKX | Trades, BBO, books snapshot, books empty-update heartbeat, order | Balance/position, positions, books5 snapshot, non-empty incremental book update |");
markdown.push("");
markdown.push("## Hot-path exchange summary");
markdown.push("");
markdown.push("The summary is unweighted across measured scenarios; it is not a production traffic mix.");
markdown.push("");
markdown.push("| Boundary | Exchange | Scenarios | Geometric speedup | Median latency change | Scenario range |");
markdown.push("| --- | --- | ---: | ---: | ---: | ---: |");
for (const summary of exchangeSummaries) {
  markdown.push(
    `| ${summary.boundary} | ${summary.exchangeLabel} | ${summary.scenarios} | ` +
      `${summary.geometricSpeedup.toFixed(2)}x | ${pct(summary.medianScenarioDeltaPct)} | ` +
      `${pct(summary.minScenarioDeltaPct)} to ${pct(summary.maxScenarioDeltaPct)} |`,
  );
}

markdown.push("");
markdown.push("## Hot-path detail");
markdown.push("");
markdown.push(
  "| Exchange | Class | Channel fixture | Bytes | Decode legacy | Decode preferred | Decode change | " +
    "Decode+normalize legacy | Decode+normalize preferred | Decode+normalize change |",
);
markdown.push("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
const hotScenarios = [...new Set(hotPairs.map((pair) => pair.scenario))];
for (const scenario of hotScenarios) {
  const decode = hotPairs.find(
    (pair) => pair.scenario === scenario && pair.boundary === "decode-only",
  );
  const normalize = hotPairs.find(
    (pair) => pair.scenario === scenario && pair.boundary === "decode+normalize",
  );
  markdown.push(
    `| ${decode.exchangeLabel} | ${decode.channelClass} | ${decode.scenarioLabel} | ${decode.bytes} | ` +
      `${ns(decode.legacyNs)} | ${ns(decode.preferredNs)} | ${pct(decode.deltaPct)} | ` +
      `${ns(normalize.legacyNs)} | ${ns(normalize.preferredNs)} | ${pct(normalize.deltaPct)} |`,
  );
}

markdown.push("");
markdown.push("## Fallback decode-only detail");
markdown.push("");
markdown.push("| Exchange | Fallback fixture | Bytes | Legacy | Preferred+fallback | Extra latency | Change |");
markdown.push("| --- | --- | ---: | ---: | ---: | ---: | ---: |");
for (const pair of pairs.filter((value) => value.traffic === "fallback")) {
  markdown.push(
    `| ${pair.exchangeLabel} | ${pair.scenarioLabel} | ${pair.bytes} | ${ns(pair.legacyNs)} | ` +
      `${ns(pair.preferredNs)} | +${ns(pair.absoluteDeltaNs)} | ${pct(pair.deltaPct)} |`,
  );
}

markdown.push("");
markdown.push("## Scope");
markdown.push("");
markdown.push(
  "The timer starts with in-memory raw bytes and stops after either typed decode or existing `IntoWsData` " +
    "normalization. It excludes socket/TLS I/O, Tokio routing, task fan-out, strategy callbacks, order " +
    "submission, and venue latency.",
);
markdown.push("");
markdown.push(
  "Each pair ran legacy first and preferred second. No TRACE subscriber was installed, so fallback results " +
    "include the disabled trace callsite but not trace recording or export. Criterion `Throughput::Bytes` was " +
    "configured only to derive byte-rate from single-thread latency; this was not a concurrent throughput test.",
);
markdown.push("");
fs.writeFileSync(path.join(root, "report.md"), `${markdown.join("\n")}\n`);

console.log(
  `wrote results.json, results.csv, and report.md from ${runs.length} run(s); ` +
    `${expectedKeys.length * 2} Criterion functions per run`,
);
