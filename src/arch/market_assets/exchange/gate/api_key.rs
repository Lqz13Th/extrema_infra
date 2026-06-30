use data_encoding::HEXLOWER;
use hmac::{KeyInit, Mac};
use sha2::{Digest, Sha512};

use reqwest::{Client, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::arch::market_assets::{
    api_general::*,
    exchange::secret::{redact_identifier, redact_secret},
};
use crate::errors::{InfraError, InfraResult};

use super::api_utils::{
    GATE_CHANNEL_ID_HEADER, GATE_SIZE_DECIMAL_HEADER, GATE_SIZE_DECIMAL_HEADER_VALUE,
};

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

#[derive(Clone, Serialize, Deserialize)]
pub struct GateKey {
    pub api_key: String,
    pub secret_key: String,
    pub user_id: String,
}

impl std::fmt::Debug for GateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateKey")
            .field("api_key", &redact_identifier(&self.api_key))
            .field("secret_key", &redact_secret())
            .field("user_id", &redact_identifier(&self.user_id))
            .finish()
    }
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

    pub fn sign_now(
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
    ) -> InfraResult<Response> {
        let res = client
            .get(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header(GATE_SIZE_DECIMAL_HEADER, GATE_SIZE_DECIMAL_HEADER_VALUE)
            .send()
            .await?;

        Ok(res)
    }

    pub(crate) async fn post_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
    ) -> InfraResult<Response> {
        self.post_request_with_channel_id(client, signature, body, url, None)
            .await
    }

    pub(crate) async fn post_request_with_channel_id(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
        channel_id: Option<&str>,
    ) -> InfraResult<Response> {
        let mut request = client
            .post(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header(GATE_SIZE_DECIMAL_HEADER, GATE_SIZE_DECIMAL_HEADER_VALUE)
            .header("Content-Type", "application/json");

        if let Some(channel_id) = channel_id {
            request = request.header(GATE_CHANNEL_ID_HEADER, channel_id);
        }

        let res = request.body(body.to_string()).send().await?;

        Ok(res)
    }

    pub(crate) async fn put_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
    ) -> InfraResult<Response> {
        let res = client
            .put(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header(GATE_SIZE_DECIMAL_HEADER, GATE_SIZE_DECIMAL_HEADER_VALUE)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;

        Ok(res)
    }

    pub(crate) async fn delete_request(
        &self,
        client: &Client,
        signature: &Signature<u64>,
        body: &str,
        url: &str,
    ) -> InfraResult<Response> {
        let res = client
            .delete(url)
            .header("KEY", &self.api_key)
            .header("SIGN", &signature.signature)
            .header("Timestamp", signature.timestamp.to_string())
            .header(GATE_SIZE_DECIMAL_HEADER, GATE_SIZE_DECIMAL_HEADER_VALUE)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;

        Ok(res)
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
            RequestMethod::Delete => "DELETE",
        };

        let encoded_query = encode_query_string(query_string);
        let signature = self.sign_now(method_str, endpoint, encoded_query.as_deref(), body)?;
        let url = gate_build_full_url(base_url, endpoint, encoded_query.as_deref());

        let response = match method {
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
            RequestMethod::Delete => {
                let body_str = body.unwrap_or("");
                self.delete_request(client, &signature, body_str, &url)
                    .await?
            },
        };

        let label = format!("Gate {:?} {}", method, endpoint);
        parse_json_response(&label, response).await
    }

    pub(crate) async fn send_signed_post_request_with_channel_id<T>(
        &self,
        client: &Client,
        query_string: Option<&str>,
        body: Option<&str>,
        base_url: &str,
        endpoint: &str,
        channel_id: Option<&str>,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send,
    {
        let encoded_query = encode_query_string(query_string);
        let body_str = body.unwrap_or("");
        let signature =
            self.sign_now("POST", endpoint, encoded_query.as_deref(), Some(body_str))?;
        let url = gate_build_full_url(base_url, endpoint, encoded_query.as_deref());
        let response = self
            .post_request_with_channel_id(client, &signature, body_str, &url, channel_id)
            .await?;

        let label = format!("Gate Post {}", endpoint);
        parse_json_response(&label, response).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_gate_key_secrets() {
        let key = GateKey::new(
            "gate_api_key_1234567890",
            "gate_secret_key_1234567890",
            "52955084",
        );
        let debug = format!("{:?}", key);

        assert!(debug.contains("gate_a...7890"));
        assert!(!debug.contains("gate_api_key_1234567890"));
        assert!(!debug.contains("gate_secret_key_1234567890"));
        assert!(!debug.contains("52955084"));
        assert!(debug.contains("[REDACTED]"));
    }
}
