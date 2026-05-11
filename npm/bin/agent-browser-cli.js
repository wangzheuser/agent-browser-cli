#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function platformPackageName() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === "darwin" && arch === "arm64") return "@sleepinsummer/agent-browser-cli-darwin-arm64";
  if (platform === "darwin" && arch === "x64") return "@sleepinsummer/agent-browser-cli-darwin-x64";
  if (platform === "win32" && arch === "x64") return "@sleepinsummer/agent-browser-cli-win32-x64";
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function resolveBinary() {
  if (process.env.AGENT_BROWSER_CLI_BIN) return process.env.AGENT_BROWSER_CLI_BIN;
  try {
    const pkg = platformPackageName();
    return require.resolve(`${pkg}/bin/agent-browser-cli${process.platform === "win32" ? ".exe" : ""}`);
  } catch (_) {
    const ext = process.platform === "win32" ? ".exe" : "";
    const local = path.resolve(__dirname, "..", "..", "target", "release", `agent-browser-cli${ext}`);
    if (fs.existsSync(local)) return local;
    throw new Error("agent-browser-cli native binary not found. Run `npm run build` for local development.");
  }
}

const bin = resolveBinary();
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 0);
