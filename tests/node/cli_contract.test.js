import assert from "node:assert/strict";
import test from "node:test";

import {
  assertCommon,
  assertHex32,
  listScenarios,
  scenario,
  validate,
} from "../helpers/emberline.js";

test("la CLI lista y valida los escenarios publicados", () => {
  const names = listScenarios();
  assert.deepEqual(names, ["normal", "rebate", "penalty", "pool", "crossflow"]);
  for (const name of names) {
    assert.equal(validate(name), `ok ${name}`);
  }
});

test("cada reporte expone el contrato JSON estable", () => {
  for (const name of listScenarios()) {
    const report = scenario(name);
    assertCommon(report, name);
    assert.ok(Array.isArray(report.events));
    assert.ok(Array.isArray(report.routes));
    assert.ok(Object.hasOwn(report.pool, "available"));
    assert.ok(Object.hasOwn(report.accounting.account_balances, "rebate-pool"));
  }
});

test("los identificadores principales se serializan como hex de 32 bytes", () => {
  const report = scenario("normal");
  const firstRoute = report.routes[0];
  assertHex32(firstRoute.plan.id);
  assertHex32(firstRoute.plan.operator);
  assertHex32(firstRoute.plan.asset);
  assertHex32(firstRoute.reservation.id);
  assertHex32(firstRoute.receipt.id);
});
