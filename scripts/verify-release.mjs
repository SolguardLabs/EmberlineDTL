import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const expectedDocs = [
  "01-arquitectura.md",
  "02-routing-y-admision.md",
  "03-economia-de-rebates.md",
  "04-tesoreria-y-stress.md",
  "05-operacion.md",
  "06-integracion-y-observabilidad.md",
  "07-despliegue.md",
];

function fail(message) {
  console.error(`release verification failed: ${message}`);
  process.exitCode = 1;
}

function walk(directory, extensions) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return walk(path, extensions);
    return entry.isFile() && extensions.has(extname(entry.name)) ? [path] : [];
  });
}

function count(pattern, paths) {
  return paths.reduce((total, path) => {
    const matches = readFileSync(path, "utf8").match(pattern);
    return total + (matches?.length ?? 0);
  }, 0);
}

const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
if (packageJson.version !== "1.0.0") fail("package.json must declare 1.0.0");

const cargoToml = readFileSync(join(root, "Cargo.toml"), "utf8");
if (!/^version\s*=\s*"1\.0\.0"$/mu.test(cargoToml)) {
  fail("Cargo.toml must declare 1.0.0");
}

const docsDirectory = join(root, "docs");
const actualDocs = readdirSync(docsDirectory)
  .filter((name) => name.endsWith(".md"))
  .sort();
if (JSON.stringify(actualDocs) !== JSON.stringify(expectedDocs)) {
  fail(`docs manifest differs: ${actualDocs.join(", ")}`);
}

const markdown = [
  join(root, "README.md"),
  join(root, "SECURITY.md"),
  ...actualDocs.map((name) => join(docsDirectory, name)),
];
const diagrams = count(/```mermaid/gu, markdown);
if (diagrams !== 27) fail(`expected 27 Mermaid diagrams, found ${diagrams}`);

const publicFiles = [
  ...markdown,
  ...walk(join(root, "src"), new Set([".rs"])),
  ...walk(join(root, "sdk"), new Set([".js"])),
  ...walk(join(root, "tests"), new Set([".js"])),
];
const restricted = /\b(?:ctf|laboratorio|vulnerabilidad|vulnerable|exploit|bypass|atacante)\b/iu;
for (const path of publicFiles) {
  if (restricted.test(readFileSync(path, "utf8"))) {
    fail(`restricted public terminology in ${relative(root, path)}`);
  }
}

const banner = readFileSync(join(root, "assets", "banner.png"));
if (banner.length < 300_000 || banner.toString("ascii", 1, 4) !== "PNG") {
  fail("banner must be a production PNG of at least 300 KB");
} else {
  const width = banner.readUInt32BE(16);
  const height = banner.readUInt32BE(20);
  if (width < 1_600 || height < 900) {
    fail(`banner dimensions too small: ${width}x${height}`);
  }
}

const rustTests = count(/#\s*\[\s*test\s*\]/gu, walk(join(root, "src"), new Set([".rs"])));
const nodeTests = count(/\btest\s*\(/gu, walk(join(root, "tests", "node"), new Set([".js"])));
if (rustTests < 11) fail(`expected at least 11 Rust tests, found ${rustTests}`);
if (nodeTests < 13) fail(`expected at least 13 Node tests, found ${nodeTests}`);

const protectedFiles = JSON.parse(
  readFileSync(join(root, "scripts", "protected-files.json"), "utf8"),
);
for (const [path, expected] of Object.entries(protectedFiles)) {
  const actual = execFileSync("git", ["hash-object", path], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  if (actual !== expected) fail(`compatibility hash differs for ${path}`);
}

if (!process.exitCode) {
  console.log(
    `release verified: ${actualDocs.length} docs, ${diagrams} diagrams, ${rustTests} Rust tests, ${nodeTests} Node tests`,
  );
}
