use data_encoding::HEXLOWER;
use reqwest::Client;
use rmp_serde::to_vec;
use secp256k1::{Message, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use sha3::{Digest, Keccak256};
use simd_json::from_slice;

use crate::errors::{InfraError, InfraResult};

const EIP712_DOMAIN_NAME: &str = "Exchange";
const EIP712_DOMAIN_VERSION: &str = "1";
const EIP712_CHAIN_ID: u64 = 1337;
const EIP712_VERIFYING_CONTRACT: [u8; 20] = [0u8; 20];

#[allow(dead_code)]
pub fn read_hyperliquid_env_key() -> InfraResult<HyperliquidKey> {
    let _ = dotenvy::dotenv();

    let private_key = std::env::var("HYPERLIQUID_PRIVATE_KEY")
        .map_err(|_| InfraError::EnvVarMissing("HYPERLIQUID_PRIVATE_KEY".into()))?;

    HyperliquidKey::new_checked(&private_key)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperliquidKey {
    pub private_key: String,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyperliquidSignature {
    pub r: String,
    pub s: String,
    pub v: u8,
}

#[derive(Clone, Debug)]
pub struct HyperliquidSignedRequest<'a> {
    pub client: &'a Client,
    pub action: &'a serde_json::Value,
    pub nonce: u64,
    pub vault_address: Option<&'a str>,
    pub expires_after: Option<u64>,
    pub is_mainnet: bool,
    pub base_url: &'a str,
    pub endpoint: &'a str,
}

impl HyperliquidKey {
    pub fn new(private_key: &str) -> Self {
        let normalized = normalize_hex(private_key);
        let address = derive_eth_address_from_hex(&normalized).unwrap_or_default();
        Self {
            private_key: normalized,
            address,
        }
    }

    pub fn new_checked(private_key: &str) -> InfraResult<Self> {
        let key_bytes = decode_hex_32(private_key)?;
        let address = derive_eth_address(&key_bytes)?;
        Ok(Self {
            private_key: normalize_hex(private_key),
            address,
        })
    }

    pub fn sign_l1_action(
        &self,
        action: &serde_json::Value,
        nonce: u64,
        vault_address: Option<&str>,
        is_mainnet: bool,
    ) -> InfraResult<HyperliquidSignature> {
        let action_bytes = to_vec(action).map_err(|e| InfraError::Msg(e.to_string()))?;
        let vault_bytes = vault_address
            .map(decode_hex_20)
            .transpose()?
            .unwrap_or([0u8; 20]);

        let mut payload = Vec::with_capacity(action_bytes.len() + 28);
        payload.extend_from_slice(&action_bytes);
        payload.extend_from_slice(&vault_bytes);
        payload.extend_from_slice(&nonce.to_be_bytes());

        let connection_id = keccak256(&payload);
        let digest = eip712_digest(connection_id, is_mainnet);

        sign_digest(&self.private_key, &digest)
    }

    pub async fn send_signed_request<T>(&self, req: HyperliquidSignedRequest<'_>) -> InfraResult<T>
    where
        T: DeserializeOwned + Send,
    {
        let signature =
            self.sign_l1_action(req.action, req.nonce, req.vault_address, req.is_mainnet)?;

        let mut body = json!({
            "action": req.action,
            "nonce": req.nonce,
            "signature": signature
        });

        if let Some(v) = req.expires_after {
            body["expiresAfter"] = json!(v);
        }

        let url = [req.base_url, req.endpoint].concat();
        let res = req
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await?;

        let mut res_bytes = res.bytes().await?.to_vec();
        let value: T = from_slice(&mut res_bytes)?;
        Ok(value)
    }
}

fn sign_digest(private_key: &str, digest: &[u8; 32]) -> InfraResult<HyperliquidSignature> {
    let key_bytes = decode_hex_32(private_key)?;
    let secret_key = SecretKey::from_byte_array(key_bytes)
        .map_err(|_| InfraError::Msg("Invalid Hyperliquid private key".into()))?;

    let secp = Secp256k1::new();
    let msg = Message::from_digest(*digest);
    let sig = secp.sign_ecdsa_recoverable(msg, &secret_key);
    let (rec_id, sig_bytes) = sig.serialize_compact();

    let r = hex_with_prefix(&sig_bytes[0..32]);
    let s = hex_with_prefix(&sig_bytes[32..64]);
    let v = i32::from(rec_id) as u8 + 27;

    Ok(HyperliquidSignature { r, s, v })
}

fn eip712_digest(connection_id: [u8; 32], is_mainnet: bool) -> [u8; 32] {
    let domain_separator = eip712_domain_separator();
    let message_hash = agent_struct_hash(connection_id, is_mainnet);

    let mut buf = Vec::with_capacity(2 + 32 + 32);
    buf.extend_from_slice(b"\x19\x01");
    buf.extend_from_slice(&domain_separator);
    buf.extend_from_slice(&message_hash);
    keccak256(&buf)
}

fn eip712_domain_separator() -> [u8; 32] {
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(EIP712_DOMAIN_NAME.as_bytes());
    let version_hash = keccak256(EIP712_DOMAIN_VERSION.as_bytes());

    let mut enc = Vec::with_capacity(32 * 5);
    enc.extend_from_slice(&type_hash);
    enc.extend_from_slice(&name_hash);
    enc.extend_from_slice(&version_hash);
    enc.extend_from_slice(&encode_u256(EIP712_CHAIN_ID));
    enc.extend_from_slice(&encode_address(EIP712_VERIFYING_CONTRACT));

    keccak256(&enc)
}

fn agent_struct_hash(connection_id: [u8; 32], is_mainnet: bool) -> [u8; 32] {
    let type_hash = keccak256(b"Agent(string source,bytes32 connectionId)");
    let source = if is_mainnet { "a" } else { "b" };
    let source_hash = keccak256(source.as_bytes());

    let mut enc = Vec::with_capacity(32 * 3);
    enc.extend_from_slice(&type_hash);
    enc.extend_from_slice(&source_hash);
    enc.extend_from_slice(&connection_id);

    keccak256(&enc)
}

fn encode_u256(value: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..32].copy_from_slice(&value.to_be_bytes());
    out
}

fn encode_address(addr: [u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..32].copy_from_slice(&addr);
    out
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}

fn normalize_hex(input: &str) -> String {
    input.trim().trim_start_matches("0x").to_ascii_lowercase()
}

fn decode_hex_32(input: &str) -> InfraResult<[u8; 32]> {
    let hex = normalize_hex(input);
    if hex.len() != 64 {
        return Err(InfraError::Msg("Private key must be 32 bytes hex".into()));
    }
    let bytes = HEXLOWER
        .decode(hex.as_bytes())
        .map_err(|_| InfraError::Msg("Invalid hex in private key".into()))?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_hex_20(input: &str) -> InfraResult<[u8; 20]> {
    let hex = normalize_hex(input);
    if hex.len() != 40 {
        return Err(InfraError::Msg("Address must be 20 bytes hex".into()));
    }
    let bytes = HEXLOWER
        .decode(hex.as_bytes())
        .map_err(|_| InfraError::Msg("Invalid hex in address".into()))?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn derive_eth_address(private_key: &[u8; 32]) -> InfraResult<String> {
    let secret_key = SecretKey::from_byte_array(*private_key)
        .map_err(|_| InfraError::Msg("Invalid Hyperliquid private key".into()))?;
    let secp = Secp256k1::new();
    let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let pubkey_uncompressed = pubkey.serialize_uncompressed();
    let hash = keccak256(&pubkey_uncompressed[1..]);
    Ok(hex_with_prefix(&hash[12..32]))
}

fn derive_eth_address_from_hex(private_key: &str) -> InfraResult<String> {
    let key_bytes = decode_hex_32(private_key)?;
    derive_eth_address(&key_bytes)
}

fn hex_with_prefix(bytes: &[u8]) -> String {
    format!("0x{}", HEXLOWER.encode(bytes))
}
