# Extrema Infra

A high-performance zero-cost abstractions quantitative trading infrastructure built with Rust.  

It maximizes runtime efficiency through **static dispatch** and promotes scalability with **Heterogeneous Lists (HList)** for strategy registration.  

At its core: **One unified framework for multiple exchanges, multiple strategies, zero runtime boxing.**

---

## Why HList?

Traditional frameworks force strategies into **homogeneous containers** (e.g., `Vec<Box<dyn Strategy>>`), which means:
- Runtime overhead due to dynamic dispatch (`vtable` lookups).  
- Possible type erasure issues.  
- Harder to leverage compile-time optimizations.

With **HList**:
- **Heterogeneous strategies** (different struct types) can be stored in one container.  
- **Compile-time guarantees**: only strategies implementing the `Strategy` trait can be registered.  
- **Zero-cost abstraction**: static dispatch, no `Box`, no dynamic allocation.  
- **Maximum flexibility**: easily mix and match different strategy types while keeping everything static.

---

## Traditional vs HList Approach

| Aspect                    | Traditional Vec<Box<dyn Trait>> | HList-based Extrema Infra     |
|---------------------------|---------------------------------|-------------------------------|
| **Dispatch**              | Dynamic (runtime `vtable`)      | Static (compile-time inlined) |
| **Type Safety**           | Runtime only                    | Compile-time enforced         |
| **Performance**           | Extra indirection, heap alloc   | Zero overhead, no heap alloc  |
| **Compile-time Checking** | Limited                         | Full (trait bounds enforced)  |

---

## Key Features

- **Unified Exchange Abstraction**  
  - All exchanges (Binance, OKX, etc.) normalized into unified `Market` enum + structs.  
  - Strategies write once, run anywhere.  

- **Broadcast-based Data Distribution**  
  - Subscribe once, broadcast to many.  
  - Multiple strategies consume the same feed without extra I/O.  

- **Static Efficiency**  
  - No dynamic boxing, no runtime dispatch.  
  - Unified REST & WS interfaces with pre-converted data.  

- **Lock-Free Concurrency**  
  - Message passing via channels and broadcast without mutex locks.  
  - Eliminates contention, ensuring **low-latency, high-throughput** event delivery.  
  - Perfect for real-time trading workloads.  

---

## Strategy Execution Model
- Trait-driven: `on_tick`, `on_bar`, `on_lob`.
- HList ensures safe registration of multiple strategy types.
- All infra timestamps are unified to microseconds (µs).
- All instrument names returned by the internal API are automatically normalized.

Instrument naming conventions:

- Crypto: underscore-separated, e.g., BTC_USDT_PERP
- Stock: exchange dot notation, e.g., AAPL_NASDAQ_EQ

---

## Strategy Traits

The extrema_infra crate provides the core traits to implement trading strategies:

- **Strategy**  
  Entry point of your strategy. Defines how it executes and spawns tasks.

- **EventHandler** (composed of):
  - **AltEventHandler** — handle timer or alternative task events.
  - **CexEventHandler** — handle centralized exchange (CEX) events like trades, orderbook (LOB), candles, account orders.
  - **DexEventHandler** — handle decentralized exchange (DEX) events.

- **CommandEmitter**  
  Used to initialize and register command handles for communication with tasks.

A minimal strategy must implement at least `Strategy` + `AltEventHandler` + `CommandEmitter`.

---

## Exchange API Traits

For connecting to exchanges, you need to implement these traits for each exchange client:

- **CexWebsocket**  
  Defines how to build subscription/connect messages for websocket streams.

- **MarketCexApi = CexPublicRest + CexPrivateRest**
  - **CexPublicRest**: market data (ticker, orderbook, candles, instruments).
  - **CexPrivateRest**: trading operations (init API key, place/cancel orders, get balance, get positions).

Default implementations return `InfraError::Unimplemented`, so you must override the methods you need.

---

## Example: Spawn example strategy

Make sure you activated the desired exchange api of the extrema infra crate on Cargo.toml:

```toml
[package]
name = "strategy"
version = "0.1.0"
edition = "2024"

[dependencies]
# Enable only the exchange APIs you need
extrema_infra = { path = "../extrema_infra", features = ["all"] }

# Tokio async runtime
tokio = { version = "1.47.1", features = ["full"] }

# Logging
tracing = "0.1.41"
tracing-subscriber = "0.3.20"
```

Then, on your main.rs:

```rust
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;


///# Empty strategy
#[derive(Clone)]
struct EmptyStrategy;

impl EventHandler for EmptyStrategy {}
impl DexEventHandler for EmptyStrategy {}

impl Strategy for EmptyStrategy {
  async fn execute(&mut self) {
    info!("[EmptyStrategy] Executing init strategy...");
  }

  fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}

impl AltEventHandler for EmptyStrategy {
  async fn on_timer(
    &mut self,
    msg: Arc<AltTimerEvent>,
  ) {
    info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
  }
}
impl CexEventHandler for EmptyStrategy {
  async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
    info!("[EmptyStrategy] Candle event: {:?}", msg);
  }
}

impl CommandEmitter for EmptyStrategy {
  fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
    info!("[EmptyStrategy] Command channel registered: {:?}", _command_handle);
  }
}


///# Binance strategy
#[derive(Clone)]
struct BinanceStrategy {
  command_handles: Vec<Arc<CommandHandle>>,
  binance_um_cli: BinanceUmCli, // public binance um future client without api keys
}

impl BinanceStrategy {
  fn new() -> Self {
    Self {
      command_handles: Vec::new(),
      binance_um_cli: BinanceUmCli::default(),
    }
  }

  async fn send_subscribe(&self, channel: WsChannel) -> InfraResult<()> {
    let ws_msg = self.binance_um_cli
            .get_public_sub_msg(&channel, Some(&["BTC_USDT_PERP".to_string()]))
            .await?;

    if let Some(handle) = self.find_handle_by_channel(&channel) {
      info!("[BinanceStrategy] Sending subscribe to {:?}", handle);

      let cmd = TaskCommand::Subscribe {
        msg: ws_msg,
        ack: AckHandle::none(),
      };

      if let Err(e) = handle.cmd_tx.send(cmd).await {
        warn!("[BinanceStrategy] Failed to send subscribe cmd: {:?}", e);
      }
    } else {
      warn!("[BinanceStrategy] No handle found for channel {:?}", channel);
    }

    Ok(())
  }

  async fn send_connect(&self, channel: WsChannel) -> InfraResult<()> {
    let ws_url = self.binance_um_cli.get_public_connect_msg(&channel).await?;

    if let Some(handle) = self.find_handle_by_channel(&channel) {
      info!("[BinanceStrategy] Sending connect to {:?}", handle);

      let (tx, rx) = oneshot::channel();
      let cmd = TaskCommand::Connect {
        msg: ws_url,
        ack: AckHandle::new(tx),
      };

      if let Err(e) = handle.cmd_tx.send(cmd).await {
        warn!("[BinanceStrategy] Failed to send connect cmd: {:?}", e);
        return Ok(());
      }

      match rx.await {
        Ok(Ok(AckStatus::Connect)) => {
          self.send_subscribe(channel.clone()).await?;
        }
        Ok(Ok(AckStatus::Subscribe)) => {
          info!("[BinanceStrategy] Subscribe complete");
        }
        _ => error!("[BinanceStrategy] Received unexpected response from server"),
      }
    } else {
      warn!("[BinanceStrategy] No handle found for channel {:?}", channel);
    }

    Ok(())
  }

  fn find_handle_by_channel(&self, channel: &WsChannel) -> Option<Arc<CommandHandle>> {
    self.command_handles.iter().find_map(|handle| {
      match &handle.task_info {
        TaskInfo::WsTask(ws_task) => {
          if ws_task.ws_channel == *channel {
            Some(handle.clone())
          } else {
            None
          }
        },
        _ => None,
      }
    })
  }
}

impl EventHandler for BinanceStrategy {}
impl DexEventHandler for BinanceStrategy {}

impl Strategy for BinanceStrategy {
  async fn execute(&mut self) {
    info!("[BinanceStrategy] Starting strategy");
  }
}

impl AltEventHandler for BinanceStrategy {
  async fn on_timer(
    &mut self,
    msg: Arc<AltTimerEvent>,
  ) {
    info!("[BinanceStrategy] AltEventHandler: {:?}", msg);
  }
}

impl CexEventHandler for BinanceStrategy {
  async fn on_cex_event(&mut self, _msg: Arc<WsTaskInfo>)  {
    info!("[BinanceStrategy] Triggering connect for channel: {:?}", _msg.ws_channel);
    if let Err(e) = self.send_connect(_msg.ws_channel.clone()).await {
      error!("[BinanceStrategy] connect failed: {:?}", e);
    }
  }

  async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
    info!("[BinanceStrategy] Candle event: {:?}", msg);
  }
}

impl CommandEmitter for BinanceStrategy {
  fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
    info!("[BinanceStrategy] Command channel registered: {:?}", command_handle);
    self.command_handles.push(command_handle);
  }
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  info!("Logger initialized");

  let binance_ws_candle = WsTaskInfo {
    market: Market::BinanceUmFutures,
    ws_channel: WsChannel::Candle(Some(CandleParam::OneMinute)),
    chunk: 1, // how many websocket connection on each task
  };

  let alt_task = AltTaskInfo {
    alt_task_type: AltTaskType::TimerBasedState(5),
  };

  let mediator = EnvBuilder::new()
          .with_board_cast_channel(BoardCastChannel::default_cex_event())
          .with_board_cast_channel(BoardCastChannel::default_candle())
          .with_board_cast_channel(BoardCastChannel::default_candle()) // duplicated skip
          .with_board_cast_channel(BoardCastChannel::default_timer())
          .with_strategy(EmptyStrategy)
          .with_strategy(BinanceStrategy::new())
          .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
          .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
          .build();

  mediator.execute().await;
}
```
