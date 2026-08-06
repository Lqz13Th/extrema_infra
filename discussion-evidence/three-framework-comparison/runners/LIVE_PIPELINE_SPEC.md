# Live Pipeline Benchmark Specification

This workload is separate from the normalized native tick-to-trade microbenchmark. It is a zero-network, live-shaped CPU/framework benchmark.

## Measured Path

The success path is:

```text
prebuilt raw public Trade frame
-> production public WS decode and normalization
-> framework-native routing
-> strategy signal handler and deterministic decision
-> framework-native order submission route
-> simulated in-memory venue
-> prebuilt raw private AccountOrder ACK frame decode
-> native owner callback plus required strategy observations
-> prebuilt raw private AccountOrder FILL frame decode
-> native owner callback plus required strategy observations
```

Business-error runs replace the successful private ACK/FILL sequence for each rejected order with one raw exchange error/reject frame and its native terminal callback.

TLS, socket IO, kernel networking, reconnect/backoff, real exchange latency and sleeps are excluded. Fixtures, instrument registration, JSON construction and correlation allocation happen before the timed phase. Absolute results include strict correlation, topology and completion instrumentation.

Frameworks may use the Binance trade stream type supported by their production adapter. The logical input is one public last-trade update; repositories do not have to use the same Binance wire event name when an adapter is absent.

## Common `batch_sharded` Contract

`batch_sharded` is the horizontal comparison topology for all three frameworks. `LIVE_TASKS` means the number of independent logical batches, denoted by `B`.

Each batch has exactly:

```text
20 independent Trade WS ingress lanes
  -> 5 strategy signal handlers
     -> each handler owns exactly 4 of the 20 Trade lanes

1 bounded raw AccountOrder WS ingress lane
  -> decode each raw private frame exactly once
  -> one batch-local fan-out domain
  -> exactly 5 strategy observations per raw private frame
```

The AccountOrder arrow above specifies logical fan-out, not a mandatory channel implementation. Only Extrema uses a physical Tokio broadcast ring. Barter uses synchronous fan-out inside a composite strategy callback, while Nautilus combines one native owner callback with four non-owner topic callbacks.

The common count invariants are:

| Quantity | Required value |
| --- | ---: |
| batches / AccountOrder ingress lanes / fan-out domains | `B` |
| Trade ingress lanes | `20B` |
| strategy signal handlers | `5B` |
| Trade lanes owned by each signal handler | `4` |
| AccountOrder handlers in each fan-out domain | `5` |
| observations for each decoded raw private frame | `5` |
| canonical native-owner observations for each raw private frame | `1` |
| non-owner observations for each raw private frame | `4` |

Trade lanes are partitioned, never broadcast: each Trade frame reaches one signal handler. AccountOrder frames are fanned out: every decoded frame reaches all five logical handlers in its batch. A batch must never observe another batch's Trade, order, account or correlation state.

For `B=5`, this creates 100 Trade ingress lanes, 25 signal handlers, five bounded AccountOrder ingress lanes and five independent account fan-out domains. A barrier releases all Trade producers together. Each producer yields cooperatively after every frame so one hot producer cannot consume its entire finite fixture without allowing the other ready ingress lanes to run.

`shared_handler` remains an optional historical/control topology. It does not satisfy the 20/5/4 sharded contract and must not be pooled with or relabeled as `batch_sharded`. Records made before topology became a case dimension are interpreted as `shared_handler` only.

## Native `ws5_strategy20` Contract

`ws5_strategy20` compares the same logical workload through each framework's native single-domain architecture. It does not force the three frameworks to use the same physical scheduler.

The fixed logical input is:

```text
20 zero-network Public WS producer groups
  -> each group multiplexes exactly 5 symbols in message-round order
  -> 100 symbols total

20 signal handlers
  -> handler N owns exactly the 5 symbols from producer N

1 bounded raw AccountOrder ingress
  -> decode each raw private frame exactly once
  -> 1 fan-out domain
  -> exactly 20 strategy observations per private frame
     = 1 canonical native-owner observation + 19 non-owner observations
```

`LIVE_TASKS` is the number of logical Public WS producer groups and must be 20. `LIVE_INSTRUMENTS_PER_TASK` is the number of symbols multiplexed by each producer and must be 5. These producers start from a barrier and yield after every published frame. They model already-received WebSocket payloads; `physical_sockets` is zero and TLS/socket work remains excluded.

The physical signal dispatch remains native:

| Framework | Native single-domain implementation |
| --- | --- |
| Extrema Infra | 20 zero-network producer groups publish through 20 native `TaskChannels` Trade task keys into 20 independently scheduled native Strategy tasks on one multi-thread Tokio runtime. The harness does not open or run 20 physical `WsTaskRunner` connections. |
| Nautilus Trader | 20 logical producers feed one LiveNode containing 20 real Strategies. Message-bus Strategy callbacks run synchronously on the LiveNode's single engine thread. |
| Barter-rs | 20 production transformers feed one Engine and one native composite Strategy containing 20 synchronous logical handlers. |

The private fan-out is also architecture-specific. Extrema uses one Tokio broadcast ring with 20 receivers. Nautilus sends one native order event to the actual owner and one owner-specific CustomData publication observed by the other 19 Strategies. Barter enters one native account callback and synchronously invokes 20 logical observations inside its composite Strategy. Barter's Binance private decoder remains an explicitly reported benchmark bridge.

The Trade wire event is production-native rather than byte-identical: Extrema and Nautilus consume Binance USD-M `aggTrade`, while Barter consumes its supported `trade` event. Timestamp, symbol, trade ID, price, quantity, side, per-symbol ordering and resulting business decisions are identical. Byte-identical comparisons are reported separately by channel-specific decoder benchmarks.

Every conserved `ws5_strategy20` result must report and validate:

| Quantity | Required value |
| --- | ---: |
| Public WS producer groups / ingress lanes | `20` |
| symbols per Public WS / signal handler | `5` |
| total symbols | `100` |
| signal handlers | `20` |
| AccountOrder ingress lanes / fan-out domains | `1` |
| AccountOrder observations per private frame | `20` |
| canonical owner / non-owner observations per private frame | `1 / 19` |

Old `shared_handler` and `batch_sharded` records are separate datasets and must not be pooled with `ws5_strategy20`.

## Physical Implementations

The logical workload is common; the runtime architecture is intentionally native to each framework.

### Extrema Infra

Each batch creates 20 native Trade task keys and 20 cooperative Tokio ingress tasks. Five native Extrema Strategy tasks each subscribe to four fixed Trade task keys. The batch also owns one OrderExecution runner, one venue handler, one bounded raw AccountOrder ingress task and one `TaskChannels` AccountOrder broadcast ring with five receivers.

The private ingress task decodes each frame once and broadcasts a decoded `Arc` to the five Strategy tasks. Handler 0 is the fixed canonical account owner; the other four callbacks are non-owner observations. Broadcast receivers execute independently. A slow receiver does not synchronously stop the other receiver callbacks, but it may overrun the bounded ring; any observed lag is a hard run failure.

Batches have separate task keys, handlers, runners, venues, ingress queues, rings and ownership state, but share one multi-thread Tokio runtime and its worker pool.

Capacities and overflow behavior:

- raw private AccountOrder `mpsc`: 8,192, using `send().await` backpressure;
- Trade, OrderExecution and AccountOrder broadcast rings: 8,192 each;
- skipped broadcast messages are observable as `broadcast_lagged` and must be zero.

### Barter-rs

Each batch creates 20 cooperative Tokio Trade ingress tasks and production transformers, one independent synchronous Barter Engine/feed, one venue and one bounded raw AccountOrder ingress task. The Engine supports one native Strategy, so the benchmark uses one composite Strategy containing five concrete logical signal handlers. Each logical handler owns four fixed Trade lanes.

Each private frame is decoded once and enters the native Engine account path once. The composite Strategy then invokes its five logical handlers synchronously. Logical handler 0 is the canonical observation counter. There is no physical AccountOrder broadcast ring or topic, and `trade_task_keys` is zero because Barter does not implement Extrema task keys.

A slow logical handler stalls the remaining handlers in that composite callback and therefore stalls that batch's Engine callback. Other batches have independent Engines and feeds.

Capacities and overflow behavior:

- raw private AccountOrder `mpsc`: 8,192, using `send().await` backpressure;
- public/Engine feed: unbounded in this harness;
- no broadcast ring exists, so broadcast lag is not an observable Barter failure category; loss and closure remain guarded by explicit conservation and timeout checks.

### Nautilus Trader

Each batch runs on its own OS thread with a current-thread Tokio runtime and one LiveNode. It has 20 per-instrument Trade producer tasks and five real Nautilus Strategy instances, each subscribed to four instruments. Batch identities for trader, account, client and strategies are unique.

Each raw private frame is decoded once. The Strategy that originated the order receives the one native order-event callback. Four other Strategies receive benchmark CustomData observations through the corresponding owner-specific topic. Five owner-specific topics are registered per batch; a frame selects the topic associated with its actual owner. Therefore all five Strategies are possible owners, but each raw private frame still has exactly one owner callback and four non-owner callbacks.

A blocking callback can stall its batch's current-thread runtime. Other batches run on separate OS threads and LiveNodes. Nautilus has no Tokio broadcast ring in this path, so its topic count must not be reported as a ring count or as five physical copies of the native order event.

Capacities and overflow behavior:

- raw private AccountOrder `mpsc`: 8,192; synchronous order submission uses `try_send`, and `Full` or `Closed` is fatal;
- internal DataEvent and ExecutionEvent channels used here are unbounded;
- source, callback, route and timeout checks expose loss or closure; broadcast lag is not an applicable category.

## Modes And Scenarios

- `stress`: every valid Trade tick creates one order.
- `mixed_live`: each instrument creates an order on every `LIVE_ORDER_EVERY`-th valid tick for that instrument; default 20, or 5%.
- `success`: no injected business errors; each successful order produces one raw ACK and one raw FILL frame.
- `business_error`: every `LIVE_REJECT_EVERY`-th order is rejected; default 10. Order ordinals start at zero, so any non-empty error run exercises the path.

The decision and error schedule are deterministic and identical across measured runs. In `mixed_live business_error`, `LIVE_MESSAGES_PER_INSTRUMENT` must be at least `LIVE_ORDER_EVERY`.

## Simulated Signal Computation

`LIVE_SIGNAL_COMPUTE_NS` adds deterministic CPU-bound strategy work to every valid public Trade callback. The owning signal handler executes one monotonic-clock busy wait of the configured duration after production decode/routing and before the order decision. It is applied to both order-producing and no-order ticks, exactly once per valid tick.

The helper deliberately does not use `sleep`, a Tokio timer or a blocking-pool handoff: this workload measures how each framework's native callback and scheduling architecture behaves when signal calculation occupies CPU. A configured value of zero retains the same callback and call-count instrumentation but performs no wait.

Every result reports `signal_compute_ns` and `signal_compute_calls`, and conservation requires:

```text
signal_compute_calls = total_ticks
```

The configured work is included in wall elapsed time and in downstream tick-to-order, tick-to-ACK and tick-to-FILL latency. It is not included in public decode latency because `t_public` is recorded before the strategy callback runs.

With `O` submitted orders, `R` rejects and `S = O - R` successes, let `H` be the required AccountOrder observations per frame (`5` for `batch_sharded`, `20` for `ws5_strategy20`):

```text
raw_private_frames       = 2S + R
account_observations     = H * raw_private_frames
native_owner_callbacks  = raw_private_frames
non_owner_callbacks     = (H - 1) * raw_private_frames
```

Canonical ACK, FILL, reject and terminal business counters advance once per business event, not once per logical observer. Transport disconnect/reconnect belongs in a later IO benchmark and is not simulated here. A benchmark-local decode or translation bridge must be identified in the report and cannot be presented as production-adapter coverage.

## Timing Boundaries

Each correlation records monotonic timestamps:

- `t0`: immediately before public raw-frame decode;
- `t_public`: immediately after public decode/normalization;
- `t_order`: first line of the simulated venue/order endpoint;
- `t_ack`: canonical native ACK callback entry;
- `t_fill`: canonical native FILL callback entry.

Reported distributions are `t_public - t0`, `t_order - t0`, `t_ack - t0` and `t_fill - t0`. `max_in_flight` increments at simulated endpoint entry and decrements at the canonical native terminal callback.

Wall throughput starts when the producer barrier opens. It ends only after all Trade frames have completed their strategy decisions, all expected orders have reached canonical terminal callbacks, and all required AccountOrder observations for every private frame have completed: five for `batch_sharded`, or twenty for `ws5_strategy20`. This includes trailing no-order ticks in `mixed_live`; it also prevents asynchronous Extrema receivers or Nautilus non-owner topics from being omitted from elapsed time.

## Conservation And Reporting

Every successful `batch_sharded` or `ws5_strategy20` measurement must expose and validate:

- raw Trade count, public frames parsed and strategy Trade callbacks;
- configured signal-compute duration and exactly one signal-compute call per valid Trade tick;
- expected orders, endpoint calls, ACKs, FILLs, rejects and terminal callbacks;
- raw private frames decoded exactly once;
- `20B` Trade ingress lanes, `5B` signal handlers and four Trade lanes per handler;
- `B` bounded AccountOrder ingress lanes and `B` fan-out domains;
- expected and actual topology-specific AccountOrder observation counts;
- native-owner and non-owner callback counts separately;
- physical `account_broadcast_rings`, `account_topics`, receivers, task keys, engines/LiveNodes, venues and order runners without invented aliases;
- `account_ingress_tasks`, `account_ingress_lanes`, `account_ingress_capacity` and the framework's actual overflow/backpressure model;
- per-lane, per-batch and per-handler sent/completed minima and maxima;
- duplicate, missing, route, cross-batch, delivery-loss and ending-in-flight counters;
- `broadcast_lagged`, plus whether broadcast lag is physically observable for that framework;
- handler, batch-isolation, owner-selection and private-fan-out model descriptors.

Any loss, lag, duplicate, route mismatch, cross-batch observation, unexpected business error, callback-count mismatch or remaining in-flight order makes the measurement non-conserved.

The normalized `account_callbacks` field always means all required logical AccountOrder observations: five-way for `batch_sharded`, twenty-way for `ws5_strategy20`. It must not be populated from a framework-specific native callback counter with different semantics.

## Burst Throughput Versus Sustained Capacity

The default `LIVE_MESSAGES_PER_INSTRUMENT=256` case is a fixed finite lossless burst: 5,120 Trade frames per batch. It is useful for topology scaling and latency comparison, but a passing throughput result does not prove an indefinitely sustainable input rate. Bounded queues and rings may absorb part of a short burst.

A longer `LIVE_MESSAGES_PER_INSTRUMENT=1024` `stress` run is a sustained-overload probe. It deliberately exceeds the short fixture and can reveal ring lag, fatal queue overflow or inability to drain at the offered rate. It is still a finite probe, not a proof of infinite-duration capacity. Results from the 256-message burst and the 1,024-message overload probe must be labeled and summarized separately.

Backpressure and loss are different outcomes. A `send().await` producer may slow to the consumer's drain rate with zero loss; `try_send` may fail immediately at capacity; a broadcast sender may continue while a receiver lags. Throughput is comparable only among conserved runs, while a failed overload case remains a capacity result and must not be silently discarded.

## Dataset And Runtime Controls

A case is keyed by:

```text
(framework, topology, mode, scenario, tasks)
```

Dataset metadata fixes the workload, runtime worker count, host/platform, Rust/Cargo versions, repository digests, runner schema/source and topology contract. A summary must reject incompatible datasets instead of pooling them.

Controls:

```text
LIVE_TASKS
LIVE_INSTRUMENTS_PER_TASK=20 # batch_sharded; 5 for ws5_strategy20
LIVE_MESSAGES_PER_INSTRUMENT
LIVE_MODE=stress|mixed_live
LIVE_ORDER_EVERY
LIVE_REJECT_EVERY
LIVE_SIGNAL_COMPUTE_NS=0
LIVE_WORKER_THREADS=16
LIVE_WARMUP_RUNS
LIVE_MEASURED_RUNS
LIVE_SCENARIO=success|business_error
LIVE_TOPOLOGY=shared_handler|batch_sharded|ws5_strategy20
```

The unified capacity matrix uses `batch_sharded` for Nautilus, Barter and Extrema, normally with the `1,2,5,10,16,32,64` batch ladder. Business-error coverage defaults to `B=5`. The current harness requires 16 configured worker threads; framework-specific runtime details remain explicitly reported.

Each successful measured run prints one `LIVE_PIPELINE_RESULT <json>` line. The runner writes it as `record_type=measurement` only after normalized conservation succeeds. A command timeout, non-zero exit, malformed output or failed conservation is written as `record_type=failure`, and the runner continues with the rest of the matrix before returning a non-zero status. The summarizer emits failure counts and reasons for each case but computes throughput and latency statistics only from conserved measurements. A case with only failures remains visible with blank performance and unknown diagnostic columns.

The default raw and summary outputs are `outputs/live-pipeline-capacity-raw-2026-08-02-batch-sharded.jsonl` and `outputs/live-pipeline-capacity-summary-2026-08-02-batch-sharded.csv`; they preserve the 2026-08-01 baseline. Resume requires an exact schema/source, host, toolchain, workload, topology contract and repository match, plus the complete expected run set for every completed case.
