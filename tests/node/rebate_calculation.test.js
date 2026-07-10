import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCommon,
  assertPoolConservation,
  operator,
  route,
  scenario,
} from "../helpers/emberline.js";

test("el scoring premia la ruta mas eficiente y rapida", () => {
  const report = scenario("rebate");
  assertCommon(report, "rebate");

  const preferred = route(report, "preferred-fast-lane");
  const standard = route(report, "standard-lane");

  assert.ok(preferred.quote.adjusted_cost < preferred.quote.observed_cost);
  assert.ok(standard.quote.adjusted_cost < standard.quote.observed_cost);
  assert.ok(preferred.quote.combined_score_bps > standard.quote.combined_score_bps);
  assert.ok(preferred.quote.operator_rebate > standard.quote.operator_rebate);
  assert.equal(preferred.quote.pool_penalty, 0);
  assert.equal(standard.quote.pool_penalty, 0);

  const primeOperator = operator(report, "north-bridge");
  const standardOperator = operator(report, "delta-runner");
  assert.equal(primeOperator.rebates_earned, preferred.quote.operator_rebate);
  assert.equal(standardOperator.rebates_earned, standard.quote.operator_rebate);
  assertPoolConservation(report);
});
