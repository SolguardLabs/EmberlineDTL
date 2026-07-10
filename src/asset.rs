use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{AssetId, Bps, EmberError, EmberResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetSpec {
    pub id: AssetId,
    pub symbol: String,
    pub decimals: u8,
    pub settlement_precision: u8,
    pub risk_weight: Bps,
    pub min_pool_buffer: Bps,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetBook {
    assets: BTreeMap<AssetId, AssetSpec>,
    by_symbol: BTreeMap<String, AssetId>,
}

impl AssetSpec {
    pub fn stable(symbol: &str, decimals: u8) -> EmberResult<Self> {
        Ok(Self {
            id: AssetId::symbol(symbol),
            symbol: symbol.to_owned(),
            decimals,
            settlement_precision: decimals.min(9),
            risk_weight: Bps::new(1_000)?,
            min_pool_buffer: Bps::new(750)?,
        })
    }

    pub fn volatile(symbol: &str, decimals: u8, risk_weight: Bps) -> EmberResult<Self> {
        Ok(Self {
            id: AssetId::symbol(symbol),
            symbol: symbol.to_owned(),
            decimals,
            settlement_precision: decimals.min(9),
            risk_weight,
            min_pool_buffer: Bps::new(1_500)?,
        })
    }
}

impl AssetBook {
    pub fn with_defaults() -> EmberResult<Self> {
        let mut book = Self::default();
        book.insert(AssetSpec::stable("USDC", 6)?)?;
        book.insert(AssetSpec::stable("EURO", 6)?)?;
        book.insert(AssetSpec::volatile("SOL", 9, Bps::new(2_200)?)?)?;
        book.insert(AssetSpec::volatile("ETH", 9, Bps::new(1_800)?)?)?;
        Ok(book)
    }

    pub fn insert(&mut self, asset: AssetSpec) -> EmberResult<()> {
        self.by_symbol.insert(asset.symbol.clone(), asset.id);
        self.assets.insert(asset.id, asset);
        Ok(())
    }

    pub fn get(&self, id: AssetId) -> EmberResult<&AssetSpec> {
        self.assets.get(&id).ok_or(EmberError::AssetNotFound(id))
    }

    pub fn by_symbol(&self, symbol: &str) -> EmberResult<&AssetSpec> {
        let id = *self
            .by_symbol
            .get(symbol)
            .ok_or_else(|| EmberError::Policy(format!("asset symbol not registered: {symbol}")))?;
        self.get(id)
    }

    pub fn id_for(&self, symbol: &str) -> EmberResult<AssetId> {
        self.by_symbol(symbol).map(|asset| asset.id)
    }

    pub fn values(&self) -> impl Iterator<Item = &AssetSpec> {
        self.assets.values()
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}
