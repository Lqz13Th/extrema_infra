use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio::sync::Mutex;

use std::future::{ready, Future};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::protocol::Message,
    MaybeTlsStream,
    WebSocketStream,
};
use futures_util::stream::SplitSink;

// ========================
// 消息类型
// ========================
#[derive(Clone, Debug)]
pub struct WsTrade { pub symbol: String, pub price: f64, pub qty: f64 }
#[derive(Clone, Debug)]
pub struct WsLob { pub symbol: String, pub bid: f64, pub ask: f64 }

#[derive(Clone, Debug)]
pub struct Timer { pub reminder: f64 }

// ========================
// 策略 trait
// ========================
pub trait Strategy: Clone + Send + Sync + 'static {
    fn on_trade(&mut self, _msg: Arc<WsTrade>) -> impl std::future::Future<Output=()> + Send { async {} }
    fn on_lob(&mut self, _msg: Arc<WsLob>) -> impl std::future::Future<Output=()> + Send { async {} }
    fn on_time(&mut self) -> impl std::future::Future<Output=()> + Send { async {} }
    fn on_init_ws_stream(
        &mut self,
        _ws_write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }
}

// ========================
// HList
// ========================
#[derive(Clone)] pub struct HNil;
#[derive(Clone)] pub struct HCons<Head, Tail> { pub head: Head, pub tail: Tail }

impl Strategy for HNil {}
impl<Head, Tail> Strategy for HCons<Head, Tail>
where Head: Strategy, Tail: Strategy
{
    fn on_trade(&mut self, msg: Arc<WsTrade>) -> impl std::future::Future<Output=()> + Send {
        let fut_head = self.head.on_trade(msg.clone());
        let fut_tail = self.tail.on_trade(msg);
        async move { fut_head.await; fut_tail.await; }
    }
    fn on_lob(&mut self, msg: Arc<WsLob>) -> impl std::future::Future<Output=()> + Send {
        let fut_head = self.head.on_lob(msg.clone());
        let fut_tail = self.tail.on_lob(msg);
        async move { fut_head.await; fut_tail.await; }
    }
    fn on_time(&mut self) -> impl std::future::Future<Output=()> + Send {
        let fut_head = self.head.on_time();
        let fut_tail = self.tail.on_time();
        async move { fut_head.await; fut_tail.await; }
    }
}

macro_rules! hlist { () => { HNil }; ($head:expr $(, $tail:expr)*) => { HCons { head: $head, tail: hlist!($($tail),*) } }; }

// ========================
// Channel enum
// ========================
#[derive(Clone)]
pub enum Channel {
    Trade(broadcast::Sender<Arc<WsTrade>>),
    Lob(broadcast::Sender<Arc<WsLob>>),
    Timer(broadcast::Sender<Arc<Timer>>),
}

// ========================
// EnvCore
// ========================
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub struct EnvCore<S> {
    pub strategies: S,
    pub channels: Arc<Vec<Channel>>,
}

impl<S> EnvCore<S>
where S: Strategy + Clone
{
    pub fn new(strategies: S, channels: Vec<Channel>) -> Self {
        Self { strategies, channels: Arc::new(channels) }
    }

    /// 每个策略集合独占一个 task
    pub async fn spawn_strategies(&self) {
        let mut strategies = self.strategies.clone();
        let channels = self.channels.clone();

        tokio::spawn(async move {
            // 使用 Option<Receiver>，缺失 channel 用 pending 占位
            let mut rx_trade = channels.iter().find_map(|ch| {
                if let Channel::Trade(tx) = ch { Some(tx.subscribe()) } else { None }
            });
            let mut rx_lob = channels.iter().find_map(|ch| {
                if let Channel::Lob(tx) = ch { Some(tx.subscribe()) } else { None }
            });
            let mut rx_timer = channels.iter().find_map(|ch| {
                if let Channel::Timer(tx) = ch { Some(tx.subscribe()) } else { None }
            });

            loop {
                tokio::select! {
                    msg = async {
                        if let Some(rx) = rx_trade.as_mut() { rx.recv().await } else { futures::future::pending().await }
                    } => {
                        if let Ok(msg) = msg { strategies.on_trade(msg).await; }
                        else { break; }
                    },

                    msg = async {
                        if let Some(rx) = rx_lob.as_mut() { rx.recv().await } else { futures::future::pending().await }
                    } => {
                        if let Ok(msg) = msg { strategies.on_lob(msg).await; }
                        else { break; }
                    },

                    msg = async {
                        if let Some(rx) = rx_timer.as_mut() { rx.recv().await } else { futures::future::pending().await }
                    } => {
                        if let Ok(_msg) = msg { strategies.on_time().await; }
                        else { break; }
                    },
                }
            }
        });
    }


    /// spawn ws tasks
    pub fn spawn_ws_tasks(&self) {
        let strategies = self.strategies.clone();
        let channels = self.channels.clone();
        for ch in channels.iter() {
            let ch_clone = ch.clone();
            let mut strategies_clone = strategies.clone();

            tokio::spawn(async move {
                match ch_clone {
                    Channel::Trade(tx) => {
                        // 模拟 ws
                        let url = "wss://echo.websocket.org";
                        let (mut ws_stream, _) = connect_async(url).await.unwrap();
                        println!("Trade WS connected");

                        ws_stream.send(tokio_tungstenite::tungstenite::Message::Text("trade".into())).await.unwrap();
                        let (ws_write, mut ws_read) = ws_stream.split();
                        let ws_write_arc = Arc::new(Mutex::new(ws_write));
                        strategies_clone.on_init_ws_stream(ws_write_arc.clone()).await;
                        while let Some(msg) = ws_read.next().await {
                            if let Ok(tokio_tungstenite::tungstenite::Message::Text(_txt)) = msg {
                                let trade = Arc::new(WsTrade { symbol: "BTCUSDT".into(), price: 50000.0, qty: 0.1 });
                                let _ = tx.send(trade);
                            }
                        }
                    },
                    Channel::Lob(tx) => {
                        let url = "wss://echo.websocket.org";
                        let (mut ws_stream, _) = connect_async(url).await.unwrap();
                        println!("Lob WS connected");

                        ws_stream.send(tokio_tungstenite::tungstenite::Message::Text("lob".into())).await.unwrap();

                        while let Some(msg) = ws_stream.next().await {
                            if let Ok(tokio_tungstenite::tungstenite::Message::Text(_txt)) = msg {
                                let lob = Arc::new(WsLob { symbol: "BTCUSDT".into(), bid: 49999.0, ask: 50001.0 });
                                let _ = tx.send(lob);
                            }
                        }
                    },
                    Channel::Timer(tx) => {
                        loop {
                            sleep(Duration::from_millis(1000)).await;
                            let timer = Arc::new(Timer { reminder: 1.0 });
                            if let Err(e) = tx.send(timer) {
                                println!("Timer send error: {:?}", e);
                            }
                        }
                    },
                }
            });
        }
    }
}

// ========================
// 示例策略
// ========================
#[derive(Clone)] struct StratA { a: f64 }
impl Strategy for StratA {
    async fn on_trade(&mut self, msg: Arc<WsTrade>)  {
        self.a += 2.0 * msg.qty;
        println!("[StratA] trade => a = {}, msg = {:?}", self.a, msg);
    }
    async fn on_lob(&mut self, msg: Arc<WsLob>)  {
        self.a += 2.0 ;
        println!("[StratA] lob => a = {}, msg = {:?}", self.a, msg);
    }
    async fn on_time(&mut self) {
        self.a += 0.5;
        println!("[StratA] on_time => a = {}", self.a);
    }
}

#[derive(Clone)] struct StratB { cash: f64 }
impl Strategy for StratB {
    async fn on_trade(&mut self, msg: Arc<WsTrade>)  {
        self.cash += 1.0 * msg.qty;
        println!("[StratB] trade => cash = {}, msg = {:?}", self.cash, msg);
    }
    async fn on_lob(&mut self, msg: Arc<WsLob>) {
        self.cash += 2.0;
        println!("[StratB] lob => cash = {}, msg = {:?}", self.cash, msg);
    }
    async fn on_time(&mut self) {
        self.cash += 1.0;
        println!("[StratB] on_time => cash = {}", self.cash);
    }
}

// ========================
// main
// ========================
#[tokio::main]
async fn main() {
    let strategies = hlist!(StratA { a: 1.0 }, StratB { cash: 2.0 });

    let trade_chan = Channel::Trade(broadcast::channel(16).0);
    let lob_chan = Channel::Lob(broadcast::channel(16).0);
    let timer_chan = Channel::Timer(broadcast::channel(16).0);

    let runner = EnvCore::new(strategies, vec![trade_chan, lob_chan]);

    // spawn strategy task
    runner.spawn_strategies().await;


    // spawn ws tasks
    runner.spawn_ws_tasks();

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}
