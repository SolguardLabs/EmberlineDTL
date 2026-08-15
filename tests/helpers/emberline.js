import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const cache = new Map();

export function runCli(args) {
  return execFileSync(cargo, ["run", "--quiet", "--", ...args], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, CARGO_TERM_COLOR: "never" },
    timeout: 120_000,
  });
}

export function listScenarios() {
  return runCli(["--list"]).trim().split(/\r?\n/u).filter(Boolean);
}

export function scenario(name) {
  if (!cache.has(name)) {
    cache.set(name, JSON.parse(runCli(["scenario", name])));
  }
  return cache.get(name);
}

export function validate(name) {
  return runCli(["validate", name]).trim();
}

export function route(report, label) {
  const found = report.routes.find((entry) => entry.plan.label === label);
  assert.ok(found, `missing route ${label}`);
  return found;
}

export function operator(report, label) {
  const found = report.operators.find((entry) => entry.label === label);
  assert.ok(found, `missing operator ${label}`);
  return found;
}

export function assertHex32(value) {
  assert.equal(typeof value, "string");
  assert.match(value, /^[0-9a-f]{64}$/u);
}

export function assertCommon(report, name) {
  assert.equal(report.protocol, "EmberlineDTL");
  assert.equal(report.scenario, name);
  assert.equal(report.network_id, 41720);
  assertHex32(report.state_digest);
  assertHex32(report.reference_digest);
  assert.equal(report.reference_controls, 430);
  assert.ok(report.assets.length >= 4);
  assert.ok(report.operators.length >= 4);
  assert.equal(report.invariants.pool_non_negative, true);
  assert.equal(report.invariants.reservations_conserved, true);
  assert.equal(report.invariants.operators_non_negative, true);
  assert.equal(report.invariants.route_links_valid, true);
  assert.equal(report.invariants.reports_deterministic, true);
}

export function assertPoolConservation(report) {
  const initialPool = 8_000_000;
  assert.equal(
    report.pool.available + report.pool.reserved + report.pool.paid + report.pool.penalties,
    initialPool,
  );
  assert.equal(report.accounting.pool_available, report.pool.available);
  assert.equal(report.accounting.pool_reserved, report.pool.reserved);
  assert.equal(report.accounting.pool_paid, report.pool.paid);
  assert.equal(report.accounting.pool_penalties, report.pool.penalties);
}

export function sumRouteQuotes(report, field) {
  return report.routes.reduce((total, item) => total + (item.quote?.[field] ?? 0), 0);
}
