#!/usr/bin/env node
// Bumps all app manifests and locks in lockstep, then tags locally.
// Usage: node scripts/release.mjs <patch|minor|major|x.y.z>
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const bump = process.argv[2];
if (!bump) {
  console.error("Usage: node scripts/release.mjs <patch|minor|major|x.y.z>");
  process.exit(1);
}

const pkgPath = new URL("../package.json", import.meta.url);
const confPath = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const npmLockPath = new URL("../package-lock.json", import.meta.url);
const cargoPath = new URL("../src-tauri/Cargo.toml", import.meta.url);
const cargoLockPath = new URL("../src-tauri/Cargo.lock", import.meta.url);

if (execSync("git status --porcelain", { encoding: "utf8" }).trim()) {
  console.error("Commit or stash working tree changes before creating a release.");
  process.exit(1);
}

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const [major, minor, patch] = pkg.version.split(".").map(Number);

let nextVersion;
if (/^\d+\.\d+\.\d+$/.test(bump)) {
  nextVersion = bump;
} else if (bump === "patch") {
  nextVersion = `${major}.${minor}.${patch + 1}`;
} else if (bump === "minor") {
  nextVersion = `${major}.${minor + 1}.0`;
} else if (bump === "major") {
  nextVersion = `${major + 1}.0.0`;
} else {
  console.error(`Unknown bump type: ${bump}`);
  process.exit(1);
}

pkg.version = nextVersion;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.version = nextVersion;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

const npmLock = JSON.parse(readFileSync(npmLockPath, "utf8"));
npmLock.version = nextVersion;
npmLock.packages[""].version = nextVersion;
writeFileSync(npmLockPath, JSON.stringify(npmLock, null, 2) + "\n");

const cargo = readFileSync(cargoPath, "utf8").replace(/^version = "[^"]+"/m, `version = "${nextVersion}"`);
writeFileSync(cargoPath, cargo);
const cargoLock = readFileSync(cargoLockPath, "utf8").replace(/(\[\[package\]\]\s+name = "task-timer"\s+version = ")[^"]+"/m, (_, prefix) => `${prefix}${nextVersion}"`);
writeFileSync(cargoLockPath, cargoLock);

execSync("node scripts/verify-versions.mjs", { stdio: "inherit" });

execSync("git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json", { stdio: "inherit" });
execSync(`git commit -m "chore: release v${nextVersion}"`, { stdio: "inherit" });
execSync(`git tag v${nextVersion}`, { stdio: "inherit" });

console.log(`\nTagged v${nextVersion}. Run: git push && git push origin v${nextVersion}`);
