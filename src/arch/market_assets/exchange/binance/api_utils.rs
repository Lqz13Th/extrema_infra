use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

use crate::arch::{
    market_assets::base_data::SUBSCRIBE,
    task_execution::task_ws::{LobFrequency, LobParam},
};
use crate::errors::{InfraError, InfraResult};

use super::api_key::BinanceKey;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BinanceListenKey {
    pub listenKey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinanceUniversalTransferType {
    MainUmFuture,
    UmFutureMain,
    MainCmFuture,
    CmFutureMain,
    MainMargin,
    MarginMain,
    MainFunding,
    FundingMain,
    MainOption,
    OptionMain,
    MainPortfolioMargin,
    PortfolioMarginMain,
    IsolatedMarginMargin,
    MarginIsolatedMargin,
}

impl BinanceUniversalTransferType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::MainUmFuture => "MAIN_UMFUTURE",
            Self::UmFutureMain => "UMFUTURE_MAIN",
            Self::MainCmFuture => "MAIN_CMFUTURE",
            Self::CmFutureMain => "CMFUTURE_MAIN",
            Self::MainMargin => "MAIN_MARGIN",
            Self::MarginMain => "MARGIN_MAIN",
            Self::MainFunding => "MAIN_FUNDING",
            Self::FundingMain => "FUNDING_MAIN",
            Self::MainOption => "MAIN_OPTION",
            Self::OptionMain => "OPTION_MAIN",
            Self::MainPortfolioMargin => "MAIN_PORTFOLIO_MARGIN",
            Self::PortfolioMarginMain => "PORTFOLIO_MARGIN_MAIN",
            Self::IsolatedMarginMargin => "ISOLATEDMARGIN_MARGIN",
            Self::MarginIsolatedMargin => "MARGIN_ISOLATEDMARGIN",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceUniversalTransferReq {
    pub transfer_type: BinanceUniversalTransferType,
    pub asset: String,
    pub amount: String,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
    pub recv_window: Option<u64>,
}

impl BinanceUniversalTransferReq {
    pub(crate) fn to_query_string(&self) -> String {
        let mut parts = vec![
            format!("type={}", self.transfer_type.as_str()),
            format!("asset={}", self.asset),
            format!("amount={}", self.amount),
        ];

        if let Some(from_symbol) = self.from_symbol.as_deref() {
            parts.push(format!("fromSymbol={from_symbol}"));
        }
        if let Some(to_symbol) = self.to_symbol.as_deref() {
            parts.push(format!("toSymbol={to_symbol}"));
        }
        if let Some(recv_window) = self.recv_window {
            parts.push(format!("recvWindow={recv_window}"));
        }

        parts.join("&")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceUniversalTransferHistoryReq {
    pub transfer_type: BinanceUniversalTransferType,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub current: Option<u32>,
    pub size: Option<u32>,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
    pub recv_window: Option<u64>,
}

impl BinanceUniversalTransferHistoryReq {
    pub fn new(transfer_type: BinanceUniversalTransferType) -> Self {
        Self {
            transfer_type,
            start_time: None,
            end_time: None,
            current: None,
            size: None,
            from_symbol: None,
            to_symbol: None,
            recv_window: None,
        }
    }

    pub(crate) fn to_query_string(&self) -> String {
        let mut parts = vec![format!("type={}", self.transfer_type.as_str())];

        if let Some(start) = self.start_time {
            parts.push(format!("startTime={start}"));
        }
        if let Some(end) = self.end_time {
            parts.push(format!("endTime={end}"));
        }
        if let Some(current) = self.current {
            parts.push(format!("current={current}"));
        }
        if let Some(size) = self.size {
            parts.push(format!("size={size}"));
        }
        if let Some(from_symbol) = self.from_symbol.as_deref() {
            parts.push(format!("fromSymbol={from_symbol}"));
        }
        if let Some(to_symbol) = self.to_symbol.as_deref() {
            parts.push(format!("toSymbol={to_symbol}"));
        }
        if let Some(window) = self.recv_window {
            parts.push(format!("recvWindow={window}"));
        }

        parts.join("&")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinanceSubAccountTransferAccountType {
    Spot,
    UsdtFuture,
    CoinFuture,
    Margin,
    IsolatedMargin,
    Alpha,
}

impl BinanceSubAccountTransferAccountType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Spot => "SPOT",
            Self::UsdtFuture => "USDT_FUTURE",
            Self::CoinFuture => "COIN_FUTURE",
            Self::Margin => "MARGIN",
            Self::IsolatedMargin => "ISOLATED_MARGIN",
            Self::Alpha => "ALPHA",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceSubAccountUniversalTransferReq {
    pub from_email: Option<String>,
    pub to_email: Option<String>,
    pub from_account_type: BinanceSubAccountTransferAccountType,
    pub to_account_type: BinanceSubAccountTransferAccountType,
    pub client_tran_id: Option<String>,
    pub symbol: Option<String>,
    pub asset: String,
    pub amount: String,
    pub recv_window: Option<u64>,
}

impl BinanceSubAccountUniversalTransferReq {
    pub(crate) fn to_query_string(&self) -> InfraResult<String> {
        if self.from_account_type == self.to_account_type
            && self.from_email.is_none()
            && self.to_email.is_none()
        {
            return Err(InfraError::ApiCliError(
                "Binance sub-account universal transfer requires fromEmail or toEmail when account types are the same".into(),
            ));
        }

        let mut parts = vec![
            format!("fromAccountType={}", self.from_account_type.as_str()),
            format!("toAccountType={}", self.to_account_type.as_str()),
            format!("asset={}", self.asset.to_uppercase()),
            format!("amount={}", self.amount),
        ];

        if let Some(from_email) = self.from_email.as_deref() {
            parts.push(format!("fromEmail={from_email}"));
        }
        if let Some(to_email) = self.to_email.as_deref() {
            parts.push(format!("toEmail={to_email}"));
        }
        if let Some(client_tran_id) = self.client_tran_id.as_deref() {
            parts.push(format!("clientTranId={client_tran_id}"));
        }
        if let Some(symbol) = self.symbol.as_deref() {
            parts.push(format!("symbol={symbol}"));
        }
        if let Some(recv_window) = self.recv_window {
            parts.push(format!("recvWindow={recv_window}"));
        }

        Ok(parts.join("&"))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceSubAccountUniversalTransferHistoryReq {
    pub from_email: Option<String>,
    pub to_email: Option<String>,
    pub client_tran_id: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub recv_window: Option<u64>,
}

impl BinanceSubAccountUniversalTransferHistoryReq {
    pub(crate) fn to_query_string(&self) -> InfraResult<Option<String>> {
        if self.from_email.is_some() && self.to_email.is_some() {
            return Err(InfraError::ApiCliError(
                "Binance sub-account transfer history cannot send fromEmail and toEmail together"
                    .into(),
            ));
        }

        let mut parts: Vec<String> = Vec::new();

        if let Some(from_email) = self.from_email.as_deref() {
            parts.push(format!("fromEmail={from_email}"));
        }
        if let Some(to_email) = self.to_email.as_deref() {
            parts.push(format!("toEmail={to_email}"));
        }
        if let Some(client_tran_id) = self.client_tran_id.as_deref() {
            parts.push(format!("clientTranId={client_tran_id}"));
        }
        if let Some(start_time) = self.start_time {
            parts.push(format!("startTime={start_time}"));
        }
        if let Some(end_time) = self.end_time {
            parts.push(format!("endTime={end_time}"));
        }
        if let Some(page) = self.page {
            parts.push(format!("page={page}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(recv_window) = self.recv_window {
            parts.push(format!("recvWindow={recv_window}"));
        }

        if parts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parts.join("&")))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceDepositHistoryReq {
    pub coin: Option<String>,
    pub status: Option<i32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub include_source: Option<bool>,
    pub tx_id: Option<String>,
    pub recv_window: Option<u64>,
}

impl BinanceDepositHistoryReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(coin) = self.coin.as_deref() {
            parts.push(format!("coin={}", coin.to_uppercase()));
        }
        if let Some(status) = self.status {
            parts.push(format!("status={status}"));
        }
        if let Some(start) = self.start_time {
            parts.push(format!("startTime={start}"));
        }
        if let Some(end) = self.end_time {
            parts.push(format!("endTime={end}"));
        }
        if let Some(offset) = self.offset {
            parts.push(format!("offset={offset}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(include_source) = self.include_source {
            parts.push(format!("includeSource={include_source}"));
        }
        if let Some(tx_id) = self.tx_id.as_deref() {
            parts.push(format!("txId={tx_id}"));
        }
        if let Some(window) = self.recv_window {
            parts.push(format!("recvWindow={window}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("&"))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWithdrawHistoryReq {
    pub coin: Option<String>,
    pub withdraw_order_id: Option<String>,
    pub status: Option<i32>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub id_list: Option<Vec<String>>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub recv_window: Option<u64>,
}

impl BinanceWithdrawHistoryReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(coin) = self.coin.as_deref() {
            parts.push(format!("coin={}", coin.to_uppercase()));
        }
        if let Some(woid) = self.withdraw_order_id.as_deref() {
            parts.push(format!("withdrawOrderId={woid}"));
        }
        if let Some(status) = self.status {
            parts.push(format!("status={status}"));
        }
        if let Some(offset) = self.offset {
            parts.push(format!("offset={offset}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(ids) = self.id_list.as_deref()
            && !ids.is_empty()
        {
            parts.push(format!("idList={}", ids.join(",")));
        }
        if let Some(start) = self.start_time {
            parts.push(format!("startTime={start}"));
        }
        if let Some(end) = self.end_time {
            parts.push(format!("endTime={end}"));
        }
        if let Some(window) = self.recv_window {
            parts.push(format!("recvWindow={window}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("&"))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWithdrawReq {
    pub coin: String,
    pub address: String,
    pub amount: String,
    pub network: Option<String>,
    pub address_tag: Option<String>,
    pub withdraw_order_id: Option<String>,
    pub transaction_fee_flag: Option<bool>,
    pub name: Option<String>,
    pub wallet_type: Option<u8>,
    pub recv_window: Option<u64>,
}

impl BinanceWithdrawReq {
    pub(crate) fn to_query_string(&self) -> String {
        let mut parts = vec![
            format!("coin={}", self.coin),
            format!("address={}", self.address),
            format!("amount={}", self.amount),
        ];

        if let Some(network) = self.network.as_deref() {
            parts.push(format!("network={network}"));
        }
        if let Some(address_tag) = self.address_tag.as_deref() {
            parts.push(format!("addressTag={address_tag}"));
        }
        if let Some(withdraw_order_id) = self.withdraw_order_id.as_deref() {
            parts.push(format!("withdrawOrderId={withdraw_order_id}"));
        }
        if let Some(transaction_fee_flag) = self.transaction_fee_flag {
            parts.push(format!("transactionFeeFlag={transaction_fee_flag}"));
        }
        if let Some(name) = self.name.as_deref() {
            parts.push(format!("name={name}"));
        }
        if let Some(wallet_type) = self.wallet_type {
            parts.push(format!("walletType={wallet_type}"));
        }
        if let Some(recv_window) = self.recv_window {
            parts.push(format!("recvWindow={recv_window}"));
        }

        parts.join("&")
    }
}

fn create_binance_cli_with_key<C, F>(
    keys: HashMap<String, BinanceKey>,
    shared_client: Arc<Client>,
    make_cli: F,
) -> HashMap<String, C>
where
    F: Fn(Arc<Client>, BinanceKey) -> C,
{
    keys.into_iter()
        .map(|(id, key)| (id, make_cli(Arc::clone(&shared_client), key)))
        .collect()
}

pub fn ws_subscribe_msg_binance(param: &str, insts: Option<&[String]>) -> String {
    let params: Vec<String> = match insts {
        Some(list) => list
            .iter()
            .map(|symbol| format!("{}@{}", cli_perp_to_pure_lowercase(symbol), param))
            .collect(),
        None => vec![param.into()],
    };

    let subscribe_msg = json!({
        "method": SUBSCRIBE,
        "params": params,
        "id": 1
    });

    subscribe_msg.to_string()
}

pub fn ws_subscribe_msg_binance_cm(param: &str, insts: Option<&[String]>) -> String {
    let params: Vec<String> = match insts {
        Some(list) => list
            .iter()
            .map(|symbol| {
                format!(
                    "{}@{}",
                    cli_perp_to_binance_cm(symbol).to_lowercase(),
                    param
                )
            })
            .collect(),
        None => vec![param.into()],
    };

    let subscribe_msg = json!({
        "method": SUBSCRIBE,
        "params": params,
        "id": 1
    });

    subscribe_msg.to_string()
}

pub fn binance_lob_stream(lob_param: &Option<LobParam>) -> InfraResult<String> {
    match lob_param {
        None => Ok(format!("depth{}", binance_lob_frequency_suffix(&None)?)),
        Some(LobParam::Bbo { frequency }) => match frequency {
            None | Some(LobFrequency::Realtime) => Ok("bookTicker".into()),
            Some(freq) => Err(InfraError::ApiCliError(format!(
                "Binance bookTicker does not support requested frequency: {:?}",
                freq
            ))),
        },
        Some(LobParam::Snapshot { depth, frequency }) => {
            let depth = match depth.as_ref().copied() {
                None => 20,
                Some(depth @ (5 | 10 | 20)) => depth,
                Some(depth) => {
                    return Err(InfraError::ApiCliError(format!(
                        "Binance partial depth supports only 5, 10, or 20 levels: {}",
                        depth
                    )));
                },
            };

            Ok(format!(
                "depth{}{}",
                depth,
                binance_lob_frequency_suffix(frequency)?
            ))
        },
        Some(LobParam::Incremental { depth, frequency }) => {
            if depth.is_some() {
                return Err(InfraError::ApiCliError(format!(
                    "Binance diff depth does not support a depth parameter: {:?}",
                    depth
                )));
            }

            Ok(format!("depth{}", binance_lob_frequency_suffix(frequency)?))
        },
    }
}

fn binance_lob_frequency_suffix(frequency: &Option<LobFrequency>) -> InfraResult<&'static str> {
    match frequency {
        None | Some(LobFrequency::Ms250) => Ok(""),
        Some(LobFrequency::Ms100) => Ok("@100ms"),
        Some(LobFrequency::Ms500) => Ok("@500ms"),
        Some(freq) => Err(InfraError::ApiCliError(format!(
            "Binance LOB supports only 100ms, 250ms, or 500ms frequency: {:?}",
            freq
        ))),
    }
}

pub fn binance_fut_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let quote_currencies = ["USDT", "USDC", "USD"];
    let (pair, suffix) = match upper.rsplit_once('_') {
        Some((pair, suffix)) => (pair, Some(suffix)),
        None => (upper.as_str(), None),
    };

    for quote in quote_currencies {
        if let Some(base) = pair.strip_suffix(quote) {
            if base.is_empty() {
                warn!("Invalid Binance symbol: {}", symbol);
                return symbol.into();
            }

            return match suffix {
                Some("PERP") => format!("{}_{}_PERP", base, quote),
                Some(expiry) if expiry.chars().all(|c| c.is_ascii_digit()) => {
                    format!("{}_{}_FUT_{}", base, quote, expiry)
                },
                Some(other) => {
                    warn!("Unknown Binance futures suffix: {}", other);
                    upper
                },
                None => format!("{}_{}_PERP", base, quote),
            };
        }
    }

    upper
}

pub fn binance_spot_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let quote_currencies = [
        "USDT", "USDC", "USD1", "FDUSD", "TUSD", "USDP", "BUSD", "DAI", "BTC", "ETH", "BNB", "JPY",
        "USD",
    ];

    for quote in quote_currencies {
        if upper.ends_with(quote) {
            let base = &upper[..upper.len() - quote.len()];
            if base.is_empty() {
                warn!("Invalid Binance spot symbol: {}", symbol);
                return symbol.into();
            }
            return format!("{}_{}", base, quote);
        }
    }

    upper
}

pub fn cli_perp_to_pure_lowercase(symbol: &str) -> String {
    cli_um_to_binance_symbol(symbol).to_lowercase()
}

pub fn cli_perp_to_pure_uppercase(symbol: &str) -> String {
    cli_um_to_binance_symbol(symbol).to_uppercase()
}

fn cli_um_to_binance_symbol(symbol: &str) -> String {
    if let Some(cleaned) = symbol.strip_suffix("_PERP") {
        return cleaned.replace("_", "");
    }

    if let Some((pair, expiry)) = symbol.rsplit_once("_FUT_") {
        return format!("{}_{}", pair.replace("_", ""), expiry);
    }

    symbol.replace("_", "")
}

pub fn cli_perp_to_binance_cm(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if let Some(pair) = upper.strip_suffix("_PERP") {
        return format!("{}_PERP", normalize_binance_cm_pair(pair));
    }

    if let Some((pair, expiry)) = upper.rsplit_once("_FUT_") {
        return format!("{}_{}", normalize_binance_cm_pair(pair), expiry);
    }

    normalize_binance_cm_pair(&upper)
}

pub fn cli_perp_to_binance_cm_pair(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if let Some(pair) = upper.strip_suffix("_PERP") {
        return normalize_binance_cm_pair(pair);
    }

    if let Some((pair, _)) = upper.rsplit_once("_FUT_") {
        return normalize_binance_cm_pair(pair);
    }

    normalize_binance_cm_pair(&upper)
}

fn normalize_binance_cm_pair(pair: &str) -> String {
    if let Some(base) = pair.strip_suffix("_USDT") {
        format!("{}USD", base.replace('_', ""))
    } else if let Some(base) = pair.strip_suffix("USDT") {
        format!("{base}USD")
    } else {
        pair.replace('_', "")
    }
}

pub fn cli_spot_to_binance_spot(inst: &str) -> String {
    inst.replace('_', "").to_uppercase()
}

#[cfg(test)]
mod tests {
    use crate::arch::task_execution::task_ws::{LobFrequency, LobParam};

    use super::*;

    #[test]
    fn converts_binance_cm_between_cli_and_native_symbol() {
        assert_eq!(binance_fut_inst_to_cli("BTCUSD_PERP"), "BTC_USD_PERP");
        assert_eq!(cli_perp_to_binance_cm("BTC_USD_PERP"), "BTCUSD_PERP");
        assert_eq!(cli_perp_to_binance_cm("BTC_USDT_PERP"), "BTCUSD_PERP");
        assert_eq!(
            cli_perp_to_binance_cm("BTC_USD_FUT_240329"),
            "BTCUSD_240329"
        );
    }

    #[test]
    fn converts_binance_cm_cli_to_pair_for_pair_endpoints() {
        assert_eq!(cli_perp_to_binance_cm_pair("BTC_USD_PERP"), "BTCUSD");
        assert_eq!(cli_perp_to_binance_cm_pair("BTC_USDT_PERP"), "BTCUSD");
        assert_eq!(cli_perp_to_binance_cm_pair("BTC_USD_FUT_240329"), "BTCUSD");
    }

    #[test]
    fn builds_binance_lob_stream_names() {
        assert_eq!(binance_lob_stream(&None).unwrap(), "depth");
        assert_eq!(
            binance_lob_stream(&Some(LobParam::Bbo { frequency: None })).unwrap(),
            "bookTicker"
        );
        assert_eq!(
            binance_lob_stream(&Some(LobParam::Snapshot {
                depth: None,
                frequency: Some(LobFrequency::Ms100),
            }))
            .unwrap(),
            "depth20@100ms"
        );
        assert_eq!(
            binance_lob_stream(&Some(LobParam::Snapshot {
                depth: Some(10),
                frequency: Some(LobFrequency::Ms500),
            }))
            .unwrap(),
            "depth10@500ms"
        );
        assert_eq!(
            binance_lob_stream(&Some(LobParam::Incremental {
                depth: None,
                frequency: Some(LobFrequency::Ms250),
            }))
            .unwrap(),
            "depth"
        );
    }

    #[test]
    fn rejects_unsupported_binance_lob_requests() {
        assert!(
            binance_lob_stream(&Some(LobParam::Bbo {
                frequency: Some(LobFrequency::Ms100),
            }))
            .is_err()
        );
        assert!(
            binance_lob_stream(&Some(LobParam::Snapshot {
                depth: Some(50),
                frequency: None,
            }))
            .is_err()
        );
        assert!(
            binance_lob_stream(&Some(LobParam::Incremental {
                depth: Some(20),
                frequency: None,
            }))
            .is_err()
        );
    }

    #[test]
    fn builds_binance_lob_subscribe_symbols() {
        let um_insts = vec!["BTC_USDT_PERP".into()];
        let um_msg: serde_json::Value =
            serde_json::from_str(&ws_subscribe_msg_binance("depth20@100ms", Some(&um_insts)))
                .unwrap();
        assert_eq!(um_msg["params"][0], "btcusdt@depth20@100ms");

        let cm_insts = vec!["BTC_USD_PERP".into()];
        let cm_msg: serde_json::Value =
            serde_json::from_str(&ws_subscribe_msg_binance_cm("bookTicker", Some(&cm_insts)))
                .unwrap();
        assert_eq!(cm_msg["params"][0], "btcusd_perp@bookTicker");
    }

    #[test]
    fn builds_binance_sub_account_universal_transfer_query() {
        let req = BinanceSubAccountUniversalTransferReq {
            from_email: None,
            to_email: Some("sub@example.com".into()),
            from_account_type: BinanceSubAccountTransferAccountType::Spot,
            to_account_type: BinanceSubAccountTransferAccountType::UsdtFuture,
            client_tran_id: Some("client-1".into()),
            symbol: None,
            asset: "usdt".into(),
            amount: "1.25".into(),
            recv_window: Some(5_000),
        };

        assert_eq!(
            req.to_query_string().unwrap(),
            "fromAccountType=SPOT&toAccountType=USDT_FUTURE&asset=USDT&amount=1.25&toEmail=sub@example.com&clientTranId=client-1&recvWindow=5000"
        );
    }

    #[test]
    fn rejects_binance_same_account_type_transfer_without_email() {
        let req = BinanceSubAccountUniversalTransferReq {
            from_email: None,
            to_email: None,
            from_account_type: BinanceSubAccountTransferAccountType::Spot,
            to_account_type: BinanceSubAccountTransferAccountType::Spot,
            client_tran_id: None,
            symbol: None,
            asset: "USDT".into(),
            amount: "1".into(),
            recv_window: None,
        };

        assert!(req.to_query_string().is_err());
    }

    #[test]
    fn builds_binance_sub_account_transfer_history_query() {
        let empty_req = BinanceSubAccountUniversalTransferHistoryReq::default();
        assert_eq!(empty_req.to_query_string().unwrap(), None);

        let req = BinanceSubAccountUniversalTransferHistoryReq {
            from_email: Some("from@example.com".into()),
            to_email: None,
            client_tran_id: Some("client-2".into()),
            start_time: Some(1),
            end_time: Some(2),
            page: Some(3),
            limit: Some(4),
            recv_window: None,
        };

        assert_eq!(
            req.to_query_string().unwrap().as_deref(),
            Some(
                "fromEmail=from@example.com&clientTranId=client-2&startTime=1&endTime=2&page=3&limit=4"
            )
        );
    }

    #[test]
    fn rejects_binance_sub_account_transfer_history_with_both_emails() {
        let req = BinanceSubAccountUniversalTransferHistoryReq {
            from_email: Some("from@example.com".into()),
            to_email: Some("to@example.com".into()),
            ..Default::default()
        };

        assert!(req.to_query_string().is_err());
    }
}
