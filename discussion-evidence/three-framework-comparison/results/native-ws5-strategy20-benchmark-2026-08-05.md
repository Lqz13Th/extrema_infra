# 三框架原生结构 Benchmark：20 WS x 5 Symbols x 20 Strategies

日期：2026-08-05（Asia/Shanghai）

## 结论

本轮固定相同的逻辑输入与业务守恒，但保留三套框架各自的原生物理结构：

```text
20 logical Public WS ingress x 5 symbols = 100 symbols
  -> 20 signal handlers, each owns one WS group / 5 symbols
  -> one shared AccountOrder ingress and fan-out domain
  -> each raw private frame is decoded once
  -> 20 observations = 1 canonical owner + 19 observers
```

- `stress`（每个 tick 都下单）由 Extrema 领先：`107,541 tick/s`，约为 Barter 的 `2.03x`、Nautilus 的 `6.21x`。
- `mixed_live`（每个 symbol 每 20 tick 下单；M=256 下实际为 12/256=`4.6875%`）由 Barter 领先：`1,115,874 tick/s`，约为 Extrema 的 `1.96x`、Nautilus 的 `3.16x`。
- 排名反转符合架构：Extrema 的 20 个原生 Strategy task 能跨 Tokio workers 并行；Barter 的单 Engine + 同步 composite Strategy 在轻下单路径调度最省，但每 tick 下单后，私有事件的 20 路同步 fan-out 成为瓶颈。
- Nautilus 的 20 个 Strategy 是真实 Strategy，但同处一个 LiveNode/current-thread engine，callback 同步串行；完整 domain routing、execution engine 和 19 个 CustomData observer 的成本使它在本拓扑最慢。
- 单帧 Public WS decode/normalize 不是框架总排名：Trade/BBO 是 Nautilus 最快，Depth frame 是 Barter 最快；全链路排名由策略调度、下单和 private fan-out 决定。

这里的 `WS` 是零网络、预构造 raw JSON 的 logical ingress task，不是 20 条真实 TLS/socket 连接。

## 原生物理结构

| 框架 | Signal 结构 | Private fan-out | 阻塞与容量行为 |
| --- | --- | --- | --- |
| Extrema Infra | 20 个原生 Strategy task；每个订阅一条独立 Trade TaskKey，处理固定 5 symbols；共享 16-worker Tokio runtime | 一个全局 `TaskChannels` AccountOrder broadcast ring，20 个 Strategy receiver，共享 decoded `Arc` | 单个 Strategy 阻塞不会同步卡住其余 callback，但会占用 worker；慢 receiver 可能产生 broadcast lag，lag 是硬失败 |
| Barter-rs | 一个原生 Engine + 一个原生 composite Strategy；内部 20 个 logical handler 同步调用 | 同一个 `AlgoStrategy` callback 内顺序完成 20 次 logical observation；没有物理 broadcast ring | 一个 handler 阻塞会卡住后续 handler 和整个 Engine；raw private ingress 用 `send().await` 回压 |
| Nautilus Trader | 一个 LiveNode、一个 current-thread Tokio runtime、20 个真实 Strategy；LiveNode engine 同步分发 callback | 下单 Strategy 收一个 native order callback；另外 19 个 Strategy 收 owner-specific `CustomData` observation | 一个 callback 阻塞会卡住这个 LiveNode 的其他消息；bounded private ingress 用 `try_send`，满时 fail-fast |

因此，三者相同的是输入、策略分组、订单数、private frame 数和 20 路观察守恒；不同的是实现这些工作的 task、loop、channel 和 callback 数量。不能把 Barter 的 20 个 logical handler 说成 20 个并行 task，也不能把 Nautilus 的 20 个真实 Strategy 说成 20 个并行 engine。

## 全链路计时边界

计时从 public raw frame decode 前开始，到全部 Trade 决策、订单终态和每个 private frame 的 20 路观察完成后结束。包含：

- raw public JSON decode + native normalize；
- 框架原生数据路由与策略 callback；
- 原生 order submission 路径；
- benchmark-local in-memory venue；
- raw ACK/FILL/reject decode 与 native terminal callback；
- 每个 private frame 的 1 owner + 19 observer。

不包含 TLS、socket、内核网络、真实交易所延迟、重连/backoff、sleep、持久化和 PnL reporting。

结果为 2 次 warmup + 7 次 measured 的 wall-throughput 中位数。CV 使用 7 次结果的样本标准差除以均值。

## 吞吐结果

### Success

| Mode | 输入规模 | Framework | Median tick/s | CV | 排名 |
| --- | --- | --- | ---: | ---: | ---: |
| stress | M=64；6,400 ticks/orders | Extrema | **107,541** | 0.865% | 1 |
| stress | M=64；6,400 ticks/orders | Barter | 53,071 | 77.963% | 2 |
| stress | M=64；6,400 ticks/orders | Nautilus | 17,330 | 0.778% | 3 |
| mixed_live | M=256；25,600 ticks、1,200 orders | Barter | **1,115,874** | 10.746% | 1 |
| mixed_live | M=256；25,600 ticks、1,200 orders | Extrema | 570,106 | 3.086% | 2 |
| mixed_live | M=256；25,600 ticks、1,200 orders | Nautilus | 352,789 | 0.964% | 3 |

Barter `stress/success` 明显双峰：5 次约 `49k-54k tick/s`，另外 2 次约 `203k-206k`。中位数能描述本次多数运行，但 `77.963% CV` 表明这不是稳定服务率，不能拿单次 206k 当结论。

### Business Error

每第 10 个订单注入交易所业务 reject：

| Mode | Framework | Median tick/s | CV | Rejects |
| --- | --- | ---: | ---: | ---: |
| stress | Extrema | **111,788** | 2.781% | 640 |
| stress | Barter | 57,098 | 12.274% | 640 |
| stress | Nautilus | 17,658 | 2.819% | 640 |
| mixed_live | Barter | **1,186,568** | 20.291% | 120 |
| mixed_live | Extrema | 598,148 | 5.158% | 120 |
| mixed_live | Nautilus | 365,269 | 3.810% | 120 |

Reject 只产生一个 raw private frame，而成功订单产生 ACK + FILL 两个 frame，所以 error case 的 downstream 工作量更少；它只能用于三框架同 scenario 横比，不能用略高吞吐证明“错误路径更快”。

## Success 延迟

以下为每轮 percentile 再跨 7 轮取中位数。`Public` 单位为 `us`，其余单位为 `ms`；格式均为 `p50 / p95 / p99`。有限 burst 的 tick-to-order/ACK/FILL 包含排队延迟，不是单事件 service time。

| Mode | Framework | Public decode | Tick-to-order | Tick-to-ACK | Tick-to-FILL |
| --- | --- | ---: | ---: | ---: | ---: |
| stress | Extrema | 0.917 / 1.625 / 2.708 | 1.594 / 11.351 / 11.665 | 28.667 / 40.536 / 41.136 | 28.669 / 40.536 / 41.137 |
| stress | Barter | 0.958 / 1.583 / 3.125 | 3.655 / 30.823 / 35.675 | 54.525 / 111.501 / 116.348 | 54.526 / 111.502 / 116.361 |
| stress | Nautilus | **0.667 / 1.250 / 2.250** | 14.528 / 27.868 / 28.974 | 213.283 / 354.145 / 364.832 | 213.285 / 354.148 / 364.835 |
| mixed_live | Extrema | 0.750 / 1.375 / 2.000 | **0.210 / 1.338 / 1.942** | **0.976 / 2.812 / 3.125** | **0.991 / 2.833 / 3.134** |
| mixed_live | Barter | 0.833 / 1.333 / 1.917 | 4.603 / 5.204 / 5.232 | 9.283 / 15.910 / 16.334 | 9.284 / 15.923 / 16.336 |
| mixed_live | Nautilus | **0.667 / 1.167 / 1.666** | 11.759 / 19.316 / 19.892 | 43.494 / 64.505 / 66.011 | 43.497 / 64.508 / 66.014 |

Mixed-live 吞吐由 Barter 领先，但订单延迟由 Extrema 领先。这说明吞吐的主要分子是 25,600 个 public ticks，而 Extrema 对少量下单任务的跨 worker 推进更快；两种指标回答的是不同问题。

## 守恒与行为一致性

| Case | Ticks / orders / success / reject | Raw private frames | 20-way account observations |
| --- | ---: | ---: | ---: |
| stress success | 6,400 / 6,400 / 6,400 / 0 | 12,800 | 256,000 = 12,800 owner + 243,200 observer |
| stress error | 6,400 / 6,400 / 5,760 / 640 | 12,160 | 243,200 = 12,160 owner + 231,040 observer |
| mixed success | 25,600 / 1,200 / 1,200 / 0 | 2,400 | 48,000 = 2,400 owner + 45,600 observer |
| mixed error | 25,600 / 1,200 / 1,080 / 120 | 2,280 | 45,600 = 2,280 owner + 43,320 observer |

84 条正式 full-chain measurement 全部 `conservation_ok=true`，且每条均满足：

- `public_parsed = trade_callbacks = total_ticks`；
- `order_endpoint_calls = expected_orders`；
- `account_callbacks = expected_account_callbacks`；
- `drops / duplicates / missing / route / cross-batch / delivery-loss / broadcast-lag / ending-in-flight = 0`。

定向验证：Extrema 15 tests、Barter 14 integration tests、Nautilus 6 tests 全部通过；三方 M=1 smoke 的所有计数同样守恒。

## Public WS 多频道

独立 Criterion 0.8.2 microbenchmark，统一为 1 秒 warmup、3 秒 measurement、100 samples、三轮；表中取三轮 `median_ns` 的中位数。边界仅为 `raw payload -> production DTO decode -> native normalize`。

| Channel | Nautilus | Barter | Extrema | 最快 |
| --- | ---: | ---: | ---: | --- |
| Trade | **244.89 ns** | 260.87 ns (+6.53%) | 297.42 ns (+21.45%) | Nautilus |
| BBO / bookTicker | **278.16 ns** | 310.32 ns (+11.56%) | 377.66 ns (+35.77%) | Nautilus |
| Depth frame | 475.20 ns (+12.16%) | **423.67 ns** | 524.30 ns (+23.75%) | Barter |

- BBO 使用三方字节完全相同的 Binance USD-M `bookTicker` payload。
- Depth 使用三方字节完全相同的 `depthUpdate` payload，但只到单帧 normalize，不含 snapshot、sequencer、gap recovery 或 local-book apply。
- Trade 的业务值相同，但 Extrema/Nautilus 走原生 `aggTrade`，Barter 走其生产支持的 `trade` schema，不是同字节 parser 对比。
- 输出对象也不同：Extrema `Vec<Ws*>`、Barter `MarketEvent`、Nautilus 高精度 native model。对象构造属于各自生产 normalize 成本。
- 不包含框架路由、strategy callback、order/private path，因此不能从这张表推出框架总性能。

## M=256 Stress 过载探针

这是另一个固定 burst：25,600 ticks/orders、51,200 success private frames、1,024,000 account observations。1 warmup + 3 measured；失败 case 不进入吞吐排名。

| Framework | 结果 | 原生木桶 |
| --- | --- | --- |
| Nautilus | **失败** | bounded AccountOrder ingress capacity 8,192；同步 submit 使用 `try_send`，返回 `no available capacity` |
| Extrema | **失败** | OrderExecution broadcast receiver lagged，严格 observer 报 `skipped=1` |
| Barter | 3/3 守恒通过；中位 **49,273 tick/s** | private ingress `send().await` 施加回压；没有 broadcast-lag 类别 |

这证明在该 offered burst 下，Nautilus 和 Extrema 的原生容量边界先被撞到；Barter 通过回压无损完成。它仍是有限探针，不证明 Barter 可以无限期承受同一外部到达率。

## 公平边界与不可等价部分

本轮的“公平”是相同逻辑输入、相同订单与 callback 守恒、相同计时终点，再使用各框架原生结构；不是强行让三边执行相同 CPU 指令：

- Full-chain Trade 不是同一 raw bytes：Barter 使用生产 Binance Spot `trade` transformer；Extrema/Nautilus 使用 Binance USD-M `aggTrade`。价格、数量、side、symbol 分组、序号和订单 schedule 等逻辑值一致。
- Extrema 的 Binance public/private decoder、TaskChannels、Strategy tasks 与 OrderExecution runner 是框架路径；venue 是 benchmark-local in-memory venue。
- Barter public path 是 production parser/transformer，Engine、Strategy、risk 和 AccountEvent 是原生路径；Binance private `executionReport -> native AccountEvent` 是明确标注的 benchmark bridge，因为仓库没有对应 live private adapter。
- Barter 的一个 success FILL raw frame 会被转换为 `Trade + FullyFilled Snapshot` 两个原生 Engine events；连同 ACK，每个成功订单实际推进 3 个 account Engine events。公平计数仍按 2 个 raw private frames 和每帧 20 次逻辑观察守恒，但这项原生状态机工作是其 stress 成本的一部分。
- Nautilus success private path 复用 production Binance order-update DTO 与 `parse_exec`，但 correlation/dispatch 和 venue 是 benchmark-local；一个 owner 走 native order event，19 个 observer 走 benchmark `CustomData`。该下单路径直接到 execution engine/client，`risk_commands=0`。
- Business-error wire 也不相同：Extrema 解 production AccountOrder reject，Nautilus 解 Binance `code/msg/id` error envelope，Barter 解 benchmark `executionReport` reject；三边的 reject schedule 与 native terminal 语义一致，但不是同字节 parser 比较。
- 所有仓库在测量时均为 dirty worktree；JSONL metadata 固定 commit、benchmark source SHA-256、runner SHA-256、host 与 toolchain，不能和其他源码状态的数据静默合并。

因此这组结果适合回答“相同业务压力下，各框架原生架构的 CPU/调度木桶在哪里”，不等价于真实 Binance 网络端到端延迟，也不是生产交易所容量 SLA。

## 环境与原始数据

- Host：Apple Silicon macOS，15 logical CPUs；runner 配置 16 worker threads。
- Toolchain：`rustc 1.97.1`、`cargo 1.97.1`。
- Commits：Nautilus `0975556b`、Barter `33e56188`、Extrema `03844576`。
- Full-chain runner schema：5。

文件：

- `work/benchmarks/LIVE_PIPELINE_SPEC.md`：完整 workload、计时和守恒规范。
- `work/benchmarks/run_live_pipeline_capacity.py`：三框架统一 runner 与严格 topology normalization。
- `outputs/live-pipeline-native-ws5-strategy20-stress-m64-2026-08-05.jsonl`：stress raw results。
- `outputs/live-pipeline-native-ws5-strategy20-mixed-m256-2026-08-05.jsonl`：mixed-live raw results。
- `outputs/live-pipeline-native-ws5-strategy20-overload-stress-m256-2026-08-05.jsonl`：过载 measurement/failure records。
- `outputs/binance-public-ws-native-2026-08-05.jsonl`：Public WS 三频道、三轮 Criterion results。
- `work/benchmarks/binance_public_ws_crossbench/benches/channels.rs`：Public WS correctness assertions 与 benchmark cases。
