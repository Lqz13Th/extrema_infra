use data_encoding::BASE64;
use hmac::Mac;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use simd_json::from_slice;

use crate::arch::market_assets::api_general::*;
use crate::errors::{InfraError, InfraResult};

use super::api_utils::get_okx_timestamp;

pub fn read_okx_env_key() -> InfraResult<OkxKey> {
    let _ = dotenvy::dotenv();

    let api_key = std::env::var("OKX_API_KEY")
        .map_err(|_| InfraError::EnvVarMissing("OKX_API_KEY".into()))?;
    let secret_key = std::env::var("OKX_SECRET_KEY")
        .map_err(|_| InfraError::EnvVarMissing("OKX_SECRET_KEY".into()))?;
    let passphrase = std::env::var("OKX_PASSPHRASE")
        .map_err(|_| InfraError::EnvVarMissing("OKX_PASSPHRASE".into()))?;

    Ok(OkxKey::new(&api_key, &secret_key, &passphrase))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OkxKey {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
}

impl OkxKey {
    fn new(api_key: &str, secret_key: &str, passphrase: &str) -> Self {
        Self {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
            passphrase: passphrase.into(),
        }
    }

    pub fn sign(&self, raw_sign: String, timestamp: String) -> InfraResult<Signature<String>> {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| InfraError::SecretKeyLength)?;
        mac.update(raw_sign.as_bytes());

        Ok(Signature {
            signature: BASE64.encode(&mac.finalize().into_bytes()),
            timestamp,
        })
    }

    pub fn sign_now(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>,
    ) -> InfraResult<Signature<String>> {
        let timestamp = get_okx_timestamp();

        let raw_sign = match body {
            Some(b) => format!("{}{}{}{}", timestamp, method, uri, b),
            None => format!("{}{}{}", timestamp, method, uri),
        };

        self.sign(raw_sign, timestamp)
    }

    pub(crate) async fn post_request(
        &self,
        client: &Client,
        signature: &Signature<String>,
        body: String,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .post(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn get_request(
        &self,
        client: &Client,
        signature: &Signature<String>,
        body: String,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .get(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn put_request(
        &self,
        client: &Client,
        signature: &Signature<String>,
        body: String,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .put(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn send_signed_request<T>(
        &self,
        client: &Client,
        method: RequestMethod,
        body: String,
        base_url: &str,
        endpoint: &str,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send,
    {
        let url = [base_url, endpoint].concat();

        let mut response = match method {
            RequestMethod::Get => {
                let signature = self.sign_now("GET", endpoint, Some(&body))?;
                self.get_request(client, &signature, body, &url).await?
            },
            RequestMethod::Post => {
                let signature = self.sign_now("POST", endpoint, Some(&body))?;
                self.post_request(client, &signature, body, &url).await?
            },
            RequestMethod::Put => {
                let signature = self.sign_now("PUT", endpoint, Some(&body))?;
                self.put_request(client, &signature, body, &url).await?
            },
        };

        let result: T = from_slice(&mut response)?;
        Ok(result)
    }
}
