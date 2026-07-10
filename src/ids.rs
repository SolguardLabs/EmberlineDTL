use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

const HASH_NAMESPACE: &[u8] = b"emberline-dtl/id/v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Digest([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AccountId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AssetId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct OperatorId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RouteId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SettlementId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ReservationId([u8; 32]);

impl Digest {
    pub const fn zero() -> Self {
        Self([0u8; 32])
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn domain(domain: &str, parts: &[&[u8]]) -> Self {
        Self(domain_hash(domain, parts))
    }

    pub fn from_serializable<T: Serialize>(domain: &str, value: &T) -> crate::EmberResult<Self> {
        let encoded = serde_json::to_vec(value)
            .map_err(|error| crate::EmberError::Serialization(error.to_string()))?;
        Ok(Self::domain(domain, &[encoded.as_slice()]))
    }
}

impl AccountId {
    pub fn named(label: &str) -> Self {
        Self(domain_hash("account", &[label.as_bytes()]))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl AssetId {
    pub fn symbol(symbol: &str) -> Self {
        Self(domain_hash("asset", &[symbol.as_bytes()]))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl OperatorId {
    pub fn named(label: &str) -> Self {
        Self(domain_hash("operator", &[label.as_bytes()]))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl RouteId {
    pub fn named(label: &str) -> Self {
        Self(domain_hash("route", &[label.as_bytes()]))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl SettlementId {
    pub fn derive(route: RouteId, nonce: u64) -> Self {
        Self(domain_hash(
            "settlement",
            &[&route.bytes(), &nonce.to_be_bytes()],
        ))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl ReservationId {
    pub fn derive(route: RouteId, operator: OperatorId, nonce: u64) -> Self {
        Self(domain_hash(
            "reservation",
            &[&route.bytes(), &operator.bytes(), &nonce.to_be_bytes()],
        ))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

fn domain_hash(domain: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(HASH_NAMESPACE);
    hasher.update(&(domain.len() as u64).to_be_bytes());
    hasher.update(domain.as_bytes());
    for part in parts {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

fn serialize_hex<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hex::encode(bytes))
}

fn deserialize_hex<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    let bytes = hex::decode(encoded).map_err(de::Error::custom)?;
    if bytes.len() != 32 {
        return Err(de::Error::custom(format!(
            "invalid id byte length: expected 32, received {}",
            bytes.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

macro_rules! hex_id {
    ($name:ident) => {
        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serialize_hex(&self.0, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(Self(deserialize_hex(deserializer)?))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", hex::encode(self.0))
            }
        }
    };
}

hex_id!(Digest);
hex_id!(AccountId);
hex_id!(AssetId);
hex_id!(OperatorId);
hex_id!(RouteId);
hex_id!(SettlementId);
hex_id!(ReservationId);
