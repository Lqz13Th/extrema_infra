//! Read-only Hyperliquid REST API usage.
//!
//! This example shows how to call Hyperliquid through the LOB API traits:
//! public instrument metadata, tickers, live funding rates, funding metadata,
//! and optional read-only account balances/positions.
//!
//! It never places orders or signs exchange actions. Account read calls run only
//! when an owner address is available in the environment.
//!
//! Run the public-only flow:
//!
//! ```text
//! cargo run --example hyperliquid_api_usage_example --features hyperliquid
//! ```
//!
//! Pass a different perpetual instrument as the first argument:
//!
//! ```text
//! cargo run --example hyperliquid_api_usage_example --features hyperliquid -- ETH_USDC_PERP
//! ```
//!
//! Enable the optional account read calls with:
//!
//! ```text
//! HYPERLIQUID_OWNER_ADDRESS=0x...
//! HYPERLIQUID_VAULT_ADDRESS=0x... # optional
//! ```

use std::env;

use extrema_infra::{
    arch::market_assets::{
        base_data::InstrumentType,
        exchange::prelude::{HyperliquidAuth, HyperliquidCli},
    },
    prelude::*,
};

#[tokio::main]
async fn main() -> InfraResult<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let mut cli = HyperliquidCli::default();

    let perp_inst = env::args()
        .nth(1)
        .unwrap_or_else(|| "BTC_USDC_PERP".to_string());

    let perps = cli.get_instrument_info(InstrumentType::Perpetual).await?;
    println!("perpetual instruments: {}", perps.len());
    for inst in perps.iter().take(5) {
        println!(
            "  {} code={}",
            inst.inst,
            inst.inst_code.as_deref().unwrap_or("-")
        );
    }

    let tickers = cli
        .get_tickers(
            Some(std::slice::from_ref(&perp_inst)),
            Some(InstrumentType::Perpetual),
        )
        .await?;
    println!("{perp_inst} ticker: {tickers:#?}");

    let funding = cli.get_funding_rate_live(Some(&perp_inst)).await?;
    println!("{perp_inst} live funding: {funding:#?}");

    let funding_info = cli.get_funding_rate_info(Some(&perp_inst)).await?;
    println!("{perp_inst} funding info: {funding_info:#?}");

    if let Ok(owner_address) = env::var("HYPERLIQUID_OWNER_ADDRESS") {
        cli.auth = Some(HyperliquidAuth {
            owner_address: owner_address.to_ascii_lowercase(),
            agent_private_key: String::new(),
            vault_address: env::var("HYPERLIQUID_VAULT_ADDRESS")
                .ok()
                .map(|address| address.to_ascii_lowercase()),
        });

        let usdc = vec!["USDC".to_string()];
        let balances = cli.get_balance(Some(&usdc)).await?;
        println!("USDC balances: {balances:#?}");

        let positions = cli
            .get_positions(Some(std::slice::from_ref(&perp_inst)))
            .await?;
        println!("{perp_inst} positions: {positions:#?}");
    } else {
        println!(
            "Skipping account read calls. Set HYPERLIQUID_OWNER_ADDRESS to fetch \
             balances and positions."
        );
    }

    Ok(())
}
