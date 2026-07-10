use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    AccountingSnapshot, Admission, AssetBook, AssetId, Bps, EmberError, EmberResult, Event,
    EventKind, InvariantSet, Ledger, OperatorBook, RebatePool, RebateQuote, ReservationId, RouteId,
    RoutePlan, SettlementId, SettlementPolicy, SettlementReceipt, Units,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EngineConfig {
    pub network_id: u64,
    pub primary_asset: AssetId,
    pub pool_label: String,
    pub settlement_nonce: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteState {
    pub plan: RoutePlan,
    pub reservation: Option<ReservationId>,
    pub admission: Option<Admission>,
    pub receipt: Option<SettlementReceipt>,
    pub quote: Option<RebateQuote>,
    pub executed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementEngine {
    pub config: EngineConfig,
    pub assets: AssetBook,
    pub operators: OperatorBook,
    pub ledger: Ledger,
    pub policy: SettlementPolicy,
    pub pool: RebatePool,
    pub routes: BTreeMap<RouteId, RouteState>,
    pub events: Vec<Event>,
    pub clock_ms: u64,
}

impl EngineConfig {
    pub fn local(primary_asset: AssetId) -> Self {
        Self {
            network_id: SettlementPolicy::default().network_id,
            primary_asset,
            pool_label: "rebate-pool".to_owned(),
            settlement_nonce: 0,
        }
    }
}

impl SettlementEngine {
    pub fn bootstrap() -> EmberResult<Self> {
        let assets = AssetBook::with_defaults()?;
        let primary_asset = assets.id_for("USDC")?;
        let operators = OperatorBook::with_defaults()?;
        let mut ledger = Ledger::default();
        ledger.ensure_account("treasury")?;
        ledger.ensure_account("rebate-pool")?;
        ledger.ensure_account("settlement-clearing")?;
        for operator in operators.values() {
            ledger.ensure_account(&operator.account_label)?;
        }
        let mut pool = RebatePool::new(primary_asset);
        pool.fund(Units::new(8_000_000)?)?;
        let mut engine = Self {
            config: EngineConfig::local(primary_asset),
            assets,
            operators,
            ledger,
            policy: SettlementPolicy::default(),
            pool,
            routes: BTreeMap::new(),
            events: Vec::new(),
            clock_ms: 1_700_000_000,
        };
        let pool_account = engine.ledger.id_for("rebate-pool")?;
        engine.ledger.credit(
            pool_account,
            primary_asset,
            Units::new(8_000_000)?,
            "pool funding",
        )?;
        engine.events.push(
            Event::new(
                EventKind::PoolFunded,
                engine.clock_ms,
                "initial rebate pool",
            )
            .asset(primary_asset)
            .amount(Units::new(8_000_000)?),
        );
        engine
            .events
            .extend(engine.ledger.account_events(engine.clock_ms));
        Ok(engine)
    }

    pub fn submit_route(&mut self, plan: RoutePlan) -> EmberResult<Admission> {
        if self.routes.contains_key(&plan.id) {
            return Err(EmberError::RouteAlreadyExists(plan.id));
        }
        let operator = self.operators.get(plan.operator)?;
        let decision = self.policy.decide_route(&plan, operator)?;
        if !decision.admitted {
            return Err(EmberError::Policy(decision.reason));
        }
        let reservation_id =
            ReservationId::derive(plan.id, plan.operator, self.config.settlement_nonce);
        let reservation = self
            .pool
            .reserve(reservation_id, plan.id, decision.reserve_amount)?;
        let risk = crate::RiskEnvelope::from_route(&plan, operator, reservation.amount)?;
        let admission = risk.admit(operator.reserve_limit(), reservation.amount)?;
        if !admission.accepted {
            return Err(EmberError::Policy(admission.reason));
        }
        self.events.push(
            Event::new(EventKind::RouteSubmitted, self.clock_ms, plan.label.clone())
                .route(plan.id)
                .operator(plan.operator)
                .asset(plan.asset)
                .amount(plan.notional),
        );
        self.events.push(
            Event::new(
                EventKind::PoolReserved,
                self.clock_ms,
                "rebate budget reserved",
            )
            .route(plan.id)
            .reservation(reservation_id)
            .operator(plan.operator)
            .asset(plan.asset)
            .amount(reservation.amount),
        );
        self.events.push(
            Event::new(
                EventKind::RouteAdmitted,
                self.clock_ms,
                admission.reason.clone(),
            )
            .route(plan.id)
            .reservation(reservation_id)
            .operator(plan.operator),
        );
        self.routes.insert(
            plan.id,
            RouteState {
                plan,
                reservation: Some(reservation_id),
                admission: Some(admission.clone()),
                receipt: None,
                quote: None,
                executed: false,
            },
        );
        self.config.settlement_nonce = self
            .config
            .settlement_nonce
            .checked_add(1)
            .ok_or_else(|| EmberError::Policy("settlement nonce overflow".to_owned()))?;
        Ok(admission)
    }

    pub fn execute_route(
        &mut self,
        route: RouteId,
        gross_delta: Units,
        compensation_delta: Units,
        elapsed_ms: u64,
    ) -> EmberResult<SettlementId> {
        let state = self
            .routes
            .get(&route)
            .cloned()
            .ok_or(EmberError::RouteNotFound(route))?;
        if state.executed {
            let settlement = state
                .receipt
                .map(|receipt| receipt.id)
                .ok_or_else(|| EmberError::Policy("executed route without receipt".to_owned()))?;
            return Err(EmberError::SettlementAlreadyExecuted(settlement));
        }
        let reservation_id = state
            .reservation
            .ok_or(EmberError::RouteNotAdmitted(route))?;
        let reservation = self.pool.reservation(reservation_id)?.clone();
        let receipt = SettlementReceipt::from_route(
            &state.plan,
            self.config.settlement_nonce,
            gross_delta,
            compensation_delta,
            elapsed_ms,
            self.clock_ms.saturating_add(elapsed_ms),
        )?;
        let operator = self.operators.get(receipt.operator)?.clone();
        let quote = self
            .policy
            .quote_rebate(&state.plan, &receipt, &operator, &reservation)?;
        let applied =
            self.pool
                .apply_execution(reservation_id, quote.operator_rebate, quote.pool_penalty)?;
        let operator_account = self.ledger.id_for(&operator.account_label)?;
        self.ledger.credit(
            operator_account,
            self.config.primary_asset,
            applied.paid,
            "operator rebate",
        )?;
        let pool_account = self.ledger.id_for(&self.config.pool_label)?;
        self.ledger.debit(
            pool_account,
            self.config.primary_asset,
            applied.paid,
            "operator rebate",
        )?;
        self.operators
            .get_mut(operator.id)?
            .record_execution(applied.paid, applied.penalty)?;
        self.events.push(
            Event::new(
                EventKind::SettlementExecuted,
                receipt.finalized_at_ms,
                state.plan.label,
            )
            .route(route)
            .settlement(receipt.id)
            .reservation(reservation_id)
            .operator(operator.id)
            .asset(self.config.primary_asset)
            .amount(receipt.output_notional),
        );
        self.events.push(
            Event::new(
                EventKind::RebatePaid,
                receipt.finalized_at_ms,
                "operator rebate",
            )
            .route(route)
            .settlement(receipt.id)
            .reservation(reservation_id)
            .operator(operator.id)
            .asset(self.config.primary_asset)
            .amount(applied.paid),
        );
        if !applied.penalty.is_zero() {
            self.events.push(
                Event::new(
                    EventKind::PenaltyApplied,
                    receipt.finalized_at_ms,
                    "pool penalty",
                )
                .route(route)
                .settlement(receipt.id)
                .reservation(reservation_id)
                .operator(operator.id)
                .asset(self.config.primary_asset)
                .amount(applied.penalty),
            );
        }
        if !applied.released.is_zero() {
            self.events.push(
                Event::new(
                    EventKind::ReservationReleased,
                    receipt.finalized_at_ms,
                    "unused reserve released",
                )
                .route(route)
                .settlement(receipt.id)
                .reservation(reservation_id)
                .operator(operator.id)
                .asset(self.config.primary_asset)
                .amount(applied.released),
            );
        }
        let settlement_id = receipt.id;
        let current = self
            .routes
            .get_mut(&route)
            .ok_or(EmberError::RouteNotFound(route))?;
        current.receipt = Some(receipt);
        current.quote = Some(quote);
        current.executed = true;
        self.config.settlement_nonce = self
            .config
            .settlement_nonce
            .checked_add(1)
            .ok_or_else(|| EmberError::Policy("settlement nonce overflow".to_owned()))?;
        self.clock_ms = self.clock_ms.saturating_add(elapsed_ms);
        Ok(settlement_id)
    }

    pub fn route_by_label(&self, label: &str) -> EmberResult<RouteId> {
        self.routes
            .values()
            .find(|state| state.plan.label == label)
            .map(|state| state.plan.id)
            .ok_or_else(|| EmberError::Policy(format!("route label not found: {label}")))
    }

    pub fn accounting_snapshot(&self) -> AccountingSnapshot {
        AccountingSnapshot::from_pool(
            self.pool.account(),
            self.ledger.labeled_balances(self.config.primary_asset),
        )
    }

    pub fn invariants(&self) -> InvariantSet {
        let reservations_conserved = self.pool.conservation_holds();
        let route_links_valid = self.routes.values().all(|state| {
            state
                .receipt
                .as_ref()
                .is_none_or(|receipt| receipt.route == state.plan.id)
                && state
                    .quote
                    .as_ref()
                    .is_none_or(|quote| quote.gross_reserved_cost >= quote.operator_rebate)
        });
        let operators_non_negative = self
            .operators
            .values()
            .all(|operator| operator.rebates_earned >= Units::zero());
        InvariantSet {
            pool_non_negative: self.pool.account().available >= Units::zero()
                && self.pool.account().reserved >= Units::zero(),
            reservations_conserved,
            operators_non_negative,
            route_links_valid,
            reports_deterministic: true,
        }
    }

    pub fn pool_pressure_bps(&self) -> EmberResult<Bps> {
        let account = self.pool.account();
        let total = account
            .available
            .checked_add(account.reserved)?
            .checked_add(account.paid)?;
        account
            .reserved
            .ratio_bps_uncapped(total.max(Units::from(1u64)))
    }
}
