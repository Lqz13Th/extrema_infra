use std::{fmt::Debug, hint::black_box};

use serde_json::from_slice;

use crate::arch::{
    market_assets::exchange::{
        binance::{
            binance_ws_msg::BinanceWsData,
            schemas::{
                cm_futures_ws::lob::WsPartialDepthBinanceCM,
                spot_ws::account_order::WsAccountOrderEnvelopeBinanceSpot,
                um_futures_ws::{
                    account_position::WsAccountPositionBinanceUM,
                    agg_trades::WsAggTradeBinanceUM,
                    lob::{WsBookTickerBinanceUM, WsDiffDepthBinanceUM},
                },
            },
        },
        gate::{
            gate_ws_msg::GateWsData,
            schemas::{
                futures_ws::{
                    lob::{
                        WsBookTickerGateFutures, WsOrderBookGateFutures,
                        WsOrderBookUpdateGateFutures,
                    },
                    trades::WsTradeGateFutures,
                },
                spot_ws::account_order::WsAccountOrderGateSpot,
            },
        },
        hyperliquid::{
            hyperliquid_ws_msg::{HyperliquidWsData, HyperliquidWsState},
            schemas::ws::{
                account_order::WsAccountOrderHyperliquid,
                account_position::WsAccountPositionHyperliquid, lob::WsLobHyperliquid,
                trades::WsTradeHyperliquid,
            },
        },
        okx::{
            okx_ws_msg::OkxWsData,
            schemas::ws::{
                account_order::WsAccountOrderOkx, lob::OkxWsLobBook, trades::WsTradesOkx,
            },
        },
    },
    strategy_base::handler::lob_events::WsLob,
    traits::conversion::IntoWsData,
};

pub const BINANCE_TRADE: &[u8] = br#"{"e":"aggTrade","E":1780563843114,"a":987654321,"s":"BTCUSDT","p":"63405.40","q":"0.125","f":100,"l":101,"T":1780563843113,"m":true}"#;
pub const BINANCE_BBO: &[u8] = br#"{"e":"bookTicker","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#;
pub const BINANCE_DIFF_DEPTH: &[u8] = br#"{"e":"depthUpdate","E":1780563845145,"T":1780563845143,"s":"BTCUSDT","U":10708445053618,"u":10708445072904,"pu":10708445053562,"b":[["50746.30","0.000"],["50748.80","0.002"]],"a":[["63436.10","1.801"]]}"#;
pub const BINANCE_CM_SNAPSHOT: &[u8] = br#"{"e":"depthUpdate","E":1780563999230,"T":1780563999229,"s":"BTCUSD_PERP","ps":"BTCUSD","U":1691890103301,"u":1691890108316,"pu":1691890103299,"b":[["63298.6","5563"]],"a":[["63298.7","333"]]}"#;
pub const BINANCE_SPOT_ORDER: &[u8] = br#"{"subscriptionId":1,"event":{"E":1781905826733,"s":"REUSDT","c":"spot-client-id","i":370702544401581041,"S":"BUY","o":"MARKET","q":"16","p":"0","L":"0.87295","z":"16","X":"FILLED"}}"#;
pub const BINANCE_POSITIONS: &[u8] = br#"{"e":"ACCOUNT_UPDATE","E":1781905826733,"T":1781905826733,"a":{"m":"ORDER","B":[],"P":[{"s":"BTCUSDT","pa":"1.5","ep":"63900.1","cr":"0","up":"0","mt":"cross","iw":"0","ps":"BOTH"}]}}"#;
pub const BINANCE_ACK: &[u8] = br#"{"status":200,"result":null,"id":42}"#;
pub const BINANCE_NESTED_ERROR: &[u8] =
    br#"{"status":400,"id":42,"error":{"code":-1,"msg":"bad request"}}"#;
pub const BINANCE_TOP_LEVEL_ERROR: &[u8] = br#"{"code":-1,"msg":"bad request","id":42}"#;
pub const BINANCE_AMBIGUOUS_POSITIONS: &[u8] = br#"{"s":"ETHUSDT","pa":"2","ep":"3200","mt":"isolated","ps":"BOTH","a":{"P":[{"s":"BTCUSDT","pa":"1.5","ep":"63900.1","mt":"cross","ps":"BOTH"}]}}"#;

pub const GATE_TRADE: &[u8] = br#"{"time":1669843487,"time_ms":1669843487733,"channel":"futures.trades","event":"update","result":[{"contract":"BTC_USDT","create_time":1669843487,"create_time_ms":1669843487724,"id":180276616,"price":"1287","size":-3}]}"#;
pub const GATE_TRADE_SINGLE: &[u8] = br#"{"channel":"futures.trades","event":"update","result":{"contract":"BTC_USDT","create_time_ms":1669843487724,"id":180276616,"price":"1287","size":-3}}"#;
pub const GATE_BBO: &[u8] = br#"{"channel":"futures.book_ticker","event":"update","result":{"t":1780569310140,"u":114427040898,"s":"BTC_USDT","b":"62706.4","B":3335,"a":"62706.5","A":8275}}"#;
pub const GATE_SNAPSHOT: &[u8] = br#"{"channel":"futures.order_book","event":"all","result":{"t":1780569310995,"id":114427042721,"contract":"BTC_USDT","asks":[{"p":"62698.1","s":10046}],"bids":[{"p":"62698","s":144730}],"l":"20"}}"#;
pub const GATE_INCREMENTAL: &[u8] = br#"{"channel":"futures.order_book_update","event":"update","result":{"t":1780569312795,"U":114427044935,"u":114427045214,"s":"BTC_USDT","a":[{"p":"62706.5","s":0}],"b":[],"l":"20"}}"#;
pub const GATE_SPOT_ORDER: &[u8] = br#"{"channel":"spot.orders_v2","event":"update","result":[{"id":"370702544401581041","currency_pair":"RE_USDT","side":"buy","type":"market","amount":"16","price":"0","left":"0","filled_amount":"16","avg_deal_price":"0.87295","status":"closed","finish_as":"filled","event":"finish","update_time_ms":1781905826733,"create_time_ms":1781905826733,"text":"gate-spot-client-id"}]}"#;
pub const GATE_BBO_BATCH: &[u8] = br#"{"channel":"futures.book_ticker","event":"update","result":[{"t":1780569310140,"u":114427040898,"s":"BTC_USDT","b":"62706.4","B":3335,"a":"62706.5","A":8275}]}"#;
pub const GATE_ACK: &[u8] =
    br#"{"channel":"futures.trades","event":"subscribe","result":{"status":"success"}}"#;
pub const GATE_ERROR: &[u8] = br#"{"channel":"futures.trades","event":"subscribe","error":{"code":2,"message":"bad request"}}"#;

pub const HYPERLIQUID_TRADE: &[u8] = br#"{"channel":"trades","data":[{"coin":"BTC","side":"B","px":"63405.40","sz":"0.125","time":1780563843113,"tid":987654321}]}"#;
pub const HYPERLIQUID_ORDER: &[u8] = br#"{"channel":"orderUpdates","data":[{"order":{"coin":"GUN","side":"B","limitPx":"0.005857","sz":"0","oid":987654321,"timestamp":1781905826733,"origSz":"4350","cloid":"hl-client-id"},"status":"filled","statusTimestamp":1781905826733}]}"#;
pub const HYPERLIQUID_BBO: &[u8] = br#"{"channel":"bbo","data":{"coin":"BTC","time":1733833200000,"bbo":[{"px":"98450.5","sz":"2.5","n":3},{"px":"98451.0","sz":"1.5","n":2}]}}"#;
pub const HYPERLIQUID_L2: &[u8] = br#"{"channel":"l2Book","data":{"coin":"BTC","levels":[[{"px":"98450.5","sz":"2.5","n":3},{"px":"98449.0","sz":"1.8","n":2},{"px":"98448.0","sz":"0.75","n":1},{"px":"98447.0","sz":"3.2","n":4},{"px":"98446.0","sz":"1.1","n":2},{"px":"98445.0","sz":"2.0","n":3},{"px":"98444.0","sz":"0.5","n":1},{"px":"98443.0","sz":"1.4","n":2},{"px":"98442.0","sz":"0.9","n":1},{"px":"98441.0","sz":"1.7","n":2}],[{"px":"98451.0","sz":"1.5","n":2},{"px":"98452.0","sz":"2.1","n":3},{"px":"98453.0","sz":"0.9","n":1},{"px":"98454.0","sz":"1.7","n":2},{"px":"98455.0","sz":"2.3","n":4},{"px":"98456.0","sz":"0.6","n":1},{"px":"98457.0","sz":"1.2","n":2},{"px":"98458.0","sz":"0.8","n":1},{"px":"98459.0","sz":"1.5","n":2},{"px":"98460.0","sz":"2.0","n":3}]],"time":1733833200000}}"#;
pub const HYPERLIQUID_POSITIONS: &[u8] = br#"{"channel":"clearinghouseState","data":{"clearinghouseState":{"assetPositions":[{"type":"oneWay","position":{"coin":"BTC","szi":"1.5","entryPx":"63900.1","leverage":{"type":"cross","value":10}}}],"time":1781905826733}}}"#;
pub const HYPERLIQUID_PONG: &[u8] = br#"{"channel":"pong"}"#;
pub const HYPERLIQUID_ERROR: &[u8] = br#"{"channel":"error","data":{"message":"bad request"}}"#;
pub const HYPERLIQUID_AMBIGUOUS_LOB: &[u8] =
    br#"{"channel":"bbo","data":{"coin":"BTC","time":1,"levels":[[],[]],"bbo":[null,null]}}"#;

pub const OKX_TRADE: &[u8] = br#"{"arg":{"channel":"trades","instId":"BTC-USDT-SWAP"},"data":[{"instId":"BTC-USDT-SWAP","tradeId":"987654321","px":"63405.40","sz":"0.125","side":"buy","ts":"1780563843113"}]}"#;
pub const OKX_BBO: &[u8] = br#"{"arg":{"channel":"bbo-tbt","instId":"BCH-USDT-SWAP"},"data":[{"asks":[["111.06","55154","0","2"]],"bids":[["111.05","57745","0","2"]],"ts":"1670324386802","seqId":363996337}]}"#;
pub const OKX_SNAPSHOT: &[u8] = br#"{"arg":{"channel":"books","instId":"BTC-USDT-SWAP"},"action":"snapshot","data":[{"asks":[["8476.98","415","0","13"]],"bids":[["8476.97","256","0","12"]],"ts":"1597026383085","checksum":-855196043,"prevSeqId":-1,"seqId":123456}]}"#;
pub const OKX_HEARTBEAT: &[u8] = br#"{"arg":{"channel":"books","instId":"BTC-USDT"},"action":"update","data":[{"asks":[],"bids":[],"ts":"1597026383085","prevSeqId":15,"seqId":15}]}"#;
pub const OKX_ORDER: &[u8] = br#"{"arg":{"channel":"orders","instType":"ANY"},"data":[{"ordId":"2234567890","clOrdId":"okx-client-id","instId":"GUN-USDT-SWAP","instType":"SWAP","side":"buy","posSide":"net","tdMode":"cross","ordType":"market","state":"filled","px":null,"sz":"4350","fillPx":"0.005857","fillSz":"4350","fillPnl":null,"fillTime":"1781905826733","tradeId":"1","fee":"0","feeCcy":"USDT","uTime":"1781905826733"}]}"#;
pub const OKX_ACK: &[u8] =
    br#"{"event":"subscribe","arg":{"channel":"trades","instId":"BTC-USDT-SWAP"}}"#;
pub const OKX_ERROR: &[u8] = br#"{"event":"error","code":"60012","msg":"bad request","arg":{"channel":"trades","instId":"BTC-USDT-SWAP"}}"#;
pub const OKX_EMPTY_BATCH: &[u8] =
    br#"{"arg":{"channel":"trades","instId":"BTC-USDT-SWAP"},"data":[]}"#;

pub const MALFORMED: &[u8] = br#"{"broken":"#;

macro_rules! define_case {
    (
        $legacy_decode:ident,
        $preferred_decode:ident,
        $legacy_normalize:ident,
        $preferred_normalize:ident,
        $frame:ty,
        $decoder:expr
    ) => {
        pub fn $legacy_decode(frame: &[u8]) {
            let decoded: $frame = from_slice(frame).unwrap();
            black_box(decoded);
        }

        pub fn $preferred_decode(frame: &[u8]) {
            let decoded = ($decoder)(frame).unwrap();
            black_box(decoded);
        }

        pub fn $legacy_normalize(frame: &[u8]) {
            let decoded: $frame = from_slice(frame).unwrap();
            black_box(decoded.into_ws());
        }

        pub fn $preferred_normalize(frame: &[u8]) {
            let decoded = ($decoder)(frame).unwrap();
            black_box(decoded.into_ws());
        }
    };
}

define_case!(
    binance_trade_legacy_decode,
    binance_trade_preferred_decode,
    binance_trade_legacy_normalize,
    binance_trade_preferred_normalize,
    BinanceWsData<WsAggTradeBinanceUM>,
    BinanceWsData::<WsAggTradeBinanceUM>::decode_single
);
define_case!(
    binance_bbo_legacy_decode,
    binance_bbo_preferred_decode,
    binance_bbo_legacy_normalize,
    binance_bbo_preferred_normalize,
    BinanceWsData<WsBookTickerBinanceUM>,
    BinanceWsData::<WsBookTickerBinanceUM>::decode_single
);
define_case!(
    binance_positions_legacy_decode,
    binance_positions_preferred_decode,
    binance_positions_legacy_normalize,
    binance_positions_preferred_normalize,
    BinanceWsData<WsAccountPositionBinanceUM>,
    BinanceWsData::<WsAccountPositionBinanceUM>::decode_account_positions
);
define_case!(
    binance_diff_depth_legacy_decode,
    binance_diff_depth_preferred_decode,
    binance_diff_depth_legacy_normalize,
    binance_diff_depth_preferred_normalize,
    BinanceWsData<WsDiffDepthBinanceUM>,
    BinanceWsData::<WsDiffDepthBinanceUM>::decode_single
);
define_case!(
    binance_cm_snapshot_legacy_decode,
    binance_cm_snapshot_preferred_decode,
    binance_cm_snapshot_legacy_normalize,
    binance_cm_snapshot_preferred_normalize,
    BinanceWsData<WsPartialDepthBinanceCM>,
    BinanceWsData::<WsPartialDepthBinanceCM>::decode_single
);
define_case!(
    binance_spot_order_legacy_decode,
    binance_spot_order_preferred_decode,
    binance_spot_order_legacy_normalize,
    binance_spot_order_preferred_normalize,
    BinanceWsData<WsAccountOrderEnvelopeBinanceSpot>,
    BinanceWsData::<WsAccountOrderEnvelopeBinanceSpot>::decode_single
);
define_case!(
    gate_trade_legacy_decode,
    gate_trade_preferred_decode,
    gate_trade_legacy_normalize,
    gate_trade_preferred_normalize,
    GateWsData<WsTradeGateFutures>,
    GateWsData::<WsTradeGateFutures>::decode_batch
);
define_case!(
    gate_bbo_legacy_decode,
    gate_bbo_preferred_decode,
    gate_bbo_legacy_normalize,
    gate_bbo_preferred_normalize,
    GateWsData<WsBookTickerGateFutures>,
    GateWsData::<WsBookTickerGateFutures>::decode_single
);
define_case!(
    gate_snapshot_legacy_decode,
    gate_snapshot_preferred_decode,
    gate_snapshot_legacy_normalize,
    gate_snapshot_preferred_normalize,
    GateWsData<WsOrderBookGateFutures>,
    GateWsData::<WsOrderBookGateFutures>::decode_single
);
define_case!(
    gate_incremental_legacy_decode,
    gate_incremental_preferred_decode,
    gate_incremental_legacy_normalize,
    gate_incremental_preferred_normalize,
    GateWsData<WsOrderBookUpdateGateFutures>,
    GateWsData::<WsOrderBookUpdateGateFutures>::decode_single
);
define_case!(
    gate_spot_order_legacy_decode,
    gate_spot_order_preferred_decode,
    gate_spot_order_legacy_normalize,
    gate_spot_order_preferred_normalize,
    GateWsData<WsAccountOrderGateSpot>,
    GateWsData::<WsAccountOrderGateSpot>::decode_batch
);
define_case!(
    hyperliquid_trade_legacy_decode,
    hyperliquid_trade_preferred_decode,
    hyperliquid_trade_legacy_normalize,
    hyperliquid_trade_preferred_normalize,
    HyperliquidWsData<WsTradeHyperliquid>,
    HyperliquidWsData::<WsTradeHyperliquid>::decode_batch
);
define_case!(
    hyperliquid_bbo_legacy_decode,
    hyperliquid_bbo_preferred_decode,
    hyperliquid_bbo_legacy_normalize,
    hyperliquid_bbo_preferred_normalize,
    HyperliquidWsData<WsLobHyperliquid>,
    HyperliquidWsData::<WsLobHyperliquid>::decode_bbo
);
define_case!(
    hyperliquid_l2_legacy_decode,
    hyperliquid_l2_preferred_decode,
    hyperliquid_l2_legacy_normalize,
    hyperliquid_l2_preferred_normalize,
    HyperliquidWsData<WsLobHyperliquid>,
    HyperliquidWsData::<WsLobHyperliquid>::decode_l2_book
);
define_case!(
    hyperliquid_positions_legacy_decode,
    hyperliquid_positions_preferred_decode,
    hyperliquid_positions_legacy_normalize,
    hyperliquid_positions_preferred_normalize,
    HyperliquidWsData<WsAccountPositionHyperliquid>,
    HyperliquidWsData::<WsAccountPositionHyperliquid>::decode_clearinghouse
);
define_case!(
    hyperliquid_order_legacy_decode,
    hyperliquid_order_preferred_decode,
    hyperliquid_order_legacy_normalize,
    hyperliquid_order_preferred_normalize,
    HyperliquidWsData<WsAccountOrderHyperliquid>,
    HyperliquidWsData::<WsAccountOrderHyperliquid>::decode_batch
);
define_case!(
    okx_trade_legacy_decode,
    okx_trade_preferred_decode,
    okx_trade_legacy_normalize,
    okx_trade_preferred_normalize,
    OkxWsData<WsTradesOkx>,
    OkxWsData::<WsTradesOkx>::decode_batch
);
define_case!(
    okx_bbo_legacy_decode,
    okx_bbo_preferred_decode,
    okx_bbo_legacy_normalize,
    okx_bbo_preferred_normalize,
    OkxWsData<OkxWsLobBook>,
    OkxWsData::<OkxWsLobBook>::decode_batch
);
define_case!(
    okx_snapshot_legacy_decode,
    okx_snapshot_preferred_decode,
    okx_snapshot_legacy_normalize,
    okx_snapshot_preferred_normalize,
    OkxWsData<OkxWsLobBook>,
    OkxWsData::<OkxWsLobBook>::decode_batch
);
define_case!(
    okx_heartbeat_legacy_decode,
    okx_heartbeat_preferred_decode,
    okx_heartbeat_legacy_normalize,
    okx_heartbeat_preferred_normalize,
    OkxWsData<OkxWsLobBook>,
    OkxWsData::<OkxWsLobBook>::decode_batch
);
define_case!(
    okx_order_legacy_decode,
    okx_order_preferred_decode,
    okx_order_legacy_normalize,
    okx_order_preferred_normalize,
    OkxWsData<WsAccountOrderOkx>,
    OkxWsData::<WsAccountOrderOkx>::decode_batch
);

#[inline]
pub fn binance_bbo_current_decode(frame: &[u8]) -> serde_json::Result<impl Debug> {
    BinanceWsData::<WsBookTickerBinanceUM>::decode_single(frame)
}

#[inline]
pub fn binance_bbo_current_normalize(frame: &[u8]) -> serde_json::Result<Vec<WsLob>> {
    Ok(BinanceWsData::<WsBookTickerBinanceUM>::decode_single(frame)?.into_ws())
}

#[derive(Debug)]
pub struct BehaviorResult {
    pub name: &'static str,
    pub expected_equal: bool,
    pub actual_equal: bool,
    pub legacy_variant: String,
    pub preferred_variant: String,
    pub raw_equal: bool,
    pub normalized_equal: bool,
    pub legacy_normalized: String,
    pub preferred_normalized: String,
}

fn compare<T, Variant>(
    name: &'static str,
    expected_equal: bool,
    legacy: serde_json::Result<T>,
    preferred: serde_json::Result<T>,
    variant: Variant,
) -> BehaviorResult
where
    T: Clone + Debug + IntoWsData,
    T::Output: Debug,
    Variant: Fn(&T) -> &'static str,
{
    match (legacy, preferred) {
        (Ok(legacy), Ok(preferred)) => {
            let legacy_variant = variant(&legacy).to_string();
            let preferred_variant = variant(&preferred).to_string();
            let raw_equal = format!("{legacy:#?}") == format!("{preferred:#?}");
            let legacy_normalized = format!("{:#?}", legacy.into_ws());
            let preferred_normalized = format!("{:#?}", preferred.into_ws());
            let normalized_equal = legacy_normalized == preferred_normalized;
            let actual_equal = legacy_variant == preferred_variant && raw_equal && normalized_equal;

            BehaviorResult {
                name,
                expected_equal,
                actual_equal,
                legacy_variant,
                preferred_variant,
                raw_equal,
                normalized_equal,
                legacy_normalized,
                preferred_normalized,
            }
        },
        (Err(legacy), Err(preferred)) => {
            let legacy_error = format!(
                "{:?}|{}|{}|{}",
                legacy.classify(),
                legacy.line(),
                legacy.column(),
                legacy
            );
            let preferred_error = format!(
                "{:?}|{}|{}|{}",
                preferred.classify(),
                preferred.line(),
                preferred.column(),
                preferred
            );
            let actual_equal = legacy_error == preferred_error;

            BehaviorResult {
                name,
                expected_equal,
                actual_equal,
                legacy_variant: "Err".to_string(),
                preferred_variant: "Err".to_string(),
                raw_equal: actual_equal,
                normalized_equal: actual_equal,
                legacy_normalized: legacy_error,
                preferred_normalized: preferred_error,
            }
        },
        (legacy, preferred) => BehaviorResult {
            name,
            expected_equal,
            actual_equal: false,
            legacy_variant: if legacy.is_ok() { "Ok" } else { "Err" }.to_string(),
            preferred_variant: if preferred.is_ok() { "Ok" } else { "Err" }.to_string(),
            raw_equal: false,
            normalized_equal: false,
            legacy_normalized: format!("{legacy:#?}"),
            preferred_normalized: format!("{preferred:#?}"),
        },
    }
}

fn binance_variant<T>(message: &BinanceWsData<T>) -> &'static str {
    match message {
        BinanceWsData::ChannelSingle(_) => "ChannelSingle",
        BinanceWsData::ChannelBatch(_) => "ChannelBatch",
        BinanceWsData::AccountPositions(_) => "AccountPositions",
        BinanceWsData::Event(_) => "Event",
    }
}

fn gate_variant<T>(message: &GateWsData<T>) -> &'static str {
    match message {
        GateWsData::Channel(_) => "Channel",
        GateWsData::Single(_) => "Single",
        GateWsData::Event(_) => "Event",
    }
}

fn hyperliquid_variant<T>(message: &HyperliquidWsData<T>) -> &'static str {
    match message {
        HyperliquidWsData::Channel(channel) => match &channel.data {
            HyperliquidWsState::ChannelBatch(_) => "Channel/Batch",
            HyperliquidWsState::Clearinghouse(_) => "Channel/Clearinghouse",
            HyperliquidWsState::ChannelSingle(_) => "Channel/Single",
        },
        HyperliquidWsData::Event(_) => "Event",
    }
}

fn hyperliquid_lob_variant(message: &HyperliquidWsData<WsLobHyperliquid>) -> &'static str {
    match message {
        HyperliquidWsData::Channel(channel) => match &channel.data {
            HyperliquidWsState::ChannelBatch(_) => "Channel/Batch",
            HyperliquidWsState::Clearinghouse(_) => "Channel/Clearinghouse",
            HyperliquidWsState::ChannelSingle(WsLobHyperliquid::Book(_)) => "Channel/Single/Book",
            HyperliquidWsState::ChannelSingle(WsLobHyperliquid::Bbo(_)) => "Channel/Single/Bbo",
        },
        HyperliquidWsData::Event(_) => "Event",
    }
}

fn okx_variant<T>(message: &OkxWsData<T>) -> &'static str {
    match message {
        OkxWsData::ChannelBatch(_) => "ChannelBatch",
        OkxWsData::Event(_) => "Event",
    }
}

pub fn behavior_report() -> Vec<BehaviorResult> {
    let mut checks = Vec::new();

    macro_rules! check {
        ($name:literal, $expected:expr, $frame:expr, $ty:ty, $decoder:expr, $variant:expr) => {
            checks.push(compare(
                $name,
                $expected,
                from_slice::<$ty>($frame),
                ($decoder)($frame),
                $variant,
            ));
        };
    }

    check!(
        "binance/trade",
        true,
        BINANCE_TRADE,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/trade_ack",
        true,
        BINANCE_ACK,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/trade_nested_error",
        true,
        BINANCE_NESTED_ERROR,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/trade_top_level_error",
        true,
        BINANCE_TOP_LEVEL_ERROR,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/trade_cross_channel_bbo",
        true,
        BINANCE_BBO,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/trade_malformed",
        true,
        MALFORMED,
        BinanceWsData<WsAggTradeBinanceUM>,
        BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/account_positions",
        true,
        BINANCE_POSITIONS,
        BinanceWsData<WsAccountPositionBinanceUM>,
        BinanceWsData::<WsAccountPositionBinanceUM>::decode_account_positions,
        binance_variant
    );
    check!(
        "binance/account_positions_ack",
        true,
        BINANCE_ACK,
        BinanceWsData<WsAccountPositionBinanceUM>,
        BinanceWsData::<WsAccountPositionBinanceUM>::decode_account_positions,
        binance_variant
    );
    check!(
        "binance/um_diff_depth",
        true,
        BINANCE_DIFF_DEPTH,
        BinanceWsData<WsDiffDepthBinanceUM>,
        BinanceWsData::<WsDiffDepthBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/cm_snapshot",
        true,
        BINANCE_CM_SNAPSHOT,
        BinanceWsData<WsPartialDepthBinanceCM>,
        BinanceWsData::<WsPartialDepthBinanceCM>::decode_single,
        binance_variant
    );
    check!(
        "binance/spot_order",
        true,
        BINANCE_SPOT_ORDER,
        BinanceWsData<WsAccountOrderEnvelopeBinanceSpot>,
        BinanceWsData::<WsAccountOrderEnvelopeBinanceSpot>::decode_single,
        binance_variant
    );
    check!(
        "binance/bbo",
        true,
        BINANCE_BBO,
        BinanceWsData<WsBookTickerBinanceUM>,
        BinanceWsData::<WsBookTickerBinanceUM>::decode_single,
        binance_variant
    );
    check!(
        "binance/ambiguous_positions",
        false,
        BINANCE_AMBIGUOUS_POSITIONS,
        BinanceWsData<WsAccountPositionBinanceUM>,
        BinanceWsData::<WsAccountPositionBinanceUM>::decode_account_positions,
        binance_variant
    );

    check!(
        "gate/trade",
        true,
        GATE_TRADE,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );
    check!(
        "gate/trade_ack",
        true,
        GATE_ACK,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );
    check!(
        "gate/trade_error",
        true,
        GATE_ERROR,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );
    check!(
        "gate/trade_single_fallback",
        true,
        GATE_TRADE_SINGLE,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );
    check!(
        "gate/trade_malformed",
        true,
        MALFORMED,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );
    check!(
        "gate/bbo",
        true,
        GATE_BBO,
        GateWsData<WsBookTickerGateFutures>,
        GateWsData::<WsBookTickerGateFutures>::decode_single,
        gate_variant
    );
    check!(
        "gate/bbo_ack",
        true,
        GATE_ACK,
        GateWsData<WsBookTickerGateFutures>,
        GateWsData::<WsBookTickerGateFutures>::decode_single,
        gate_variant
    );
    check!(
        "gate/snapshot",
        true,
        GATE_SNAPSHOT,
        GateWsData<WsOrderBookGateFutures>,
        GateWsData::<WsOrderBookGateFutures>::decode_single,
        gate_variant
    );
    check!(
        "gate/incremental",
        true,
        GATE_INCREMENTAL,
        GateWsData<WsOrderBookUpdateGateFutures>,
        GateWsData::<WsOrderBookUpdateGateFutures>::decode_single,
        gate_variant
    );
    check!(
        "gate/spot_order",
        true,
        GATE_SPOT_ORDER,
        GateWsData<WsAccountOrderGateSpot>,
        GateWsData::<WsAccountOrderGateSpot>::decode_batch,
        gate_variant
    );
    check!(
        "gate/bbo_batch_fallback",
        true,
        GATE_BBO_BATCH,
        GateWsData<WsBookTickerGateFutures>,
        GateWsData::<WsBookTickerGateFutures>::decode_single,
        gate_variant
    );
    check!(
        "gate/trade_cross_channel_bbo",
        true,
        GATE_BBO,
        GateWsData<WsTradeGateFutures>,
        GateWsData::<WsTradeGateFutures>::decode_batch,
        gate_variant
    );

    check!(
        "hyperliquid/trade",
        true,
        HYPERLIQUID_TRADE,
        HyperliquidWsData<WsTradeHyperliquid>,
        HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/trade_pong",
        true,
        HYPERLIQUID_PONG,
        HyperliquidWsData<WsTradeHyperliquid>,
        HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/trade_error",
        true,
        HYPERLIQUID_ERROR,
        HyperliquidWsData<WsTradeHyperliquid>,
        HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/trade_cross_channel_bbo",
        true,
        HYPERLIQUID_BBO,
        HyperliquidWsData<WsTradeHyperliquid>,
        HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/trade_malformed",
        true,
        MALFORMED,
        HyperliquidWsData<WsTradeHyperliquid>,
        HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/bbo",
        true,
        HYPERLIQUID_BBO,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_bbo,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/l2",
        true,
        HYPERLIQUID_L2,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_l2_book,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/bbo_to_l2_fallback",
        true,
        HYPERLIQUID_L2,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_bbo,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/l2_to_bbo_fallback",
        true,
        HYPERLIQUID_BBO,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_l2_book,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/ambiguous_lob",
        false,
        HYPERLIQUID_AMBIGUOUS_LOB,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_bbo,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/account_positions",
        true,
        HYPERLIQUID_POSITIONS,
        HyperliquidWsData<WsAccountPositionHyperliquid>,
        HyperliquidWsData::<WsAccountPositionHyperliquid>::decode_clearinghouse,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/account_positions_pong",
        true,
        HYPERLIQUID_PONG,
        HyperliquidWsData<WsAccountPositionHyperliquid>,
        HyperliquidWsData::<WsAccountPositionHyperliquid>::decode_clearinghouse,
        hyperliquid_variant
    );
    check!(
        "hyperliquid/bbo_pong",
        true,
        HYPERLIQUID_PONG,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_bbo,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/l2_pong",
        true,
        HYPERLIQUID_PONG,
        HyperliquidWsData<WsLobHyperliquid>,
        HyperliquidWsData::<WsLobHyperliquid>::decode_l2_book,
        hyperliquid_lob_variant
    );
    check!(
        "hyperliquid/account_order",
        true,
        HYPERLIQUID_ORDER,
        HyperliquidWsData<WsAccountOrderHyperliquid>,
        HyperliquidWsData::<WsAccountOrderHyperliquid>::decode_batch,
        hyperliquid_variant
    );

    check!(
        "okx/trade",
        true,
        OKX_TRADE,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/trade_ack",
        true,
        OKX_ACK,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/trade_error",
        true,
        OKX_ERROR,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/trade_empty_batch",
        true,
        OKX_EMPTY_BATCH,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/trade_cross_channel_bbo",
        true,
        OKX_BBO,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/trade_malformed",
        true,
        MALFORMED,
        OkxWsData<WsTradesOkx>,
        OkxWsData::<WsTradesOkx>::decode_batch,
        okx_variant
    );
    check!(
        "okx/bbo",
        true,
        OKX_BBO,
        OkxWsData<OkxWsLobBook>,
        OkxWsData::<OkxWsLobBook>::decode_batch,
        okx_variant
    );
    check!(
        "okx/books_snapshot",
        true,
        OKX_SNAPSHOT,
        OkxWsData<OkxWsLobBook>,
        OkxWsData::<OkxWsLobBook>::decode_batch,
        okx_variant
    );
    check!(
        "okx/books_heartbeat",
        true,
        OKX_HEARTBEAT,
        OkxWsData<OkxWsLobBook>,
        OkxWsData::<OkxWsLobBook>::decode_batch,
        okx_variant
    );
    check!(
        "okx/account_order",
        true,
        OKX_ORDER,
        OkxWsData<WsAccountOrderOkx>,
        OkxWsData::<WsAccountOrderOkx>::decode_batch,
        okx_variant
    );

    checks
}

/// Compares the legacy untagged-enum decoder with the preferred Binance BBO decoder.
pub fn binance_bbo_edge_behavior_report() -> Vec<BehaviorResult> {
    const CASES: &[(&str, &[u8])] = &[
        (
            "standard",
            br#"{"e":"bookTicker","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "missing_e",
            br#"{"u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "missing_E",
            br#"{"e":"bookTicker","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114}"#,
        ),
        (
            "missing_u",
            br#"{"e":"bookTicker","s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "missing_T",
            br#"{"e":"bookTicker","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","E":1780563843114}"#,
        ),
        (
            "wrong_e",
            br#"{"e":"aggTrade","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        ("ack", br#"{"status":200,"result":null,"id":42}"#),
        (
            "nested_error",
            br#"{"status":400,"id":42,"error":{"code":-1,"msg":"bad request"}}"#,
        ),
        (
            "top_level_error",
            br#"{"code":-1,"msg":"bad request","id":42}"#,
        ),
        ("malformed", br#"{"broken":"#),
        (
            "zero_bid_price",
            br#"{"e":"bookTicker","u":7,"s":"BTCUSDT","b":"0","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "zero_bid_qty",
            br#"{"e":"bookTicker","u":7,"s":"BTCUSDT","b":"63405.40","B":"0","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "invalid_bid_price",
            br#"{"e":"bookTicker","u":7,"s":"BTCUSDT","b":"not-a-number","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
        (
            "invalid_bid_qty",
            br#"{"e":"bookTicker","u":7,"s":"BTCUSDT","b":"63405.40","B":"not-a-number","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#,
        ),
    ];

    CASES
        .iter()
        .map(|(name, frame)| {
            compare(
                name,
                true,
                from_slice::<BinanceWsData<WsBookTickerBinanceUM>>(frame),
                BinanceWsData::<WsBookTickerBinanceUM>::decode_single(frame),
                binance_variant,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::behavior_report;

    #[test]
    fn legacy_and_preferred_match_expected_behavior() {
        for check in behavior_report() {
            assert_eq!(
                check.actual_equal, check.expected_equal,
                "unexpected behavior result for {}: {check:#?}",
                check.name
            );
        }
    }
}
