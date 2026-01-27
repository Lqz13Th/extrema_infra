use data_encoding::HEXUPPER;
use hmac::Mac;

use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use simd_json::from_slice;

use crate::arch::market_assets::api_general::*;
use crate::errors::{InfraError, InfraResult};

#[allow(dead_code)]
pub fn read_binance_env_key() -> InfraResult<BinanceKey> {
    let _ = dotenvy::dotenv();

    let api_key = std::env::var("BINANCE_API_KEY")
        .map_err(|_| InfraError::EnvVarMissing("BINANCE_API_KEY".into()))?;
    let secret_key = std::env::var("BINANCE_SECRET_KEY")
        .map_err(|_| InfraError::EnvVarMissing("BINANCE_SECRET_KEY".into()))?;

    Ok(BinanceKey::new(&api_key, &secret_key))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinanceKey {
    pub api_key: String,
    pub secret_key: String,
}

impl BinanceKey {
    fn new(api_key: &str, secret_key: &str) -> Self {
        Self {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
        }
    }

    fn sign(&self, query_string: &str, timestamp: u64) -> InfraResult<Signature<u64>> {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| InfraError::SecretKeyLength)?;
        mac.update(query_string.as_bytes());

        Ok(Signature {
            signature: HEXUPPER.encode(&mac.finalize().into_bytes()),
            timestamp,
        })
    }

    fn sign_now(&self, query_string: Option<&str>) -> InfraResult<Signature<u64>> {
        let timestamp = get_mills_timestamp();

        let query_with_timestamp = match query_string {
            Some(query) => format!("{}&timestamp={}", query, timestamp),
            None => format!("timestamp={}", timestamp),
        };

        self.sign(&query_with_timestamp, timestamp)
    }

    pub(crate) async fn get_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        query_string: Option<&str>,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let full_url = binance_build_full_url(url, query_string, signature);

        let res = client
            .get(&full_url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn post_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        query_string: Option<&str>,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let full_url = binance_build_full_url(url, query_string, signature);

        let res = client
            .post(&full_url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn put_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        query_string: Option<&str>,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let full_url = binance_build_full_url(url, query_string, signature);

        let res = client
            .put(&full_url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn send_signed_request<T>(
        &self,
        client: &Client,
        method: RequestMethod,
        query_string: Option<&str>,
        base_url: &str,
        endpoint: &str,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send,
    {
        let signature = self.sign_now(query_string)?;
        let url = [base_url, endpoint].concat();

        let mut response = match method {
            RequestMethod::Get => self.get_request(client, &signature, query_string, &url).await?,
            RequestMethod::Put => self.put_request(client, &signature, query_string, &url).await?,
            RequestMethod::Post => self.post_request(client, &signature, query_string, &url).await?,
        };

        let result: T = from_slice(&mut response)?;
        Ok(result)
    }
}

fn binance_build_full_url(
    url: &str,
    query_string: Option<&str>,
    signature: &Signature<u64>,
) -> String {
    match query_string {
        Some(query) => format!(
            "{}?{}&timestamp={}&signature={}",
            url, query, signature.timestamp, signature.signature
        ),
        None => format!(
            "{}?{}timestamp={}&signature={}",
            url, "", signature.timestamp, signature.signature
        ),
    }
}
