#!/usr/bin/env node
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const lock = JSON.parse(readFileSync(new URL("../package-lock.json", import.meta.url), "utf8"));
const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const cargo = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoLock = readFileSync(new URL("../src-tauri/Cargo.lock", import.meta.url), "utf8");
const cargoVersion = cargo.match(/^version = "([^"]+)"/m)?.[1];
const lockedVersion = cargoLock.match(/\[\[package\]\]\s+name = "task-timer"\s+version = "([^"]+)"/m)?.[1];
const versions = { package: pkg.version, npmLock: lock.version, npmRoot: lock.packages[""].version, tauri: conf.version, cargo: cargoVersion, cargoLock: lockedVersion };
if (Object.values(versions).some((version) => version !== pkg.version)) {
  console.error("Version mismatch:", versions);
  process.exit(1);
}
const rustPackageVersion = (name) => {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return cargoLock.match(new RegExp(`\\[\\[package\\]\\]\\s+name = "${escaped}"\\s+version = "([^"]+)"`, "m"))?.[1];
};
const tauriPairs = [
  ["@tauri-apps/api", "tauri"],
  ["@tauri-apps/cli", "tauri"],
  ["@tauri-apps/plugin-process", "tauri-plugin-process"],
  ["@tauri-apps/plugin-updater", "tauri-plugin-updater"],
];
for (const [js, rust] of tauriPairs) {
  const jsVersion = lock.packages[`node_modules/${js}`]?.version;
  const rustVersion = rustPackageVersion(rust);
  if (!jsVersion || !rustVersion || jsVersion.split(".").slice(0, 2).join(".") !== rustVersion.split(".").slice(0, 2).join(".")) {
    console.error(`Tauri minor mismatch: ${js} ${jsVersion} vs ${rust} ${rustVersion}`);
    process.exit(1);
  }
}
const tag = process.env.GITHUB_REF?.startsWith("refs/tags/v") ? process.env.GITHUB_REF.slice("refs/tags/v".length) : null;
if (tag && tag !== pkg.version) {
  console.error(`Release tag v${tag} does not match app version ${pkg.version}`);
  process.exit(1);
}
console.log(`App and Tauri dependency versions match: ${pkg.version}`);
