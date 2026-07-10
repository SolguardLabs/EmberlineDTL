import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCommon,
  assertPoolConservation,
  operator,
  route,
  scenario,
} from "../helpers/emberline.js";

test("una liquidacion fuera de ventana registra penalizacion del pool", () => {
  const report = scenario("penalty");
  assertCommon(report, "penalty");

  const delayed = route(report, "delayed-cost-window");
  assert.equal(delayed.executed, true);
  assert.ok(delayed.receipt.elapsed_ms > 18_000);
  assert.ok(delayed.quote.pool_penalty > 0);
  assert.ok(delayed.quote.combined_score_bps < 3_000);
  assert.equal(delayed.reservation.penalty, delayed.quote.pool_penalty);
  assert.equal(report.pool.penalties, delayed.quote.pool_penalty);

  const runner = operator(report, "delta-runner");
  assert.equal(runner.penalties_paid, delayed.quote.pool_penalty);
  assertPoolConservation(report);
});
