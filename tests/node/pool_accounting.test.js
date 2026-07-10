import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCommon,
  assertPoolConservation,
  scenario,
  sumRouteQuotes,
} from "../helpers/emberline.js";

test("el accounting del pool agrega multiples rutas cerradas", () => {
  const report = scenario("pool");
  assertCommon(report, "pool");
  assert.equal(report.routes.length, 3);
  assert.equal(report.pool.reserved, 0);
  assert.equal(report.pool.reservation_count, 3);
  assert.equal(report.pool.paid, sumRouteQuotes(report, "operator_rebate"));
  assert.equal(report.pool.penalties, sumRouteQuotes(report, "pool_penalty"));
  assert.equal(
    report.pool.released,
    report.routes.reduce((total, item) => total + item.reservation.released, 0),
  );
  for (const item of report.routes) {
    assert.equal(item.executed, true);
    assert.equal(item.reservation.open, false);
  }
  assertPoolConservation(report);
});
