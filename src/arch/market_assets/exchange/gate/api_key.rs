use data_encoding::HEXLOWER;
use hmac::Mac;
use sha2::{Digest, Sha512};

use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use simd_json::from_slice;

use crate::arch::market_assets::api_general::*;
use crate::errors::{InfraError, InfraResult};

#[allow(dead_code)]
pub fn read_gate_env_key() -> InfraResult<GateKey> {
    let _ = dotenvy::dotenv();

    let api_key = std::env::var("GATE_API_KEY")
        .map_err(|_| InfraError::EnvVarMissing("GATE_API_KEY".into()))?;
    let secret_key = std::env::var("GATE_SECRET_KEY")
        .map_err(|_| InfraError::EnvVarMissing("GATE_SECRET_KEY".into()))?;
    let user_id = std::env::var("GATE_USER_ID")
        .map_err(|_| InfraError::EnvVarMissing("GATE_USER_ID".into()))?;

    Ok(GateKey::new(&api_key, &secret_key, &user_id))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateKey {
    pub api_key: String,
    pub secret_key: String,
    pub user_id: String,
}

impl GateKey {
    fn new(api_key: &str, secret_key: &str, user_id: &str) -> Self {
        Self {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
            user_id: user_id.into(),
        }
    }

    fn sign(
        &self,
        method: &str,
        full_path: &str,
        query_string: Option<&str>,
        body: Option<&str>,
        timestamp: u64,
    ) -> InfraResult<Signature<u64>> {
        let payload_hash = sha512_hex(body.unwrap_or(""));
        let query = query_string.unwrap_or("");
        let raw_sign = format!(
            "{}\n{}\n{}\n{}\n{}",
            method, full_path, query, payload_hash, timestamp
        );

        let mut mac = HmacSha512::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| InfraError::SecretKeyLength)?;
        mac.update(raw_sign.as_bytes());

        Ok(Signature {
            signature: HEXLOWER.encode(&mac.finalize().into_bytes()),
            timestamp,
        })
    }

    fn sign_now(
        &self,
        method: &str,
        full_path: &str,
        query_string: Option<&str>,
        body: Option<&str>,
    ) -> InfraResult<Signature<u64>> {
        let timestamp = get_seconds_timestamp();
        self.sign(method, full_path, query_string, body, timestamp)
    }

    pub fn ws_auth(&self, channel: &str, event: &str, time: u64) -> InfraResult<Value> {
        let raw_sign = format!("channel={}&event={}&time={}", channel, event, time);
        let mut mac = HmacSha512::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| InfraError::SecretKeyLength)?;
        mac.update(raw_sign.as_bytes());
        let signature = HEXLOWER.encode(&mac.finalize().into_bytes());

        Ok(json!({
            "method": "api_key",
            "KEY": self.api_key,
            "SIGN": signature,
        }))
    }

    pub(crate) async fn get_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .get(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn post_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .post(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn put_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
    ) -> InfraResult<Vec<u8>> {
        let res = client
            .put(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;

        Ok(res.bytes().await?.to_vec())
    }

    pub(crate) async fn send_signed_request<T>(
        &self,
        client: &Client,
        method: RequestMethod,
        query_string: Option<&str>,
        body: Option<&str>,
        base_url: &str,
        endpoint: &str,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send,
    {
        let method_str = match method {
            RequestMethod::Get => "GET",
            RequestMethod::Post => "POST",
            RequestMethod::Put => "PUT",
        };

        let signature = self.sign_now(method_str, endpoint, query_string, body)?;
        let url = gate_build_full_url(base_url, endpoint, query_string);

        let mut response = match method {
            RequestMethod::Get => self.get_request(client, &signature, &url).await?,
            RequestMethod::Post => {
                let body_str = body.unwrap_or("");
                self.post_request(client, &signature, body_str, &url)
                    .await?
            },
            RequestMethod::Put => {
                let body_str = body.unwrap_or("");
                self.put_request(client, &signature, body_str, &url).await?
            },
        };

        let result: T = from_slice(&mut response)?;
        Ok(result)
    }
}

fn sha512_hex(payload: &str) -> String {
    let mut hasher = Sha512::new();
    hasher.update(payload.as_bytes());
    HEXLOWER.encode(&hasher.finalize())
}

fn gate_build_full_url(base_url: &str, endpoint: &str, query_string: Option<&str>) -> String {
    match query_string {
        Some(query) if !query.is_empty() => format!("{}{}?{}", base_url, endpoint, query),
        _ => [base_url, endpoint].concat(),
    }
}
