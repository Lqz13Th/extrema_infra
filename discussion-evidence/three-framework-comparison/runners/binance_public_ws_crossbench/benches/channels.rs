use std::{hint::black_box, time::Duration};

use barter_data::{
    event::{MarketEvent, MarketIter},
    exchange::binance::{
        book::l1::BinanceOrderBookL1, futures::l2::BinanceFuturesOrderBookL2Update,
        trade::BinanceTrade,
    },
    subscription::{
        book::{OrderBookEvent, OrderBookL1},
        trade::PublicTrade,
    },
};
use barter_instrument::exchange::ExchangeId;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use extrema_infra::arch::{
    market_assets::{
        exchange::binance::benchmark::{
            decode_um_agg_trade, decode_um_book_ticker, decode_um_diff_depth,
        },
        market_core::Market,
    },
    strategy_base::handler::lob_events::{LobEventKind, LobLevelAction},
};
use nautilus_binance::futures::websocket::streams::{
    messages::{
        BinanceFuturesAggTradeMsg, BinanceFuturesBookTickerMsg, BinanceFuturesDepthUpdateMsg,
    },
    parse_data::{parse_agg_trade, parse_book_ticker, parse_depth_update},
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{OrderBookDeltas, QuoteTick, TradeTick},
    identifiers::{InstrumentId, Symbol},
    instruments::{CryptoPerpetual, InstrumentAny},
    types::{Currency, Price, Quantity},
};

const AGG_TRADE: &[u8] = br#"{"e":"aggTrade","E":1735689600000,"s":"BTCUSDT","a":5933014,"p":"0.001","q":"100","f":100,"l":105,"T":1735689599996,"m":true}"#;
const BARTER_TRADE: &[u8] = br#"{"e":"trade","E":1735689600000,"s":"BTCUSDT","t":5933014,"p":"0.001","q":"100","T":1735689599996,"m":true}"#;
const BOOK_TICKER: &[u8] = br#"{"e":"bookTicker","u":10708444522016,"s":"BTCUSDT","b":"63405.40","B":"4.629","a":"63405.50","A":"2.357","T":1780563843114,"E":1780563843114}"#;
const DIFF_DEPTH: &[u8] = br#"{"e":"depthUpdate","E":1780563845145,"T":1780563845143,"s":"BTCUSDT","U":10708445053618,"u":10708445072904,"pu":10708445053562,"b":[["50746.30","0.000"],["50748.80","0.002"]],"a":[["63436.10","1.801"]]}"#;
const INSTRUMENT_KEY: u64 = 7;
const TS_INIT: UnixNanos = UnixNanos::from_millis(1_780_563_845_146);

fn btcusdt_usdm_perpetual() -> InstrumentAny {
    InstrumentAny::CryptoPerpetual(CryptoPerpetual::new(
        InstrumentId::from("BTCUSDT-PERP.BINANCE"),
        Symbol::from("BTCUSDT"),
        Currency::from("BTC"),
        Currency::from("USDT"),
        Currency::from("USDT"),
        false,
        2,
        3,
        Price::from("0.01"),
        Quantity::from("0.001"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        UnixNanos::default(),
        UnixNanos::default(),
    ))
}

fn barter_trade(raw: &[u8]) -> MarketEvent<u64, PublicTrade> {
    let wire: BinanceTrade = serde_json::from_slice(raw).expect("valid Binance trade");
    let MarketIter(mut events) = (ExchangeId::BinanceFuturesUsd, INSTRUMENT_KEY, wire).into();
    events.pop().expect("one trade event").expect("valid trade")
}

fn barter_book_ticker(raw: &[u8]) -> MarketEvent<u64, OrderBookL1> {
    let wire: BinanceOrderBookL1 = serde_json::from_slice(raw).expect("valid Binance bookTicker");
    let MarketIter(mut events) = (ExchangeId::BinanceFuturesUsd, INSTRUMENT_KEY, wire).into();
    events
        .pop()
        .expect("one bookTicker event")
        .expect("valid bookTicker")
}

fn barter_diff_depth(raw: &[u8]) -> MarketEvent<u64, OrderBookEvent> {
    let wire: BinanceFuturesOrderBookL2Update =
        serde_json::from_slice(raw).expect("valid Binance depth update");
    let MarketIter(mut events) = (ExchangeId::BinanceFuturesUsd, INSTRUMENT_KEY, wire).into();
    events
        .pop()
        .expect("one depth event")
        .expect("valid depth event")
}

fn nautilus_trade(raw: &[u8], instrument: &InstrumentAny) -> TradeTick {
    let wire: BinanceFuturesAggTradeMsg =
        serde_json::from_slice(raw).expect("valid Binance aggTrade");
    parse_agg_trade(&wire, instrument, TS_INIT).expect("valid normalized aggTrade")
}

fn nautilus_book_ticker(raw: &[u8], instrument: &InstrumentAny) -> QuoteTick {
    let wire: BinanceFuturesBookTickerMsg =
        serde_json::from_slice(raw).expect("valid Binance bookTicker");
    parse_book_ticker(&wire, instrument, TS_INIT).expect("valid normalized bookTicker")
}

fn nautilus_diff_depth(raw: &[u8], instrument: &InstrumentAny) -> OrderBookDeltas {
    let wire: BinanceFuturesDepthUpdateMsg =
        serde_json::from_slice(raw).expect("valid Binance depth update");
    parse_depth_update(&wire, instrument, TS_INIT).expect("valid normalized depth update")
}

fn verify(instrument: &InstrumentAny) {
    let extrema_trade = decode_um_agg_trade(AGG_TRADE).expect("valid Extrema aggTrade");
    assert_eq!(extrema_trade.len(), 1);
    assert_eq!(extrema_trade[0].market, Market::BinanceUmFutures);
    assert_eq!(extrema_trade[0].trade_id, 5_933_014);
    assert_eq!(barter_trade(BARTER_TRADE).kind.id, "5933014");
    assert_eq!(
        nautilus_trade(AGG_TRADE, instrument).trade_id.as_str(),
        "5933014"
    );

    let extrema_bbo = decode_um_book_ticker(BOOK_TICKER).expect("valid Extrema bookTicker");
    assert!(matches!(extrema_bbo[0].event, LobEventKind::Bbo));
    assert_eq!(barter_book_ticker(BOOK_TICKER).instrument, INSTRUMENT_KEY);
    assert_eq!(
        nautilus_book_ticker(BOOK_TICKER, instrument).instrument_id,
        InstrumentId::from("BTCUSDT-PERP.BINANCE")
    );

    let extrema_depth = decode_um_diff_depth(DIFF_DEPTH).expect("valid Extrema diff depth");
    assert!(matches!(extrema_depth[0].event, LobEventKind::Incremental));
    assert!(matches!(
        extrema_depth[0].bids[0].action,
        LobLevelAction::Delete
    ));
    let barter_depth = barter_diff_depth(DIFF_DEPTH);
    let OrderBookEvent::Update(book) = barter_depth.kind else {
        panic!("expected Barter depth update")
    };
    assert_eq!(book.sequence(), 10_708_445_072_904);
    let nautilus_depth = nautilus_diff_depth(DIFF_DEPTH, instrument);
    assert_eq!(nautilus_depth.sequence, 10_708_445_072_904);
    assert_eq!(nautilus_depth.deltas.len(), 3);
}

fn bench_channels(c: &mut Criterion) {
    let instrument = btcusdt_usdm_perpetual();
    verify(&instrument);

    let mut trade = c.benchmark_group("trade_decode_native_normalize");
    trade.throughput(Throughput::Elements(1));
    trade.bench_function("nautilus", |b| {
        b.iter(|| black_box(nautilus_trade(black_box(AGG_TRADE), black_box(&instrument))))
    });
    trade.bench_function("barter", |b| {
        b.iter(|| black_box(barter_trade(black_box(BARTER_TRADE))))
    });
    trade.bench_function("extrema", |b| {
        b.iter(|| black_box(decode_um_agg_trade(black_box(AGG_TRADE)).unwrap()))
    });
    trade.finish();

    let mut bbo = c.benchmark_group("book_ticker_decode_native_normalize");
    bbo.throughput(Throughput::Elements(1));
    bbo.bench_function("extrema", |b| {
        b.iter(|| black_box(decode_um_book_ticker(black_box(BOOK_TICKER)).unwrap()))
    });
    bbo.bench_function("barter", |b| {
        b.iter(|| black_box(barter_book_ticker(black_box(BOOK_TICKER))))
    });
    bbo.bench_function("nautilus", |b| {
        b.iter(|| {
            black_box(nautilus_book_ticker(
                black_box(BOOK_TICKER),
                black_box(&instrument),
            ))
        })
    });
    bbo.finish();

    let mut depth = c.benchmark_group("depth_frame_decode_native_normalize");
    depth.throughput(Throughput::Elements(1));
    depth.bench_function("barter", |b| {
        b.iter(|| black_box(barter_diff_depth(black_box(DIFF_DEPTH))))
    });
    depth.bench_function("nautilus", |b| {
        b.iter(|| {
            black_box(nautilus_diff_depth(
                black_box(DIFF_DEPTH),
                black_box(&instrument),
            ))
        })
    });
    depth.bench_function("extrema", |b| {
        b.iter(|| black_box(decode_um_diff_depth(black_box(DIFF_DEPTH)).unwrap()))
    });
    depth.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .without_plots()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(100)
        .nresamples(100_000)
        .significance_level(0.05)
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_channels
}
criterion_main!(benches);
