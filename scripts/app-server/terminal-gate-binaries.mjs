import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

import { resolveRustyV8CargoEnv } from "../lib/rusty-v8-artifacts.mjs";

export function terminalGateBuildPackages(env = process.env) {
  const packages = [];
  if (!env.LIME_CLI_BIN?.trim()) {
    packages.push("cli");
  }
  if (!env.APP_SERVER_BIN?.trim()) {
    packages.push("app-server");
  }
  return packages;
}

export function terminalGateCargoBuildArgs({
  env = process.env,
  repoRoot = process.cwd(),
} = {}) {
  const packages = terminalGateBuildPackages(env);
  if (packages.length === 0) {
    return [];
  }
  return [
    "build",
    "--manifest-path",
    path.resolve(repoRoot, "lime-rs", "Cargo.toml"),
    ...packages.flatMap((packageName) => ["-p", packageName]),
  ];
}

export async function buildTerminalGateBinaries({
  env = process.env,
  repoRoot = process.cwd(),
  runner = spawn,
  resolveV8Env = resolveRustyV8CargoEnv,
} = {}) {
  const args = terminalGateCargoBuildArgs({ env, repoRoot });
  if (args.length === 0) {
    return;
  }
  const cargoCommand = process.platform === "win32" ? "cargo.exe" : "cargo";
  await new Promise((resolve, reject) => {
    const child = runner(cargoCommand, args, {
      cwd: repoRoot,
      env: {
        ...env,
        ...resolveV8Env({ env, repoRoot }),
      },
      shell: false,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      const detail = signal ? `signal=${signal}` : `code=${code ?? "unknown"}`;
      reject(
        new Error(`cargo build terminal Gate B binaries failed (${detail})`),
      );
    });
  });
}
