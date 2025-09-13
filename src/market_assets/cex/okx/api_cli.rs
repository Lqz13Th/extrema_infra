// use std::future::{ready, Future};
// use std::sync::Arc;
// use reqwest::Client;
// use serde::de::DeserializeOwned;
// use serde_json::{from_str, Value};
// 
// use crate::errors::*;
// use crate::traits::market_cex::*;
// use crate::market_assets::{
//     account_data::*,
//     market_data::*,
//     rules_data::*,
// };
// use crate::market_assets::api_utils::RequestMethod;
// 
// pub struct OkxApi {
//     pub client: Client,
//     pub api_key: Option<String>,
//     pub secret: Option<String>,
//     pub passphrase: Option<String>,
// }
// 
// impl OkxApi {
//     pub fn new(pub_only: bool) -> Self {
//         let client = Client::new();
//         if pub_only {
//             Self {
//                 client,
//                 api_key: None,
//                 secret: None,
//                 passphrase: None,
//             }
//         } else {
//             Self {
//                 client,
//                 api_key: Some("YOUR_KEY".into()),
//                 secret: Some("YOUR_SECRET".into()),
//                 passphrase: Some("YOUR_PASSPHRASE".into()),
//             }
//         }
//     }
// }
// 
// impl CexPublicRest for OkxApi {
//     async fn get_ticker(
//         &self,
//         symbols: Vec<String>,
//     ) -> InfraResult<Vec<TickerData>> {
//         Ok(symbols.into_iter().map(|s| TickerData {
//             symbol: s,
//             bid: 0.0,
//             ask: 0.0,
//             last: 0.0,
//             volume: 0.0,
//             timestamp: 0,
//         }).collect())
//     }
// 
//     async fn get_orderbook(
//         &self,
//         symbols: Vec<String>,
//         _depth: usize
//     ) -> InfraResult<Vec<OrderBookData>> {
//         Ok(symbols.into_iter().map(|s| OrderBookData {
//             symbol: s,
//             bids: vec![],
//             asks: vec![],
//             timestamp: 0,
//         }).collect())
//     }
// 
//     async fn get_candles(
//         &self,
//         symbols: Vec<String>,
//         interval: &str
//     ) -> InfraResult<Vec<CandleData>> {
//         Ok(symbols.into_iter().map(|s| CandleData {
//             symbol: s,
//             open: 0.0,
//             high: 0.0,
//             low: 0.0,
//             close: 0.0,
//             volume: 0.0,
//             timestamp: 0,
//         }).collect())
//     }
// 
//     async fn get_market_info(
//         &self,
//         symbols: Vec<String>,
//     ) -> InfraResult<Vec<MarketInfoData>> {
//         Ok(symbols.into_iter().map(|s| MarketInfoData {
//             symbol: s,
//             min_order_size: 0.0,
//             max_order_size: 0.0,
//             price_precision: 0,
//             lot_size: 0.0,
//         }).collect())
//     }
// }
// 
// impl CexPrivateRest for OkxApi {
//     async fn get_balance(
//         &self,
//         assets: Vec<String>,
//     ) -> InfraResult<Vec<BalanceData>> {
//         Ok(assets.into_iter().map(|a| BalanceData {
//             asset: a,
//             free: 0.0,
//             locked: 0.0,
//         }).collect())
//     }
// 
//     async fn get_position(
//         &self,
//         symbols: Vec<String>,
//     ) -> InfraResult<Vec<PositionData>> {
//         Ok(symbols.into_iter().map(|s| PositionData {
//             symbol: s,
//             side: "".to_string(),
//             size: 0.0,
//             entry_price: 0.0,
//             unrealized_pnl: 0.0,
//         }).collect())
//     }
// 
// 
// }
// 
// // 完整统一 trait
// impl MarketCexApi for OkxApi {
//     fn init_api_key(self) -> Self {
//         self
//     }
// 
//     async fn send_request<T>(
//         &self,
//         method: RequestMethod,
//         args: Option<&Value>,
//         endpoint: &str,
//     ) -> InfraResult<T>
//     where
//         T: DeserializeOwned + Send,
//     {
//         todo!()
//         // let api_keys = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;
//         // let signature = api_keys.sign_now(args)?;
//         //
//         // let base_url = match self.binance_market {
//         //     BinanceMarket::Spot => crate::market_assets::cex::binance::uri_assets::SPOT_BASE_URL,
//         //     BinanceMarket::UmFutures => crate::market_assets::cex::binance::uri_assets::UM_FUTURES_BASE_URL,
//         //     BinanceMarket::CmFutures => crate::market_assets::cex::binance::uri_assets::CM_FUTURES_BASE_URL,
//         // };
//         // let url = [base_url, endpoint].concat();
//         //
//         // let response = match method {
//         //     RequestMethod::Get => {
//         //         api_keys.get_request(&self.client, &signature, args, &url).await?
//         //     },
//         //     RequestMethod::Put => {
//         //         api_keys.put_request(&self.client, &signature, args, &url).await?
//         //     },
//         //     RequestMethod::Post => {
//         //         api_keys.post_request(&self.client, &signature, args, &url).await?
//         //     },
//         // };
//         //
//         // let result: T = from_str(&response)?;
//         // Ok(result)
//     }
// }
