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

| Aspect                  | Traditional Vec<Box<dyn Trait>> | HList-based Extrema Infra |
|--------------------------|----------------------------------|----------------------------|
| **Dispatch**             | Dynamic (runtime `vtable`)       | Static (compile-time inlined) |
| **Type Safety**          | Runtime only                    | Compile-time enforced       |
| **Performance**          | Extra indirection, heap alloc    | Zero overhead, no heap alloc |
| **Compile-time Checking**| Limited                         | Full (trait bounds enforced) |

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

- **Strategy Execution Model**  
  - Trait-driven: `on_tick`, `on_bar`, `on_lob`.  
  - HList ensures safe registration of multiple strategy types.  


---

## Example: Spawn example strategy

```rust
use std::sync::Arc;
use tracing::info;

use extrema_infra::prelude::*;

#[derive(Clone)]
struct EmptyStrategy;
impl Strategy for EmptyStrategy {
    async fn execute(&mut self) {
        info!("[EmptyStrategy] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}
impl EventHandler for EmptyStrategy {}
impl AltEventHandler for EmptyStrategy {
    async fn on_timer(&mut self) {
        println!("[EmptyStrategy] Timer")
    }
}
impl CexEventHandler for EmptyStrategy {}
impl DexEventHandler for EmptyStrategy {}
impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel initialized");
    }
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimerBasedState(5),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_timer())
        .with_strategy(EmptyStrategy)
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.execute().await;
}

