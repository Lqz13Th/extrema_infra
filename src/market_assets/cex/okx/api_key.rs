use hmac::Mac;
use data_encoding::BASE64;

use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use serde_json::from_str;
use reqwest::Client;

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::api_general::*;


#[allow(dead_code)]
pub fn read_okx_env_key() -> InfraResult<OkxKey> {
    let _ = dotenv::dotenv();

    let api_key = std::env::var("OKX_API_KEY")
        .map_err(|_| InfraError::EnvVarMissing("OKX_API_KEY".to_string()))?;
    let secret_key = std::env::var("OKX_SECRET_KEY")
        .map_err(|_| InfraError::EnvVarMissing("OKX_SECRET_KEY".to_string()))?;
    let passphrase = std::env::var("OKX_PASSPHRASE")
        .map_err(|_| InfraError::EnvVarMissing("OKX_PASSPHRASE".to_string()))?;

    Ok(OkxKey::new(&api_key, &secret_key, &passphrase))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OkxKey {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
}

impl OkxKey {
    fn new(api_key: &str, secret_key: &str, passphrase: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            passphrase: passphrase.to_string(),
        }
    }

    fn sign(
        &self,
        raw_sign: String,
        timestamp: u64,
    ) -> InfraResult<Signature<String>> {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|_| InfraError::SecretKeyLength)?;
        mac.update(raw_sign.as_bytes());

        Ok(Signature {
            signature: BASE64.encode(&mac.finalize().into_bytes()),
            timestamp: timestamp.to_string(),
        })
    }

    fn sign_now(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>
    ) -> InfraResult<Signature<String>> {
        let timestamp = get_timestamp();

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
    ) -> InfraResult<String> {
        let res = client
            .post(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| InfraError::RestApi(e))?;

        Ok(res.text().await.map_err(|e| InfraError::RestApi(e))?)
    }

    pub(crate) async fn get_request(
        &self,
        client: &Client,
        signature: &Signature<String>,
        body: String,
        url: &str,
    ) -> InfraResult<String> {
        let res = client
            .get(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| InfraError::RestApi(e))?;

        Ok(res.text().await.map_err(|e| InfraError::RestApi(e))?)
    }

    pub(crate) async fn put_request(
        &self,
        client: &Client,
        signature: &Signature<String>,
        body: String,
        url: &str,
    ) -> InfraResult<String> {
        let res = client
            .put(url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", &signature.signature)
            .header("OK-ACCESS-TIMESTAMP", &signature.timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| InfraError::RestApi(e))?;

        Ok(res.text().await.map_err(|e| InfraError::RestApi(e))?)
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

        let response = match method {
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

        let result: T = from_str(&response).map_err(|e| InfraError::Json(e))?;
        Ok(result)
    }
}
