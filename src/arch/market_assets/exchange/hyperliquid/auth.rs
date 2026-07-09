use data_encoding::HEXLOWER;
use reqwest::Client;
use rmp_serde::to_vec_named;
use secp256k1::{Message, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha3::{Digest, Keccak256};

use crate::arch::market_assets::{
    api_general::{get_mills_timestamp, parse_json_response},
    exchange::secret::{redact_identifier, redact_secret},
};
use crate::errors::{InfraError, InfraResult};

use super::{api_utils::HyperliquidWithdraw3Action, config_assets::*};

pub fn read_hyperliquid_env_auth() -> InfraResult<HyperliquidAuth> {
    let _ = dotenvy::dotenv();

    let owner_address = std::env::var("HYPERLIQUID_OWNER_ADDRESS")
        .map_err(|_| InfraError::EnvVarMissing("HYPERLIQUID_OWNER_ADDRESS".into()))?
        .to_ascii_lowercase();
    let agent_private_key = std::env::var("HYPERLIQUID_AGENT_PRIVATE_KEY")
        .map_err(|_| InfraError::EnvVarMissing("HYPERLIQUID_AGENT_PRIVATE_KEY".into()))?;
    let withdraw_private_key = std::env::var("HYPERLIQUID_WITHDRAW_PRIVATE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let vault_address = std::env::var("HYPERLIQUID_VAULT_ADDRESS")
        .ok()
        .map(|address| address.to_ascii_lowercase());

    Ok(HyperliquidAuth {
        owner_address,
        agent_private_key,
        owner_private_key: withdraw_private_key,
        vault_address,
    })
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HyperliquidAuth {
    pub owner_address: String,
    pub agent_private_key: String,
    pub owner_private_key: Option<String>,
    pub vault_address: Option<String>,
}

impl std::fmt::Debug for HyperliquidAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperliquidAuth")
            .field("owner_address", &redact_identifier(&self.owner_address))
            .field("agent_private_key", &redact_secret())
            .field(
                "withdraw_private_key",
                &self.owner_private_key.as_ref().map(|_| redact_secret()),
            )
            .field(
                "vault_address",
                &self.vault_address.as_deref().map(redact_identifier),
            )
            .finish()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidSignature {
    pub r: String,
    pub s: String,
    pub v: u64,
}

#[derive(Clone, Debug, Serialize)]
struct HyperliquidAgent {
    source: String,
    #[serde(rename = "connectionId")]
    connection_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct HyperliquidExchangeRequest<'a, A>
where
    A: Serialize,
{
    action: &'a A,
    nonce: u64,
    signature: HyperliquidSignature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    vault_address: Option<&'a str>,
}

impl HyperliquidAuth {
    pub async fn send_withdraw3_raw<T>(
        &self,
        client: &Client,
        destination: &str,
        amount: &str,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send + std::fmt::Debug,
    {
        let nonce = get_mills_timestamp();
        let action = HyperliquidWithdraw3Action {
            kind: "withdraw3",
            destination: normalize_evm_address(destination)?,
            amount: amount.to_string(),
            time: nonce,
            signature_chain_id: HYPERLIQUID_DEFAULT_SIGNATURE_CHAIN_ID.to_string(),
            hyperliquid_chain: HYPERLIQUID_MAINNET_CHAIN.to_string(),
        };
        let signature = self.sign_withdraw3_action(&action)?;
        let body = HyperliquidExchangeRequest {
            action: &action,
            nonce,
            signature,
            vault_address: self.vault_address.as_deref(),
        };
        let body_string = serde_json::to_string(&body).map_err(|e| {
            InfraError::ApiCliError(format!(
                "Serialize Hyperliquid withdraw3 body failed: {}",
                e
            ))
        })?;
        let url = [HYPERLIQUID_BASE_URL, HYPERLIQUID_EXCHANGE].concat();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body_string)
            .send()
            .await?;

        parse_json_response("Hyperliquid POST withdraw3", response).await
    }

    pub fn sign_withdraw3_action(
        &self,
        action: &HyperliquidWithdraw3Action,
    ) -> InfraResult<HyperliquidSignature> {
        let digest = withdraw3_eip712_digest(action)?;
        let secret_key = parse_secret_key(self.owner_private_key.as_deref().ok_or_else(|| {
            InfraError::EnvVarMissing("HYPERLIQUID_WITHDRAW_PRIVATE_KEY".into())
        })?)?;
        sign_digest(&secret_key, &digest)
    }

    pub async fn send_signed_exchange_action_raw<T, A>(
        &self,
        client: &Client,
        action: &A,
    ) -> InfraResult<T>
    where
        T: DeserializeOwned + Send + std::fmt::Debug,
        A: Serialize,
    {
        let nonce = get_mills_timestamp();
        let signature = self.sign_l1_action(action, nonce, self.vault_address.as_deref())?;
        let body = HyperliquidExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: self.vault_address.as_deref(),
        };
        let body_string = serde_json::to_string(&body).map_err(|e| {
            InfraError::ApiCliError(format!("Serialize Hyperliquid exchange body failed: {}", e))
        })?;
        let url = [HYPERLIQUID_BASE_URL, HYPERLIQUID_EXCHANGE].concat();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body_string)
            .send()
            .await?;

        parse_json_response("Hyperliquid POST exchange", response).await
    }

    pub fn sign_l1_action<A>(
        &self,
        action: &A,
        nonce: u64,
        vault_address: Option<&str>,
    ) -> InfraResult<HyperliquidSignature>
    where
        A: Serialize,
    {
        let connection_id = self.action_hash(action, nonce, vault_address)?;
        let agent = HyperliquidAgent {
            source: HYPERLIQUID_MAINNET_SOURCE.to_string(),
            connection_id: format!("0x{}", HEXLOWER.encode(&connection_id)),
        };
        let digest = eip712_agent_digest(&agent)?;
        let secret_key = parse_secret_key(&self.agent_private_key)?;
        sign_digest(&secret_key, &digest)
    }

    fn action_hash<A>(
        &self,
        action: &A,
        nonce: u64,
        vault_address: Option<&str>,
    ) -> InfraResult<[u8; 32]>
    where
        A: Serialize,
    {
        let mut bytes = to_vec_named(action).map_err(|e| {
            InfraError::ApiCliError(format!("Serialize Hyperliquid action failed: {}", e))
        })?;
        bytes.extend(nonce.to_be_bytes());

        match vault_address {
            Some(address) => {
                bytes.push(1);
                bytes.extend(parse_address_bytes(address)?);
            },
            None => {
                bytes.push(0);
            },
        }

        Ok(keccak256(&bytes))
    }
}

fn withdraw3_eip712_digest(action: &HyperliquidWithdraw3Action) -> InfraResult<[u8; 32]> {
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let mut domain = Vec::with_capacity(32 * 5);
    domain.extend(domain_type_hash);
    domain.extend(keccak256(b"HyperliquidSignTransaction"));
    domain.extend(keccak256(b"1"));
    domain.extend(u256_bytes(parse_hex_u64(&action.signature_chain_id)?));
    domain.extend(address_to_word([0u8; 20]));
    let domain_separator = keccak256(&domain);

    let type_hash = keccak256(
        b"HyperliquidTransaction:Withdraw(string hyperliquidChain,string destination,string amount,uint64 time)",
    );
    let mut payload = Vec::with_capacity(32 * 5);
    payload.extend(type_hash);
    payload.extend(keccak256(action.hyperliquid_chain.as_bytes()));
    payload.extend(keccak256(action.destination.as_bytes()));
    payload.extend(keccak256(action.amount.as_bytes()));
    payload.extend(u256_bytes(action.time));
    let struct_hash = keccak256(&payload);

    let mut digest_input = Vec::with_capacity(66);
    digest_input.extend(b"\x19\x01");
    digest_input.extend(domain_separator);
    digest_input.extend(struct_hash);
    Ok(keccak256(&digest_input))
}

fn eip712_agent_digest(agent: &HyperliquidAgent) -> InfraResult<[u8; 32]> {
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(b"Exchange");
    let version_hash = keccak256(b"1");

    let mut domain = Vec::with_capacity(32 * 5);
    domain.extend(domain_type_hash);
    domain.extend(name_hash);
    domain.extend(version_hash);
    domain.extend(u256_bytes(1337));
    domain.extend(address_to_word([0u8; 20]));
    let domain_separator = keccak256(&domain);

    let agent_type_hash = keccak256(b"Agent(string source,bytes32 connectionId)");
    let source_hash = keccak256(agent.source.as_bytes());
    let connection_id = parse_bytes32(&agent.connection_id)?;

    let mut struct_bytes = Vec::with_capacity(32 * 3);
    struct_bytes.extend(agent_type_hash);
    struct_bytes.extend(source_hash);
    struct_bytes.extend(connection_id);
    let struct_hash = keccak256(&struct_bytes);

    let mut digest_input = Vec::with_capacity(66);
    digest_input.extend(b"\x19\x01");
    digest_input.extend(domain_separator);
    digest_input.extend(struct_hash);

    Ok(keccak256(&digest_input))
}

fn sign_digest(secret_key: &SecretKey, digest: &[u8; 32]) -> InfraResult<HyperliquidSignature> {
    let secp = Secp256k1::new();
    let message = Message::from_digest(*digest);
    let signature = secp.sign_ecdsa_recoverable(message, secret_key);
    let (recid, compact) = signature.serialize_compact();
    let (r, s) = compact.split_at(32);

    Ok(HyperliquidSignature {
        r: format!("0x{}", HEXLOWER.encode(r)),
        s: format!("0x{}", HEXLOWER.encode(s)),
        v: i32::from(recid) as u64 + 27,
    })
}

fn parse_secret_key(secret: &str) -> InfraResult<SecretKey> {
    let bytes = decode_hex(secret)?;
    let len = bytes.len();
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| InfraError::ApiCliError(format!("Invalid secret key length: {}", len)))?;
    SecretKey::from_byte_array(key_bytes)
        .map_err(|e| InfraError::ApiCliError(format!("Invalid secret key: {}", e)))
}

fn parse_address_bytes(address: &str) -> InfraResult<[u8; 20]> {
    let bytes = decode_hex(address)?;
    if bytes.len() != 20 {
        return Err(InfraError::ApiCliError(format!(
            "Invalid Hyperliquid address length: {}",
            bytes.len()
        )));
    }

    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_bytes32(hex_string: &str) -> InfraResult<[u8; 32]> {
    let bytes = decode_hex(hex_string)?;
    if bytes.len() != 32 {
        return Err(InfraError::ApiCliError(format!(
            "Invalid Hyperliquid bytes32 length: {}",
            bytes.len()
        )));
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn normalize_evm_address(address: &str) -> InfraResult<String> {
    let bytes = parse_address_bytes(address)?;
    Ok(format!("0x{}", HEXLOWER.encode(&bytes)))
}

fn decode_hex(input: &str) -> InfraResult<Vec<u8>> {
    let cleaned = input.trim_start_matches("0x");
    let normalized = if cleaned.len().is_multiple_of(2) {
        cleaned.to_string()
    } else {
        format!("0{}", cleaned)
    };

    let mut out = Vec::with_capacity(normalized.len() / 2);
    for chunk in normalized.as_bytes().chunks(2) {
        out.push((hex_value(chunk[0], input)? << 4) | hex_value(chunk[1], input)?);
    }

    Ok(out)
}

fn hex_value(b: u8, input: &str) -> InfraResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(InfraError::ApiCliError(format!(
            "Invalid hex string: {}",
            input
        ))),
    }
}

fn parse_hex_u64(value: &str) -> InfraResult<u64> {
    u64::from_str_radix(value.trim_start_matches("0x"), 16)
        .map_err(|e| InfraError::ApiCliError(format!("Invalid hex u64 {}: {}", value, e)))
}

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    hasher.finalize().into()
}

fn u256_bytes(value: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&value.to_be_bytes());
    out
}

fn address_to_word(address: [u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&address);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_hyperliquid_auth_secrets() {
        let auth = HyperliquidAuth {
            owner_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            agent_private_key:
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                    .to_string(),
            owner_private_key: Some(
                "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            ),
            vault_address: Some("0xfedcba0987654321fedcba0987654321fedcba09".to_string()),
        };
        let debug = format!("{:?}", auth);

        assert!(debug.contains("0x1234...5678"));
        assert!(debug.contains("0xfedc...ba09"));
        assert!(!debug.contains("0x1234567890abcdef1234567890abcdef12345678"));
        assert!(
            !debug.contains(
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            )
        );
        assert!(
            !debug.contains("0x1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert!(!debug.contains("0xfedcba0987654321fedcba0987654321fedcba09"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn signs_withdraw3_like_official_python_sdk() {
        let auth = HyperliquidAuth {
            owner_address: "0x5e9ee1089755c3435139848e47e6635505d5a13a".to_string(),
            agent_private_key:
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                    .to_string(),
            owner_private_key: Some(
                "0x0123456789012345678901234567890123456789012345678901234567890123".to_string(),
            ),
            vault_address: None,
        };
        let action = HyperliquidWithdraw3Action {
            kind: "withdraw3",
            destination: "0x5e9ee1089755c3435139848e47e6635505d5a13a".to_string(),
            amount: "1".to_string(),
            time: 1687816341423,
            signature_chain_id: HYPERLIQUID_DEFAULT_SIGNATURE_CHAIN_ID.to_string(),
            hyperliquid_chain: "Testnet".to_string(),
        };

        let signature = auth.sign_withdraw3_action(&action).unwrap();

        assert_eq!(
            signature.r,
            "0x8363524c799e90ce9bc41022f7c39b4e9bdba786e5f9c72b20e43e1462c37cf9"
        );
        assert_eq!(
            signature.s,
            "0x58b1411a775938b83e29182e8ef74975f9054c8e97ebf5ec2dc8d51bfc893881"
        );
        assert_eq!(signature.v, 28);
    }
}
