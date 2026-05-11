import fs from "node:fs";
import path from "node:path";

const [, , targetName, binaryPath] = process.argv;

if (!targetName || !binaryPath) {
  console.error("Usage: node scripts/package-platform.mjs <darwin-arm64|darwin-x64|win32-x64> <binary>");
  process.exit(1);
}

const meta = {
  "darwin-arm64": { os: ["darwin"], cpu: ["arm64"], ext: "" },
  "darwin-x64": { os: ["darwin"], cpu: ["x64"], ext: "" },
  "win32-x64": { os: ["win32"], cpu: ["x64"], ext: ".exe" }
}[targetName];

if (!meta) {
  console.error(`Unsupported target package: ${targetName}`);
  process.exit(1);
}

const outDir = path.resolve("npm", "platform", targetName);
const binDir = path.join(outDir, "bin");
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(binDir, { recursive: true });
fs.copyFileSync(binaryPath, path.join(binDir, `agent-browser-cli${meta.ext}`));

fs.writeFileSync(
  path.join(outDir, "package.json"),
  JSON.stringify(
    {
      name: `@sleepinsummer/agent-browser-cli-${targetName}`,
      version: "0.2.1",
      license: "MIT",
      os: meta.os,
      cpu: meta.cpu,
      files: ["bin"]
    },
    null,
    2
  ) + "\n",
  "utf8"
);
