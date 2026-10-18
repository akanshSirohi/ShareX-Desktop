import { cp, mkdir, readFile, rename, rm } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "portal-source", "out");
const destination = path.join(root, "src-tauri", "portal");
await readFile(path.join(source, "index.html"));
const staging = path.join(root, "src-tauri", `portal-staging-${randomUUID()}`);
const backup = path.join(root, "src-tauri", `portal-backup-${randomUUID()}`);
const insideWorkspace = (p) =>
  path.resolve(p).startsWith(root + path.sep) &&
  path.dirname(path.resolve(p)) === path.join(root, "src-tauri");
if (![destination, staging, backup].every(insideWorkspace))
  throw new Error("Unsafe portal output path");
await mkdir(staging, { recursive: true });
await cp(source, staging, { recursive: true });
await cp(
  path.join(root, "src-tauri", "icons", "icon.ico"),
  path.join(staging, "favicon.ico"),
);
await readFile(path.join(staging, "index.html"));
await rename(destination, backup);
try {
  await rename(staging, destination);
} catch (error) {
  await rename(backup, destination);
  throw error;
}
await rm(backup, { recursive: true, force: true });
console.log(`Portal export synchronized to ${destination}.`);
