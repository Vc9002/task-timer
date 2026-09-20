#!/usr/bin/env node
// Bumps package.json + src-tauri/tauri.conf.json in lockstep, tags, and pushes.
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

execSync(`git add package.json src-tauri/tauri.conf.json`, { stdio: "inherit" });
execSync(`git commit -m "chore: release v${nextVersion}"`, { stdio: "inherit" });
execSync(`git tag v${nextVersion}`, { stdio: "inherit" });

console.log(`\nTagged v${nextVersion}. Run: git push && git push origin v${nextVersion}`);
