import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCommon,
  assertPoolConservation,
  operator,
  route,
  scenario,
} from "../helpers/emberline.js";

test("la ejecucion normal cierra reserva y paga rebate sin penalizacion", () => {
  const report = scenario("normal");
  assertCommon(report, "normal");
  assert.equal(report.routes.length, 1);

  const item = route(report, "normal-mainnet-euro");
  assert.equal(item.executed, true);
  assert.equal(item.admission.accepted, true);
  assert.equal(item.reservation.open, false);
  assert.equal(item.quote.pool_penalty, 0);
  assert.ok(item.quote.operator_rebate > 0);
  assert.equal(
    item.reservation.paid + item.reservation.penalty + item.reservation.released,
    item.reservation.amount,
  );

  const solver = operator(report, "ember-solver");
  assert.equal(solver.routes_executed, 1);
  assert.equal(solver.rebates_earned, item.quote.operator_rebate);
  assert.equal(report.accounting.account_balances["operator:ember-solver"], solver.rebates_earned);
  assertPoolConservation(report);
});
