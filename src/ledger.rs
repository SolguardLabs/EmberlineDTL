use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{AccountId, AssetId, EmberError, EmberResult, Event, EventKind, Units};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountState {
    pub id: AccountId,
    pub label: String,
    pub balances: BTreeMap<AssetId, Units>,
    pub reserved: BTreeMap<AssetId, Units>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Posting {
    pub account: AccountId,
    pub label: String,
    pub asset: AssetId,
    pub delta: i128,
    pub memo: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    accounts: BTreeMap<AccountId, AccountState>,
    by_label: BTreeMap<String, AccountId>,
    postings: Vec<Posting>,
}

impl AccountState {
    pub fn new(label: &str) -> Self {
        Self {
            id: AccountId::named(label),
            label: label.to_owned(),
            balances: BTreeMap::new(),
            reserved: BTreeMap::new(),
        }
    }

    pub fn balance(&self, asset: AssetId) -> Units {
        self.balances
            .get(&asset)
            .copied()
            .unwrap_or_else(Units::zero)
    }

    pub fn reserved(&self, asset: AssetId) -> Units {
        self.reserved
            .get(&asset)
            .copied()
            .unwrap_or_else(Units::zero)
    }

    pub fn available(&self, asset: AssetId) -> EmberResult<Units> {
        self.balance(asset).checked_sub(self.reserved(asset))
    }
}

impl Ledger {
    pub fn open_account(&mut self, label: &str) -> EmberResult<AccountId> {
        let account = AccountState::new(label);
        if self.accounts.contains_key(&account.id) {
            return Err(EmberError::AccountAlreadyExists(account.id));
        }
        let id = account.id;
        self.by_label.insert(label.to_owned(), id);
        self.accounts.insert(id, account);
        Ok(id)
    }

    pub fn ensure_account(&mut self, label: &str) -> EmberResult<AccountId> {
        if let Some(id) = self.by_label.get(label) {
            return Ok(*id);
        }
        self.open_account(label)
    }

    pub fn id_for(&self, label: &str) -> EmberResult<AccountId> {
        self.by_label
            .get(label)
            .copied()
            .ok_or_else(|| EmberError::Policy(format!("account label not registered: {label}")))
    }

    pub fn get(&self, id: AccountId) -> EmberResult<&AccountState> {
        self.accounts
            .get(&id)
            .ok_or(EmberError::AccountNotFound(id))
    }

    pub fn values(&self) -> impl Iterator<Item = &AccountState> {
        self.accounts.values()
    }

    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }

    pub fn credit(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Units,
        memo: &str,
    ) -> EmberResult<()> {
        self.apply_delta(account, asset, amount.raw() as i128, memo)
    }

    pub fn debit(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Units,
        memo: &str,
    ) -> EmberResult<()> {
        let state = self.get(account)?;
        let available = state.available(asset)?;
        if available < amount {
            return Err(EmberError::InsufficientBalance {
                account,
                available,
                required: amount,
            });
        }
        self.apply_delta(account, asset, -(amount.raw() as i128), memo)
    }

    pub fn reserve(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Units,
    ) -> EmberResult<()> {
        let state = self.get(account)?;
        let available = state.available(asset)?;
        if available < amount {
            return Err(EmberError::InsufficientBalance {
                account,
                available,
                required: amount,
            });
        }
        let account_state = self
            .accounts
            .get_mut(&account)
            .ok_or(EmberError::AccountNotFound(account))?;
        let current = account_state.reserved(asset);
        account_state
            .reserved
            .insert(asset, current.checked_add(amount)?);
        Ok(())
    }

    pub fn release(
        &mut self,
        account: AccountId,
        asset: AssetId,
        amount: Units,
    ) -> EmberResult<()> {
        let account_state = self
            .accounts
            .get_mut(&account)
            .ok_or(EmberError::AccountNotFound(account))?;
        let current = account_state.reserved(asset);
        account_state
            .reserved
            .insert(asset, current.checked_sub(amount)?);
        Ok(())
    }

    pub fn transfer(
        &mut self,
        from: AccountId,
        to: AccountId,
        asset: AssetId,
        amount: Units,
        memo: &str,
    ) -> EmberResult<()> {
        self.debit(from, asset, amount, memo)?;
        self.credit(to, asset, amount, memo)?;
        Ok(())
    }

    pub fn balance_of(&self, account: AccountId, asset: AssetId) -> EmberResult<Units> {
        Ok(self.get(account)?.balance(asset))
    }

    pub fn labeled_balances(&self, asset: AssetId) -> BTreeMap<String, Units> {
        self.accounts
            .values()
            .map(|account| (account.label.clone(), account.balance(asset)))
            .collect()
    }

    pub fn account_events(&self, clock_ms: u64) -> Vec<Event> {
        self.accounts
            .values()
            .map(|account| {
                Event::new(EventKind::AccountOpened, clock_ms, account.label.clone())
                    .account(account.id)
            })
            .collect()
    }

    fn apply_delta(
        &mut self,
        account: AccountId,
        asset: AssetId,
        delta: i128,
        memo: &str,
    ) -> EmberResult<()> {
        let state = self
            .accounts
            .get_mut(&account)
            .ok_or(EmberError::AccountNotFound(account))?;
        let current = state.balance(asset);
        let next = if delta >= 0 {
            current.checked_add(Units::new(delta as u128)?)?
        } else {
            current.checked_sub(Units::new(delta.unsigned_abs())?)?
        };
        state.balances.insert(asset, next);
        self.postings.push(Posting {
            account,
            label: state.label.clone(),
            asset,
            delta,
            memo: memo.to_owned(),
        });
        Ok(())
    }
}
