import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { EmberlineClient, EmberlineClientError } from "../../sdk/client.js";
import { root } from "../helpers/emberline.js";

const binary = join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "emberline_dtl.exe" : "emberline_dtl",
);

function client() {
  if (!existsSync(binary)) {
    execFileSync(process.platform === "win32" ? "cargo.exe" : "cargo", ["build", "--locked"], {
      cwd: root,
      env: { ...process.env, CARGO_TARGET_DIR: join(root, "target") },
      stdio: "pipe",
      timeout: 120_000,
    });
  }
  return new EmberlineClient({ binaryPath: binary, cwd: root });
}

test("el cliente lista escenarios mediante ejecucion sin shell", () => {
  assert.deepEqual(client().listScenarios(), ["normal", "rebate", "penalty", "pool", "crossflow"]);
});

test("el cliente devuelve un reporte tipado", () => {
  const report = client().runScenario("normal");
  assert.equal(report.protocol, "EmberlineDTL");
  assert.equal(report.scenario, "normal");
  assert.equal(report.invariants.reports_deterministic, true);
});

test("el cliente valida el contrato de escenario", () => {
  assert.equal(client().validateScenario("pool"), "ok pool");
});

test("el cliente rechaza nombres fuera del contrato", () => {
  assert.throws(
    () => client().runScenario("../normal"),
    (error) => error instanceof EmberlineClientError && error.code === "INVALID_INPUT",
  );
});

test("el cliente rechaza un binario ausente", () => {
  const missing = new EmberlineClient({ binaryPath: "target/debug/missing", cwd: root });
  assert.throws(
    () => missing.runScenario("normal"),
    (error) => error instanceof EmberlineClientError && error.code === "INVALID_INPUT",
  );
});

test("el cliente valida limites de proceso", () => {
  assert.throws(
    () => new EmberlineClient({ binaryPath: binary, cwd: root, timeoutMs: 1 }),
    (error) => error instanceof EmberlineClientError && error.code === "INVALID_INPUT",
  );
});
