# Extrema Infra Usage Guide

This guide describes the generic shape of an `extrema_infra` application. It is
based on the real downstream patterns used by signal services, portfolio
orchestrators, exchange websocket checkers, and account/order utilities.

## Runtime Model

An application usually has four layers:

1. **Strategy modules** implement business logic.
2. **Tasks** own long-running work such as timers, model workers,
   order-execution relays, and websocket relays.
3. **Broadcast channels** publish task output to all registered strategy
   modules.
4. **Command handles** let strategies send commands back to tasks after the
   runtime has spawned them.

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
async fn main() {
    let schedule_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(30)),
        chunk: 1,
        task_base_id: Some(1),
    };

    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_scheduler())
        .with_task(TaskInfo::AltTask(Arc::new(schedule_task)))
        .with_strategy_module(StrategyModule::new())
        .build();

    env.execute().await;
}
```

## Prerequisites

Strategy binaries should declare their direct runtime dependencies. Do not rely
on transitive dependencies from `extrema_infra` when using `tokio`, `rustls`, or
logging crates in your own code. Binaries that use REST or websocket clients
must also install the `rustls` AWS-LC provider before constructing those
clients.

```toml
[dependencies]
# Local workspace development:
extrema_infra = { path = "../extrema_infra" }
# Published crate usage:
# extrema_infra = "0.1.0"
tokio = { version = "1.52.3", features = ["full"] }
rustls = { version = "0.23", features = ["aws-lc-rs"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Exchange clients and websocket routing are feature-gated. Enable only the
markets used by the binary:

```toml
extrema_infra = { path = "../extrema_infra", features = ["binance", "okx"] }
```

Use `features = ["lob_clients"]` when a process needs the `LobClients`
aggregate helper. Use `features = ["all"]` when it only needs all individual
exchange modules.

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

## Scheduler and Intent Tasks

`AltTaskInfo` is used for non-websocket runtime tasks:

```rust,ignore
use std::{sync::Arc, time::Duration};

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
    .with_board_cast_channel(BoardCastChannel::default_alt_event())
    .with_board_cast_channel(BoardCastChannel::default_scheduler())
    .with_board_cast_channel(BoardCastChannel::default_order_execution())
    .with_task(TaskInfo::AltTask(Arc::new(schedule_task)))
    .with_task(TaskInfo::AltTask(Arc::new(order_execution_task)));
```

Common `AltTaskType` values:

- `TimeScheduler(Duration)`: periodic ticks delivered to `on_schedule`.
- `InstIntent`: instrument or portfolio target intents delivered to
  `on_inst_intent`.
- `OrderExecution`: order batches delivered to `on_order_execution`.
- `ModelPreds(ModelRunner::Zmq(..))`: external model process integration.
- `ModelPreds(ModelRunner::Onnx(..))`: in-process ONNX inference.

Each alt task needs its matching channel and callback: scheduler tasks use
`BoardCastChannel::default_scheduler()` and `on_schedule`, intent tasks use
`default_inst_intent()` and `on_inst_intent`, order-execution relay tasks use
`default_order_execution()` and `on_order_execution`, and model prediction tasks
use `default_model_preds()` and `on_preds`.

## Public Websocket Task

A public market-data strategy typically receives a `WsTaskInfo` startup event,
uses the command handle to connect and subscribe, then consumes normalized
events such as trades or candles. LOB updates are available only for exchange
relays that implement `WsChannel::Lob` routing.

```rust,ignore
use std::sync::Arc;

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
    .with_board_cast_channel(BoardCastChannel::default_ws_event())
    .with_board_cast_channel(BoardCastChannel::default_trade())
    .with_task(TaskInfo::WsTask(Arc::new(trades_task)));
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

Registering a `WsTaskInfo` only creates the relay task. The strategy still owns
the connect and subscribe sequence. In practice, the `on_ws_event` handler
finds the websocket handle, sends `TaskCommand::WsConnect`, then sends exchange
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

## Private Account Websocket Task

Private account streams use the same task model, but publish account-specific
payloads:

```rust,ignore
use std::sync::Arc;

use extrema_infra::prelude::*;

let positions_task = WsTaskInfo {
    market: Market::Okx,
    ws_channel: WsChannel::AccountPositions,
    filter_channels: false,
    chunk: 1,
    task_base_id: Some(3001),
};

let env = EnvBuilder::new()
    .with_board_cast_channel(BoardCastChannel::default_ws_event())
    .with_board_cast_channel(BoardCastChannel::default_account_pos())
    .with_task(TaskInfo::WsTask(Arc::new(positions_task)));
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

let env = EnvBuilder::new()
    .with_board_cast_channel(BoardCastChannel::default_alt_event())
    .with_board_cast_channel(BoardCastChannel::default_ws_event())
    .with_board_cast_channel(BoardCastChannel::default_inst_intent())
    .with_board_cast_channel(BoardCastChannel::default_order_execution())
    .with_board_cast_channel(BoardCastChannel::default_account_order())
    .with_board_cast_channel(BoardCastChannel::default_account_pos())
    .with_strategy_module(signal_module)
    .with_strategy_module(allocator_module)
    .with_strategy_module(account_state_module)
    .with_strategy_module(order_executor_module)
    .build();
```

`EnvBuilder` stores strategy modules in a heterogeneous list. This keeps each
module as its concrete type instead of forcing all modules into
`Box<dyn Strategy>`.

## Task IDs and Chunks

Every spawned task receives a `task_id`:

- If `task_base_id` is `Some(base)` and `chunk = n`, generated task IDs are
  `base`, `base + 1`, ..., `base + n - 1`.
- If `task_base_id` is `None`, generated task IDs start from `1` for that task
  declaration.

Use stable task IDs when a strategy must route events or command handles by
market, account, channel, or model worker.

Command registry keys are not market-aware. They are keyed as
`(AltTaskType, task_id)` for alt tasks and `(WsChannel, task_id)` for websocket
tasks. If two tasks share the same `AltTaskType` or `WsChannel`, give them
distinct `task_base_id` values even when they target different markets or
accounts.

## Channels

Only add channels that the process needs. `EnvBuilder` skips duplicate channel
variants, so adding `BoardCastChannel::default_trade()` twice still results in
one trade broadcast channel.

Common channel pairs:

| Task output | Required channel | Callback |
| --- | --- | --- |
| Schedule tick | `default_scheduler()` | `on_schedule` |
| Instrument intent | `default_inst_intent()` | `on_inst_intent` |
| Order execution batch | `default_order_execution()` | `on_order_execution` |
| Model predictions | `default_model_preds()` | `on_preds` |
| Public trades | `default_trade()` | `on_trade` |
| Public LOB, when the exchange relay implements it | `default_lob()` | `on_lob` |
| Public candles | `default_candle()` | `on_candle` |
| Account orders | `default_account_order()` | `on_acc_order` |
| Account balance/position | `default_account_bal_pos()` | `on_acc_bal_pos` |
| Account positions | `default_account_pos()` | `on_acc_pos` |

`default_alt_event()` and `default_ws_event()` publish task startup/control
events. Strategy modules often use these events to find command handles and
connect websocket relays.

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
