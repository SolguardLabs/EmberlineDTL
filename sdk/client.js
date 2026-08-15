import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

export class EmberlineClientError extends Error {
  constructor(code, message, details = {}) {
    super(message, { cause: details.cause });
    this.name = "EmberlineClientError";
    this.code = code;
    this.exitCode = details.exitCode;
    this.stderr = details.stderr;
  }
}

export class EmberlineClient {
  constructor(options) {
    if (!options?.binaryPath?.trim()) {
      throw new EmberlineClientError("INVALID_INPUT", "binaryPath is required");
    }
    this.cwd = resolve(options.cwd ?? process.cwd());
    this.binaryPath = isAbsolute(options.binaryPath)
      ? options.binaryPath
      : resolve(this.cwd, options.binaryPath);
    this.timeoutMs = options.timeoutMs ?? 10_000;
    this.maxOutputBytes = options.maxOutputBytes ?? 4 * 1024 * 1024;
    if (!Number.isSafeInteger(this.timeoutMs) || this.timeoutMs < 100 || this.timeoutMs > 120_000) {
      throw new EmberlineClientError("INVALID_INPUT", "timeoutMs must be between 100 and 120000");
    }
    if (!Number.isSafeInteger(this.maxOutputBytes) || this.maxOutputBytes < 1_024) {
      throw new EmberlineClientError("INVALID_INPUT", "maxOutputBytes must be at least 1024");
    }
  }

  listScenarios() {
    return this.execute(["--list"]).trim().split(/\r?\n/u).filter(Boolean);
  }

  runScenario(name) {
    const output = this.execute(["scenario", this.scenarioName(name)]);
    try {
      const report = JSON.parse(output);
      assertReport(report);
      return report;
    } catch (error) {
      if (error instanceof EmberlineClientError) throw error;
      throw new EmberlineClientError("INVALID_RESPONSE", "invalid EmberlineDTL report", {
        cause: error,
      });
    }
  }

  validateScenario(name) {
    const scenario = this.scenarioName(name);
    const result = this.execute(["validate", scenario]).trim();
    if (result !== `ok ${scenario}`) {
      throw new EmberlineClientError("INVALID_RESPONSE", "unexpected validation response");
    }
    return result;
  }

  scenarioName(value) {
    if (typeof value !== "string" || !/^[a-z0-9-]{1,64}$/u.test(value)) {
      throw new EmberlineClientError("INVALID_INPUT", "invalid scenario name");
    }
    return value;
  }

  execute(args) {
    if (!existsSync(this.binaryPath) || !statSync(this.binaryPath).isFile()) {
      throw new EmberlineClientError("INVALID_INPUT", `binary does not exist: ${this.binaryPath}`);
    }
    const result = spawnSync(this.binaryPath, args, {
      cwd: this.cwd,
      encoding: "utf8",
      shell: false,
      timeout: this.timeoutMs,
      maxBuffer: this.maxOutputBytes,
      windowsHide: true,
    });
    if (result.error) {
      const timeout = result.error.code === "ETIMEDOUT";
      throw new EmberlineClientError(timeout ? "TIMEOUT" : "PROCESS_FAILED", result.error.message, {
        exitCode: result.status ?? undefined,
        stderr: result.stderr,
        cause: result.error,
      });
    }
    if (result.status !== 0) {
      throw new EmberlineClientError("PROCESS_FAILED", "EmberlineDTL command failed", {
        exitCode: result.status ?? undefined,
        stderr: result.stderr.trim(),
      });
    }
    return result.stdout;
  }
}

function assertReport(report) {
  if (
    typeof report !== "object" ||
    report === null ||
    report.protocol !== "EmberlineDTL" ||
    typeof report.scenario !== "string" ||
    !Array.isArray(report.routes) ||
    !Array.isArray(report.operators) ||
    typeof report.pool !== "object" ||
    report.pool === null ||
    typeof report.invariants !== "object" ||
    report.invariants === null
  ) {
    throw new EmberlineClientError("INVALID_RESPONSE", "report schema mismatch");
  }
}
