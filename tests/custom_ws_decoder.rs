use std::{sync::Arc, time::Duration};

use extrema_infra::prelude::*;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::{net::TcpListener, sync::mpsc, sync::oneshot};
use tokio_tungstenite::{accept_async, tungstenite::Message};

const EVENT_TIMEOUT: Duration = Duration::from_secs(15);
const LOB_TASK: u64 = 7;
const TX_TASK: u64 = 8;
const MOCK: Market = Market::Custom(MockVenueWs::ID);

#[derive(Deserialize)]
struct MockBbo {
    bid: f64,
    ask: f64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MockWsData {
    Bbo(MockBbo),
    Event(serde::de::IgnoredAny),
}

impl MockWsData {
    fn decode(frame: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(frame)
    }
}

impl IntoWsData for MockWsData {
    type Output = Vec<WsLob>;

    fn into_ws(self) -> Self::Output {
        match self {
            MockWsData::Bbo(bbo) => vec![WsLob {
                timestamp: 0,
                market: MOCK,
                inst: "@1".into(),
                event: LobEventKind::Bbo,
                bids: vec![level(bbo.bid)],
                asks: vec![level(bbo.ask)],
                seq: None,
                checksum: None,
            }],
            MockWsData::Event(_) => Vec::new(),
        }
    }
}

fn level(price: f64) -> LobLevel {
    LobLevel {
        price,
        size: 1.0,
        action: LobLevelAction::Upsert,
        order_count: None,
        level_update_id: None,
    }
}

#[derive(Clone)]
struct MockVenueWs;

impl LobWsDecoder for MockVenueWs {
    const ID: u16 = 42;
    const NAME: &'static str = "mock_venue";

    async fn ws_channel<R: WsFrameRunner>(&self, channel: &WsChannel, runner: R) {
        match channel {
            WsChannel::Lob(_) => runner.ws_loop(TaskEvent::Lob, MockWsData::decode).await,
            WsChannel::Other(_) => runner.ws_loop(TaskEvent::WsOther, decode_raw_ws).await,
            _ => {},
        }
    }
}

#[derive(Debug, PartialEq)]
enum Received {
    Lob { market: Market, bid: f64, ask: f64 },
    Other(String),
}

#[derive(Clone)]
struct VenueProbe {
    url: String,
    events: mpsc::UnboundedSender<Received>,
    registry: Arc<CommandRegistry>,
}

impl Strategy for VenueProbe {
    async fn initialize(&mut self) {}
}

impl CommandEmitter for VenueProbe {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.registry.clone()
    }
}

impl EventHandler for VenueProbe {
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        let handle = self
            .find_ws_handle(&msg.data.ws_channel, msg.task_id)
            .expect("custom websocket task handle");
        let (tx, rx) = oneshot::channel();
        handle
            .send_command(
                TaskCommand::WsConnect {
                    msg: self.url.clone(),
                    ack: AckHandle::new(tx),
                },
                Some((AckStatus::WsConnect, rx)),
            )
            .await
            .expect("connect");
        let subscribe = match msg.data.ws_channel {
            WsChannel::Lob(_) => "lob",
            _ => "tx",
        };
        handle
            .send_command(
                TaskCommand::WsMessage {
                    msg: subscribe.into(),
                    ack: AckHandle::none(),
                },
                None,
            )
            .await
            .expect("subscribe");
    }

    async fn on_lob(&mut self, msg: InfraMsg<Vec<WsLob>>) {
        for lob in msg.data.iter() {
            let _ = self.events.send(Received::Lob {
                market: lob.market.clone(),
                bid: lob.bids[0].price,
                ask: lob.asks[0].price,
            });
        }
    }

    async fn on_ws_other(&mut self, msg: InfraMsg<Vec<WsOtherMessage>>) {
        for raw in msg.data.iter() {
            let _ = self.events.send(Received::Other(raw.raw_json.clone()));
        }
    }
}

async fn serve(listener: TcpListener) {
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut ws = accept_async(stream).await.unwrap();
            let subscribe = ws.next().await.unwrap().unwrap();
            let frames: &[&str] = if subscribe == Message::text("lob") {
                &[
                    r#"{"type":"subscribed"}"#,
                    "not json",
                    r#"{"bid":1.5,"ask":2.5}"#,
                ]
            } else {
                &[r#"{"type":"tx","id":7}"#]
            };
            for frame in frames {
                ws.send(Message::text(*frame)).await.unwrap();
            }
            while ws.next().await.is_some() {}
        });
    }
}

async fn next_event(receiver: &mut mpsc::UnboundedReceiver<Received>) -> Received {
    tokio::time::timeout(EVENT_TIMEOUT, receiver.recv())
        .await
        .expect("event before timeout")
        .expect("event channel open")
}

fn custom_task(ws_channel: WsChannel, task_id: u64) -> WsTaskInfo {
    WsTaskInfo {
        market: MOCK,
        ws_channel,
        filter_channels: true,
        chunk: 1,
        task_base_id: Some(task_id),
    }
}

#[tokio::test]
async fn custom_market_frames_reach_typed_and_raw_callbacks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(serve(listener));

    let (events, mut received) = mpsc::unbounded_channel();
    let probe = VenueProbe {
        url,
        events,
        registry: Arc::new(CommandRegistry::default()),
    };
    let env = EnvBuilder::new()
        .with_ws_decoder(MockVenueWs)
        .with_task(custom_task(WsChannel::Lob(None), LOB_TASK))
        .with_task(custom_task(WsChannel::Other("tx".into()), TX_TASK))
        .with_strategy_module(probe)
        .build()
        .unwrap();
    let runtime = tokio::spawn(env.execute());

    let mut events = vec![
        next_event(&mut received).await,
        next_event(&mut received).await,
    ];
    events.sort_by_key(|event| matches!(event, Received::Other(_)));

    assert_eq!(
        events,
        vec![
            Received::Lob {
                market: MOCK,
                bid: 1.5,
                ask: 2.5,
            },
            Received::Other(r#"{"type":"tx","id":7}"#.into()),
        ]
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(200), received.recv())
            .await
            .is_err()
    );

    runtime.abort();
    server.abort();
}
