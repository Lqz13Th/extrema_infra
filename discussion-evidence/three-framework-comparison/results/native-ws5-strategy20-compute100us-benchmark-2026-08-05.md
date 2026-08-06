# 三框架原生结构 Benchmark：每 Tick 100 us 策略计算

日期：2026-08-05（Asia/Shanghai）

## 结论

在同一个 `20 Public WS groups x 5 symbols + 20 signal handlers + 1 AccountOrder ingress` 逻辑拓扑中，每个有效 public Trade tick 加入 `100 us` CPU busy-wait 后，Extrema Infra 明显领先：

- `stress M=64`：Extrema `81,322 tick/s`，是 Barter 的 `8.63x`、Nautilus 的 `13.13x`。
- `mixed_live M=256`：Extrema `96,313 tick/s`，是 Barter 的 `9.70x`、Nautilus 的 `9.96x`。
- Barter 和 Nautilus 的 signal callback 在这个单 domain 拓扑中串行执行；`100 us/tick` 给它们形成约 `10,000 tick/s` 的硬上限。mixed-live 下二者分别达到 `9,930` 和 `9,674 tick/s`，已接近该上限。
- Extrema 的 20 个原生 Strategy task 可跨 Tokio workers 同时推进，因此没有被单 callback 串行上限卡住。其名义 busy-wait 重叠度为 stress `8.13`、mixed `9.63`。
- 这不是 parser 排名。Public decode 仍约 `0.5-1.0 us p50`，100 us 工作发生在 production decode/routing 之后、下单判断之前。

## 固定拓扑

```text
20 zero-network Public WS producer groups
  x 5 symbols = 100 symbols
  -> 20 signal handlers
     each handler owns exactly one producer group / 5 symbols

1 bounded raw AccountOrder ingress
  -> decode each private frame once
  -> one fan-out domain
  -> 20 observations per frame
     = 1 canonical owner + 19 observers
```

`WS` 在这里是已经收到 raw payload 后的 logical producer task，不是 20 条真实 TLS/socket 连接。三边输入、symbol 分组、订单 schedule、private frame 数、观察数和计时终点相同，物理调度结构保持各框架原生风格。

## 计算模型

每个有效 Trade 在所属 signal callback 内执行：

```text
production public decode/normalize
-> native route and complete correlation validation
-> Instant-based spin_loop until 100,000 ns elapsed
-> deterministic order decision
```

- 使用 `std::time::Instant` 和 `std::hint::spin_loop()`；没有 `sleep`、Tokio timer、async yield 或 `spawn_blocking`。
- order tick 和 no-order tick 都执行一次。
- 每条结果严格校验 `signal_compute_ns=100000` 和 `signal_compute_calls=total_ticks`。
- OS 抢占会使 elapsed 继续前进，所以 `throughput * 100 us` 是名义 busy-wait 重叠度，不是硬件 CPU 利用率计数器。

## 正式输入

| Mode | Ticks | Orders | Raw private frames | Account observations | Signal compute calls |
| --- | ---: | ---: | ---: | ---: | ---: |
| stress, M=64 | 6,400 | 6,400 | 12,800 | 256,000 | 6,400 |
| mixed_live, M=256 | 25,600 | 1,200 | 2,400 | 48,000 | 25,600 |

mixed-live 每个 symbol 在第 20、40、...、240 个 tick 下单，共 `12/256 = 4.6875%`，不是整 5%。正式结果均为 2 次 warmup + 7 次 measured 的中位数；CV 为七轮吞吐的样本标准差 / 均值。

## 吞吐 A/B

`0 ns` 与 `100 us` 使用同一份 schema 6 runner 和同一套 benchmark 源码，仅环境参数不同。

| Mode | Framework | 0 ns tick/s | 100 us tick/s | 100 us CV | 保留吞吐 | 名义 busy-wait 重叠度 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| stress | Extrema | 109,945 | **81,322** | 4.815% | 73.97% | 8.13 |
| stress | Barter | 49,294 | 9,424 | 0.699% | 19.12% | 0.94 |
| stress | Nautilus | 16,420 | 6,194 | 1.357% | 37.72% | 0.62 |
| mixed_live | Extrema | 552,959 | **96,313** | 6.263% | 17.42% | 9.63 |
| mixed_live | Barter | 1,158,111 | 9,930 | 0.254% | 0.86% | 0.99 |
| mixed_live | Nautilus | 354,068 | 9,674 | 0.329% | 2.73% | 0.97 |

零计算的 mixed-live 测的是框架编排开销，Barter 排第一；100 us 后工作量由 signal compute 主导，Extrema 的并行 Strategy task 使排名反转。Barter 的 0 ns stress 仍有明显离群值，CV 为 `78.692%`；因此其 0 ns 中位数适合描述这批运行的多数样本，不应当作稳定服务率。

## 100 us 全链路延迟

Tick-to-order 为跨七轮取中位后的 `p50 / p95 / p99`，FILL 为 `p50 / p99`，单位均为 ms。有限 burst 的 tick-to-order 包含 handler 队列等待，不是单 tick 的独立 service time。

| Mode | Framework | Tick-to-order | Tick-to-FILL p50 / p99 |
| --- | --- | ---: | ---: |
| stress | Extrema | **20.606 / 43.041 / 46.349** | **32.728 / 55.747** |
| stress | Barter | 326.849 / 622.680 / 648.781 | 666.758 / 676.834 |
| stress | Nautilus | 340.085 / 647.937 / 675.268 | 872.063 / 1,028.927 |
| mixed_live | Extrema | **21.545 / 104.991 / 119.153** | **35.362 / 132.367** |
| mixed_live | Barter | 1,309.993 / 2,413.150 / 2,538.946 | 2,574.127 / 2,575.488 |
| mixed_live | Nautilus | 1,313.588 / 2,440.034 / 2,559.747 | 2,618.202 / 2,639.579 |

Public decode p50 / p99 仍为微秒级：

| Mode | Extrema | Barter | Nautilus |
| --- | ---: | ---: | ---: |
| stress | 0.625 / 1.709 us | 0.959 / 2.875 us | 0.708 / 2.250 us |
| mixed_live | 0.542 / 1.708 us | 0.708 / 1.750 us | 0.667 / 1.625 us |

Barter/Nautilus mixed-live 的 tick-to-order p50 约为总计算队列的一半，p99 接近全部 `25,600 x 100 us = 2.56 s` 工作量；这正是单 engine 串行排队的形状。Extrema 将 20 个 handler 分摊到 worker pool，p99 降到约 119 ms。

## Extrema Worker 扫描

以下为 exploratory `1 warmup + 3 measured` 中位数，和正式 2+7 数据分开，不合并统计：

| Tokio workers | stress M=64 tick/s | mixed M=256 tick/s |
| ---: | ---: | ---: |
| 1 | 9,166 | 9,809 |
| 2 | 17,482 | 19,360 |
| 4 | 32,511 | 37,958 |
| 8 | 55,059 | 71,494 |
| 12 | 78,798 | 95,978 |
| 15 | failed in repeat | 105,823 |
| 16 | **89,230** | 103,042 |

15-worker stress 的一次重复在首轮约 `84,384 tick/s` 后，下一轮出现 AccountOrder broadcast lag：`skipped=18`，按硬守恒规则整次命令失败。16 workers 是本机 stress 扫描中最高的稳定采样点；mixed 下 15/16 基本同档。正式公平主表固定三边共同的 16-worker 配置，没有用失败轮或单次峰值替换结果。

## 架构原因

### Extrema Infra

20 个 signal handler 是 20 个独立原生 Strategy task，各自消费一个 Trade TaskKey。100 us callback 会占住当前 Tokio worker，但不会同步阻塞其余 Strategy task，其他 worker 可以继续处理不同 WS group。stress 每个 tick 都下单，后续还有 12,800 个 private frames 和 256,000 次观察；OrderExecution route、单 AccountOrder ingress 和 20-receiver broadcast ring 因而成为剩余木桶。worker 扫描中的 lag 失败也说明它不是无限扩展。

### Barter-rs

20 个 public producer task 最终进入一个原生 Engine 和一个 composite AlgoStrategy。Trade 只交给所属 logical handler，但 100 us 工作仍在同一个 Engine callback 线程中逐条执行；16 个 Tokio workers 不能把这个 signal loop 变成 20 路并行。mixed-live 几乎精确落在 10k tick/s 串行上限。stress 还要为每个 tick 下单并处理 private path，所以降到 9.4k。

### Nautilus Trader

20 个 Strategy 都是真实 Strategy，但位于同一个 LiveNode/current-thread engine，message-bus Strategy callback 同步分发。其 signal compute 同样串行。stress 下 native execution routing、owner order event 和 19 个 CustomData observer 带来比 Barter 更重的 downstream 工作，因此只有 6.2k；mixed-live 下订单稀疏，结果回到接近 10k 的 signal-compute 上限。

## 守恒与行为

四组正式 A/B 数据共 84 条 measurement：

- `0 ns`: stress 21 条 + mixed 21 条；
- `100 us`: stress 21 条 + mixed 21 条；
- 全部 `conservation_ok=true`；
- 全部 `signal_compute_calls=total_ticks`；
- drops、duplicates、missing、route errors、cross-domain errors、delivery loss、broadcast lag 和 ending-in-flight 总和均为 0。

定向测试通过：Extrema 16 tests、Barter 15 tests、Nautilus 7 tests。三边均覆盖 `200 ticks / 100 orders / 200 compute calls`，证明 no-order tick 也执行计算。另有统一 runner 的 0 ns 与 100 us smoke 均通过。

计算开关只加入 benchmark harness/test 文件；production Binance adapter 的订阅、Trade 过滤和业务决定逻辑没有被改变。当前正式排名只覆盖 `success`；本轮没有把 business-error case 混入，因为 reject 比成功订单少一个 private frame，工作量不同。

## 公平边界

- 包含 raw public JSON decode、native normalize/routing、100 us signal compute、order route、in-memory venue、raw ACK/FILL decode、owner callback 和 19 个 observer。
- 不包含 TLS、socket IO、内核网络、真实交易所延迟、重连/backoff、sleep、持久化和 PnL reporting。
- Trade 业务值与 schedule 一致，但 wire schema 保持 production-native：Extrema/Nautilus 为 Binance USD-M `aggTrade`，Barter 为其支持的 `trade`。
- Barter private adapter 仍是明确标注的 benchmark bridge；另外两边也包含 benchmark-local venue/correlation glue。
- Host 有 15 个 logical CPUs；16 workers 是沿用上一轮的统一 runtime 配置，不等于 16 个物理核，也不代表 Barter 或 Nautilus 的单 LiveNode/Engine signal callback 会自动变成多核。
- 所有仓库仍是 dirty worktree；JSONL 固定了 commit、benchmark source SHA-256、runner SHA-256、host 和 Rust/Cargo 版本。

## 数据文件

- `live-pipeline-native-ws5-strategy20-compute100us-stress-m64-2026-08-05.jsonl`
- `live-pipeline-native-ws5-strategy20-compute100us-mixed-m256-2026-08-05.jsonl`
- `live-pipeline-native-ws5-strategy20-compute0-stress-m64-2026-08-05.jsonl`
- `live-pipeline-native-ws5-strategy20-compute0-mixed-m256-2026-08-05.jsonl`
- 对应四份 `*-summary-2026-08-05.csv`
- `work/benchmarks/LIVE_PIPELINE_SPEC.md`
- `work/benchmarks/run_live_pipeline_capacity.py`，schema 6
- `work/benchmarks/summarize_live_pipeline_capacity.py`
