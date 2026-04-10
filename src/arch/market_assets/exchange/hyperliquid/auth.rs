use data_encoding::HEXLOWER;
use reqwest::Client;
use rmp_serde::to_vec_named;
use secp256k1::{Message, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha3::{Digest, Keccak256};
use simd_json::from_slice;

use crate::arch::traits::conversion::IntoInfraVec;
use crate::errors::{InfraError, InfraResult};

use super::{
    config_assets::{HYPERLIQUID_BASE_URL, HYPERLIQUID_EXCHANGE, HYPERLIQUID_MAINNET_SOURCE},
    hyperliquid_rest_msg::RestResHyperliquid,
};

pub fn read_hyperliquid_env_auth() -> InfraResult<HyperliquidAuth> {
    let _ = dotenvy::dotenv();

    let owner_address = std::env::var("HYPERLIQUID_OWNER_ADDRESS")
        .map_err(|_| InfraError::EnvVarMissing("HYPERLIQUID_OWNER_ADDRESS".into()))?
        .to_ascii_lowercase();
    let agent_private_key = std::env::var("HYPERLIQUID_AGENT_PRIVATE_KEY")
        .map_err(|_| InfraError::EnvVarMissing("HYPERLIQUID_AGENT_PRIVATE_KEY".into()))?;
    let target_address = std::env::var("HYPERLIQUID_TARGET_ADDRESS")
        .ok()
        .map(|address| address.to_ascii_lowercase());

    Ok(HyperliquidAuth {
        owner_address,
        agent_private_key,
        target_address,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperliquidAuth {
    pub owner_address: String,
    pub agent_private_key: String,
    pub target_address: Option<String>,
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
    pub async fn send_signed_exchange_action<T, A>(
        &self,
        client: &Client,
        action: &A,
        nonce: u64,
    ) -> InfraResult<Vec<T>>
    where
        T: DeserializeOwned + Send + std::fmt::Debug,
        A: Serialize,
    {
        let signature = self.sign_l1_action(action, nonce, self.target_address.as_deref())?;
        let body = HyperliquidExchangeRequest {
            action,
            nonce,
            signature,
            vault_address: self.target_address.as_deref(),
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

        let mut response = response.bytes().await?.to_vec();
        let result: RestResHyperliquid<T> = from_slice(&mut response)?;
        result.into_vec()
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
