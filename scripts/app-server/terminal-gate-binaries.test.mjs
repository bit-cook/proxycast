import path from "node:path";
import { describe, expect, it } from "vitest";

import {
  terminalGateBuildPackages,
  terminalGateCargoBuildArgs,
} from "./terminal-gate-binaries.mjs";

describe("terminal Gate B binaries", () => {
  it("builds both current binaries for default paths", () => {
    expect(terminalGateBuildPackages({})).toEqual(["cli", "app-server"]);
    expect(
      terminalGateCargoBuildArgs({ env: {}, repoRoot: "/workspace/lime" }),
    ).toEqual([
      "build",
      "--manifest-path",
      path.resolve("/workspace/lime/lime-rs/Cargo.toml"),
      "-p",
      "cli",
      "-p",
      "app-server",
    ]);
  });

  it("only builds owners whose binary path was not supplied", () => {
    expect(terminalGateBuildPackages({ LIME_CLI_BIN: "/tmp/lime" })).toEqual([
      "app-server",
    ]);
    expect(
      terminalGateBuildPackages({ APP_SERVER_BIN: "/tmp/app-server" }),
    ).toEqual(["cli"]);
  });

  it("does not rebuild explicitly supplied artifacts", () => {
    const env = {
      LIME_CLI_BIN: "/tmp/lime",
      APP_SERVER_BIN: "/tmp/app-server",
    };
    expect(terminalGateBuildPackages(env)).toEqual([]);
    expect(terminalGateCargoBuildArgs({ env })).toEqual([]);
  });
});
