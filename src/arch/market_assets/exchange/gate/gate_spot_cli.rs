use reqwest::Client;
use std::sync::Arc;
use tracing::error;

use crate::arch::traits::market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi};

use super::api_key::{GateKey, read_gate_env_key};

#[derive(Clone, Debug)]
pub struct GateSpotCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateSpotCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl GateSpotCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }
}

impl MarketLobApi for GateSpotCli {}

impl LobPublicRest for GateSpotCli {}

impl LobPrivateRest for GateSpotCli {
    fn init_api_key(&mut self) {
        match read_gate_env_key() {
            Ok(gate_key) => {
                self.api_key = Some(gate_key);
            },
            Err(e) => {
                error!("Failed to read GATE env key: {:?}", e);
            },
        };
    }
}

impl LobWebsocket for GateSpotCli {}
