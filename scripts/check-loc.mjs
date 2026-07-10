import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const src = join(root, "src");
const min = 5_000;
const max = 6_000;

function files(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return files(path);
    if (entry.isFile() && path.endsWith(".rs")) return [path];
    return [];
  });
}

let lines = 0;
for (const path of files(src)) {
  if (statSync(path).size === 0) continue;
  lines += readFileSync(path, "utf8").split(/\r?\n/u).filter(Boolean).length;
}

if (lines < min || lines > max) {
  console.error(`src LOC fuera de rango: ${lines} (esperado ${min}-${max})`);
  process.exit(1);
}

console.log(`src LOC: ${lines}`);
