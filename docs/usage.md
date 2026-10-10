# Extrema Infra Usage Guide

This guide covers common runtime wiring for strategy modules, task-local
broadcast rings, task bindings, and command handles.

## Runtime Model

An application usually has four layers:

1. **Strategy modules** implement business logic.
2. **Tasks** own long-running work such as timers, model workers,
   order-execution relays, and websocket relays.
3. **Task-local broadcast rings** carry task output to strategies. Every
   concrete `TaskKey` owns one ring, and that task's lifecycle event and primary
   events travel together.
4. **Command handles** let strategies send commands back to tasks after the
   runtime has prepared them.

The final binary wires those pieces together with `EnvBuilder`:

```rust,no_run
use std::{sync::Arc, time::Duration};

use extrema_infra::prelude::*;

#[derive(Clone)]
struct StrategyModule {
    registry: Arc<CommandRegistry>,
}

impl StrategyModule {
    fn new() -> Self {
        Self {
            registry: Arc::new(CommandRegistry::default()),
        }
    }
}

impl Strategy for StrategyModule {
    async fn initialize(&mut self) {
        // Load config, initialize API clients, warm caches, etc.
    }
}

impl CommandEmitter for StrategyModule {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.registry.clone()
    }
}

impl EventHandler for StrategyModule {
    async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
        println!("schedule tick: task_id={}", msg.task_id);
    }
}

#[tokio::main]
async fn main() -> InfraResult<()> {
    let schedule_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(30)),
        chunk: 1,
        task_base_id: Some(1),
    };

    let env = EnvBuilder::new()
        .with_task(schedule_task)
        .with_strategy_module(StrategyModule::new())
        .build()?;

    env.execute().await;
    Ok(())
}
```

`EnvBuilder::build()` validates task identities, explicit bindings and
websocket decoders (unique decoder ids, one registered decoder for every
`Market::Custom` task), creates the task rings, and returns
`InfraResult<EnvMediator<_, _>>`. The
`with_strategy_module` call above subscribes the module to every registered
task.

## Prerequisites

Strategy binaries should declare their direct runtime dependencies. Do not rely
on transitive dependencies from `extrema_infra` when using `tokio`, `rustls`, or
logging crates in your own code. For process-wide TLS provider initialization,
see [TLS Setup](#tls-setup).

```toml
[dependencies]
# Local workspace development:
extrema_infra = { path = "../extrema_infra" }
tokio = { version = "1.53", features = ["full"] }
rustls = { version = "0.23", features = ["aws-lc-rs"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Exchange clients and websocket routing are controlled by Cargo features. Enable
only the markets used by the binary:

```toml
extrema_infra = { path = "../extrema_infra", features = ["binance", "okx"] }
```

Use `features = ["lob_clients"]` for the `LobClients` aggregate helper; it
also enables all four exchange features (`binance`, `okx`, `gate`,
`hyperliquid`). Custom venues (`Market::Custom` with `with_ws_decoder`) need no
exchange feature.
Use `features = ["model_zmq"]` or `features = ["model_onnx"]` for model
prediction task variants; `features = ["model_runner"]` enables both. Use
`features = ["polars"]` only when downstream code needs the Polars error
conversion. Use `features = ["all"]` for every exchange module, `LobClients`,
both model runners, and Polars support.

## Strategy Module Checklist

Every strategy module should implement three traits:

```rust
use std::sync::Arc;

use extrema_infra::prelude::*;

#[derive(Clone)]
struct MyModule {
    registry: Arc<CommandRegistry>,
}

impl Strategy for MyModule {
    async fn initialize(&mut self) {}
}

impl CommandEmitter for MyModule {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.registry.clone()
    }
}

impl EventHandler for MyModule {}
```

Use `initialize` for startup-only work. Use `command_init` only to store the
runtime-provided command registry. Implement only the event callbacks your
module needs; all other callbacks default to no-op.

Strategy modules receive every registered task by default. See
[Task Bindings](#task-bindings) for explicit per-task routing.

## Scheduler and Intent Tasks

`AltTaskInfo` is used for non-websocket runtime tasks:

```rust,ignore
use std::time::Duration;

use extrema_infra::prelude::*;

let schedule_task = AltTaskInfo {
    alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(60)),
    chunk: 1,
    task_base_id: Some(10),
};

let order_execution_task = AltTaskInfo {
    alt_task_type: AltTaskType::OrderExecution,
    chunk: 1,
    task_base_id: Some(20),
};

let env = EnvBuilder::new()
    .with_task(schedule_task)
    .with_task(order_execution_task);
```

Common `AltTaskType` values:

- `TimeScheduler(Duration)`: the duration must be greater than zero. After the
  task's approximately five-second startup delay, the first `on_schedule`
  callback is immediate; later callbacks use the configured duration.
- `InstIntent`: instrument or portfolio target intents delivered to
  `on_inst_intent`.
- `OrderExecution`: relays order batches to `on_order_execution`; the receiving
  application module implements actual exchange submission.
- `ModelPreds(ModelRunner::Zmq(..))`: external model process integration.
  Enable `model_zmq`, `model_runner`, or `all`, to make this variant available.
- `ModelPreds(ModelRunner::Onnx(..))`: in-process ONNX inference. Enable
  `model_onnx`, `model_runner`, or `all`, to make this variant available.

Scheduler tasks publish to `on_schedule`, intent tasks to `on_inst_intent`,
order-execution relay tasks to `on_order_execution`, and model prediction tasks
to `on_preds`.

## Public Websocket Task

A public market-data strategy receives a `WsTaskInfo` event before each
connection or reconnection cycle, uses the command handle to connect and
subscribe, then consumes normalized events such as trades or candles. The
handler must be safe to call repeatedly and must repeat the full initialization
sequence each time. LOB updates are available only for exchange relays that
implement `WsChannel::Lob` routing.

```rust,ignore
use extrema_infra::prelude::*;

const TASK_ID: u64 = 2001;

let trades_task = WsTaskInfo {
    market: Market::BinanceUmFutures,
    ws_channel: WsChannel::Trades(Some(TradesParam::AggTrades)),
    filter_channels: false,
    chunk: 1,
    task_base_id: Some(TASK_ID),
};

let env = EnvBuilder::new()
    .with_task(trades_task);
```

The corresponding strategy callbacks are:

```rust
use extrema_infra::prelude::*;

#[derive(Clone)]
struct MyPublicWsModule;

impl EventHandler for MyPublicWsModule {
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        // Find the Ws handle, connect, and subscribe.
        let _ = msg;
    }

    async fn on_trade(&mut self, msg: InfraMsg<Vec<WsTrade>>) {
        // Consume normalized trade batches.
        let _ = msg;
    }
}
```

The strategy owns the connect and subscribe sequence. The Binance UM example
below uses its URL-only target with `TaskCommand::WsConnect`, then sends exchange
login/subscription messages as needed:

```rust,ignore
async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
    if msg.task_id != TASK_ID {
        return;
    }

    let Some(handle) = self.find_ws_handle(&msg.data.ws_channel, msg.task_id) else {
        return;
    };

    let Ok(ws_url) = exchange_client
        .get_public_connect_msg(&msg.data.ws_channel)
        .await
    else {
        return;
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .send_command(
            TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            },
            Some((AckStatus::WsConnect, rx)),
        )
        .await
        .is_err()
    {
        return;
    }

    let Ok(sub_msg) = exchange_client
        .get_public_sub_msg(&msg.data.ws_channel, Some(&insts))
        .await
    else {
        return;
    };

    let _ = handle
        .send_command(
            TaskCommand::WsMessage {
                msg: sub_msg,
                ack: AckHandle::none(),
            },
            None,
        )
        .await;
}
```

URL-only clients use `get_*_connect_msg` with `TaskCommand::WsConnect`. Clients
that require HTTP upgrade metadata, currently including Gate Futures, use
`get_*_connect_target` with `TaskCommand::WsConnectWithTarget`; the string form
cannot carry headers.

## Private Account Websocket Task

Private account streams use the same task model, but publish account-specific
payloads:

```rust,ignore
use extrema_infra::prelude::*;

let positions_task = WsTaskInfo {
    market: Market::Okx,
    ws_channel: WsChannel::AccountPositions,
    filter_channels: false,
    chunk: 1,
    task_base_id: Some(3001),
};

let env = EnvBuilder::new()
    .with_task(positions_task);
```

Useful callbacks:

- `on_acc_order`: private order updates.
- `on_acc_bal_pos`: balance and position updates.
- `on_acc_pos`: position-only updates.
- `on_ws_other`: raw JSON frames from exchange-specific
  `WsChannel::Other(...)` tasks.
- `on_lagged`: the module fell behind on a task stream and lost events; resync
  account state over REST (see [Task Bindings](#task-bindings)).

The relay preserves the complete top-level JSON frame and a local receive
timestamp. Exchange-specific code can then decode fields such as Hyperliquid's
`channel: "post"` and `data.id`; the relay-level `AckStatus::WsMessage` still
means only that the text frame was written locally.

Exchange clients normally need API-key initialization in `Strategy::initialize`
before private websocket login messages are built. Credentials and login flows
are exchange-specific; for example, OKX uses a concrete login-message helper,
Binance UM/CM futures private streams require listen-key management and
periodic renewal, and Binance Spot private streams use the WS API signed
subscription helper.

Built-in private clients read credentials from the process environment or a
`.env` file:

| Exchange | Required variables |
| --- | --- |
| Binance | `BINANCE_API_KEY`, `BINANCE_SECRET_KEY` |
| OKX | `OKX_API_KEY`, `OKX_SECRET_KEY`, `OKX_PASSPHRASE` |
| Gate | `GATE_API_KEY`, `GATE_SECRET_KEY`, `GATE_USER_ID` |
| Hyperliquid | `HYPERLIQUID_OWNER_ADDRESS`, `HYPERLIQUID_AGENT_PRIVATE_KEY`; optional `HYPERLIQUID_VAULT_ADDRESS`, and `HYPERLIQUID_WITHDRAW_PRIVATE_KEY` (owner key, required only for withdrawals and `sendToEvmWithData`) |

## Multiple Strategy Modules

Large binaries can register several independent strategy modules in one static
runtime. A portfolio process, for example, can combine signal generation, weight
allocation, account-state mediation, order execution, and evaluation modules:

```rust,ignore
use extrema_infra::prelude::*;

// These are your concrete strategy modules. Each one implements Strategy,
// CommandEmitter, EventHandler, and Clone.
let signal_module = build_signal_module();
let allocator_module = build_allocator_module();
let account_state_module = build_account_state_module();
let order_executor_module = build_order_executor_module();
let runtime_tasks = build_runtime_tasks();

let env = EnvBuilder::new()
    .with_tasks(runtime_tasks)
    .with_strategy_module(signal_module)
    .with_strategy_module(allocator_module)
    .with_strategy_module(account_state_module)
    .with_strategy_module(order_executor_module)
    .build()?;
```

`EnvBuilder` stores strategy modules in a heterogeneous list. This keeps each
module as its concrete type instead of forcing all modules into
`Box<dyn Strategy>`. Each module has its own event loop. In the simple form
above, every module subscribes to every task. Use explicit bindings for larger
runtimes that partition feeds across modules.

## Task Bindings

Every concrete runtime task owns one broadcast ring. A `TaskInfo` declaration
with `chunk = n` expands to `n` `TaskKey` values and `n` independent rings, so
traffic or lag on one publisher does not write into another publisher's ring.
A selected ring carries both that task's lifecycle event and primary events.

`with_strategy_module` and `with_strategy_modules` subscribe to every task.
Their `_on` variants accept explicit keys and avoid receiver creation and
wakeups for unrelated tasks. `TaskInfo::task_keys()` expands a declaration's
whole chunk into the keys accepted by those methods.

For example, 100 one-task trade declarations can be split across 20 same-type
signal strategies, five task streams per strategy:

```rust,ignore
let trade_tasks: Vec<TaskInfo> = build_100_trade_tasks();

let bound_signal_modules = (0..20)
    .map(|partition| -> InfraResult<_> {
        let task_keys = trade_tasks[partition * 5..(partition + 1) * 5]
            .iter()
            .map(TaskInfo::task_keys)
            .collect::<InfraResult<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok((build_signal_strategy(partition), task_keys))
    })
    .collect::<InfraResult<Vec<_>>>()?;

let env = EnvBuilder::new()
    .with_tasks(trade_tasks)
    .with_strategy_modules_on(bound_signal_modules)
    .build()?;
```

Each pair passed to `with_strategy_modules_on` gets an independent strategy
handler loop and only its five receivers. For different strategy Rust types,
call `with_strategy_module_on` once per module.

The event side supports fan-out: several strategies may bind to the same
websocket task and receive its lifecycle and primary events. The command side
still needs one owner. Designate exactly one strategy to send that task's
connect, login, and subscribe sequence; the remaining consumers must not send
duplicate startup commands when they receive `on_ws_event`.

Every spawned task receives a `task_id`:

- If `task_base_id` is `Some(base)` and `chunk = n`, generated task IDs are
  `base`, `base + 1`, ..., `base + n - 1`.
- If `task_base_id` is `None`, generated task IDs start from `1` for that task
  declaration.

Use stable task IDs when a strategy must route events or command handles by
market, account, channel, or model worker.

Both task rings and command handles use `TaskKey`. `TaskKey::Alt` stores the
complete `AltTaskType` plus `task_id`, while `TaskKey::Ws` stores the complete
`WsChannel` plus `task_id`. Embedded parameters are part of identity: for
example, `Trades(AggTrades)` and `Trades(AllTrades)` produce different routing
keys, as do schedulers with different durations. Websocket market, chunk, and
`task_base_id` are not part of the key.

The `InfraMsg` callback envelope carries `task_id`, not the full `TaskKey`.
`EnvBuilder::build()` therefore rejects a duplicate task id for the same task
type, ignoring embedded parameters: two `Trades` channels or two schedulers
cannot share an id even though their full keys differ. Different task types,
such as Trade and LOB, may reuse an id. This check runs during build, not on the
message path. Build also rejects an explicit binding to an unregistered key.

Ring capacity is fixed per event type: Schedule 1,024; WS lifecycle,
InstIntent, Candle and Other 2,048; Trade, account streams, OrderExecute and
ModelPreds 8,192; Lob 16,384; LobMbo 65,536. Total reserved slots therefore
scale with publisher count, not receiver count: 100 Trade tasks reserve 819,200
ring slots. Explicit bindings reduce receivers and wakeups, but do not reduce
publisher ring capacity.

Delivery is lossy by design. A module that falls behind a full ring loses the
oldest events instead of stalling the publisher; other tasks' rings are
unaffected. The runtime then calls `EventHandler::on_lagged(key, skipped)`,
coalesced per task stream to at most one call per second, with `skipped` the
total dropped since the previous notice. A drop inside the one-second window is
reported only with the next lag on that stream. Treat the notice as a signal to
resync, for example by re-reading positions and open orders over REST.

## Custom Venue Websocket Decoder

A venue implemented outside this crate can run on the built-in websocket relay.
Implement `LobWsDecoder`, register it with `EnvBuilder::with_ws_decoder`, and
declare its tasks on `Market::Custom(MyVenueWs::ID)`. No exchange feature is
needed.

```rust,no_run
use extrema_infra::prelude::*;
# use std::sync::Arc;
# #[derive(serde::Deserialize)]
# struct MyBbo;
# impl IntoWsData for MyBbo {
#     type Output = Vec<WsLob>;
#     fn into_ws(self) -> Self::Output {
#         Vec::new()
#     }
# }
# fn decode_bbo(frame: &[u8]) -> serde_json::Result<MyBbo> {
#     serde_json::from_slice(frame)
# }
# #[derive(Clone)]
# struct MyStrategy;
# impl Strategy for MyStrategy {
#     async fn initialize(&mut self) {}
# }
# impl CommandEmitter for MyStrategy {
#     fn command_init(&mut self, _registry: Arc<CommandRegistry>) {}
#     fn command_registry(&self) -> Arc<CommandRegistry> {
#         Arc::new(CommandRegistry::default())
#     }
# }
# impl EventHandler for MyStrategy {}

#[derive(Clone)]
struct MyVenueWs;

impl LobWsDecoder for MyVenueWs {
    const ID: u16 = 42;
    const NAME: &'static str = "my_venue";

    async fn ws_channel<R: WsFrameRunner>(&self, channel: &WsChannel, runner: R) {
        match channel {
            WsChannel::Lob(_) => runner.ws_loop(TaskEvent::Lob, decode_bbo).await,
            WsChannel::Other(_) => runner.ws_loop(TaskEvent::WsOther, decode_raw_ws).await,
            _ => {},
        }
    }
}

# fn main() -> InfraResult<()> {
let lob_task = WsTaskInfo {
    market: Market::Custom(MyVenueWs::ID),
    ws_channel: WsChannel::Lob(None),
    filter_channels: false,
    chunk: 1,
    task_base_id: Some(1),
};

let env = EnvBuilder::new()
    .with_ws_decoder(MyVenueWs)
    .with_task(lob_task)
    .with_strategy_module(MyStrategy)
    .build()?;
# let _ = env;
# Ok(())
# }
```

- `ws_channel` runs once per connection. It picks a decode function for the
  task's channel and passes it to `WsFrameRunner::ws_loop` with the matching
  `TaskEvent` constructor, which decides the callback (`TaskEvent::Lob` ->
  `on_lob`, `TaskEvent::WsOther` -> `on_ws_other`, and so on). `decode_raw_ws`
  forwards complete frames to `on_ws_other`.
- The strategy still connects, authenticates and subscribes in `on_ws_event`,
  as with built-in venues. After a disconnect the relay waits 5 s and emits
  `on_ws_event` again.
- The relay sends a websocket Ping frame after 10 s without an inbound frame.
  Application-level pings must be sent with `TaskCommand::WsMessage`.
- Do not declare tasks on channels the decoder ignores: returning without
  calling the runner closes the connection, and the task reconnects about every
  5 s.

`tests/custom_ws_decoder.rs` is a complete worked example against a local
websocket server.

## TLS Setup

When exactly one built-in provider feature is enabled, `rustls` 0.23 can select
it automatically. A final binary can still install a provider explicitly before
creating REST or websocket clients to pin the process-wide choice:

```rust,no_run
rustls::crypto::aws_lc_rs::default_provider()
    .install_default()
    .expect("failed to install rustls crypto provider");
```

Explicit selection is necessary when the dependency graph enables zero or
multiple built-in providers, or when the application uses a custom provider.
It belongs in the final binary, not library code, because `rustls` allows only
one process-wide default provider.

## Reference Patterns

Downstream repositories currently exercise these patterns:

- `funding_carry`: a single strategy module with scheduler, instrument intent,
  and state-save tasks.
- `portfolio_orchestrator`: several strategy modules in one runtime, including
  signal allocation, portfolio mediation, order execution, transfer ticks,
  account websocket streams, and evaluation tasks.
- `api_checkers`: small exchange-focused subcommands that demonstrate REST calls,
  public websocket streams, and private account websocket streams.
- `tests/custom_ws_decoder.rs`: a custom venue on the built-in websocket relay.

The examples need the features listed in `Cargo.toml`, so a bare
`cargo build --examples` only builds `empty_strategy_example`:

| Example | Command |
| --- | --- |
| The smallest scheduler example | `cargo run --example empty_strategy_example` |
| Multiple strategy modules in one runtime | `cargo run --example multi_strategy_example --features binance` |
| Private account websocket setup, including Binance listen-key renewal | `cargo run --example websocket_private_account_example --features binance,okx` |
| Read-only Hyperliquid REST usage, including optional balance/position reads by owner address | `cargo run --example hyperliquid_api_usage_example --features hyperliquid` |
| OKX trades, account orders, two ZMQ model tasks and an `OrderExecution` relay with explicit bindings | `cargo run --example complex_strategy_example --features okx,model_zmq` |
