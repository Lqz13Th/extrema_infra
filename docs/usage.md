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

`EnvBuilder::build()` validates task identities and explicit bindings, creates
the task rings, and returns `InfraResult<EnvMediator<_>>`. The
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
# Published crate usage:
# extrema_infra = "0.2"
tokio = { version = "1.53.0", features = ["full"] }
rustls = { version = "0.23", features = ["aws-lc-rs"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Exchange clients and websocket routing are controlled by Cargo features. Enable
only the markets used by the binary:

```toml
extrema_infra = { path = "../extrema_infra", features = ["binance", "okx"] }
```

Use `features = ["lob_clients"]` for the `LobClients` aggregate helper.
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

- `TimeScheduler(Duration)`: periodic ticks delivered to `on_schedule`.
- `InstIntent`: instrument or portfolio target intents delivered to
  `on_inst_intent`.
- `OrderExecution`: order batches delivered to `on_order_execution`.
- `ModelPreds(ModelRunner::Zmq(..))`: external model process integration.
  Enable `model_zmq`, `model_runner`, or `all`, to make this variant available.
- `ModelPreds(ModelRunner::Onnx(..))`: in-process ONNX inference. Enable
  `model_onnx`, `model_runner`, or `all`, to make this variant available.

Scheduler tasks publish to `on_schedule`, intent tasks to `on_inst_intent`,
order-execution relay tasks to `on_order_execution`, and model prediction tasks
to `on_preds`.

## Public Websocket Task

A public market-data strategy typically receives a `WsTaskInfo` startup event,
uses the command handle to connect and subscribe, then consumes normalized
events such as trades or candles. LOB updates are available only for exchange
relays that implement `WsChannel::Lob` routing.

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

The strategy owns the connect and subscribe sequence. In practice, the
`on_ws_event` handler finds the websocket handle, sends `TaskCommand::WsConnect`,
then sends exchange login/subscription messages as needed:

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
| Hyperliquid | `HYPERLIQUID_OWNER_ADDRESS`, `HYPERLIQUID_AGENT_PRIVATE_KEY`; optional `HYPERLIQUID_VAULT_ADDRESS` |

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

Ring capacity is selected internally per concrete task. Total reserved slots
therefore scale with publisher count, not receiver count: 100 Trade tasks at
the default capacity of 8,192 reserve 819,200 ring slots. Explicit bindings
reduce receivers and wakeups, but do not reduce publisher ring capacity.

## TLS Setup

If the binary uses REST or websocket clients, install a `rustls` crypto provider
before creating those clients:

```rust,no_run
rustls::crypto::aws_lc_rs::default_provider()
    .install_default()
    .expect("failed to install rustls crypto provider");
```

This belongs in the final binary, not inside library code, because `rustls`
allows only one process-wide default provider.

## Reference Patterns

Downstream repositories currently exercise these patterns:

- `funding_carry`: a single strategy module with scheduler, instrument intent,
  and state-save tasks.
- `portfolio_orchestrator`: several strategy modules in one runtime, including
  signal allocation, portfolio mediation, order execution, transfer ticks,
  account websocket streams, and evaluation tasks.
- `api_checkers`: small exchange-focused subcommands that demonstrate REST calls,
  public websocket streams, and private account websocket streams.
- `examples/empty_strategy_example.rs`: the smallest scheduler example.
- `examples/multi_strategy_example.rs`: multiple strategy modules in one
  runtime.
- `examples/websocket_private_account_example.rs`: private account websocket
  setup.
- `examples/hyperliquid_api_usage_example.rs`: read-only Hyperliquid REST API
  usage, including public market data and optional account balance/position
  reads by owner address.
