#!/usr/bin/env node

import { access, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { localAppServerBinaryPath } from "../lib/electron-dev-sidecar.mjs";
import { buildTerminalGateBinaries } from "./terminal-gate-binaries.mjs";

const execFileAsync = promisify(execFile);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "../..");
const clientDistPath = path.join(
  rootDir,
  "packages",
  "app-server-client",
  "dist",
  "index.js",
);
const cliBinaryName = process.platform === "win32" ? "lime.exe" : "lime";
const defaultCliBinaryPath = path.join(
  rootDir,
  "lime-rs",
  "target",
  "debug",
  cliBinaryName,
);

const {
  PROTOCOL_VERSION,
  connectAppServerSidecar,
  resolveSidecarBinaryPath,
  sidecarArgs,
  stdioSidecar,
} = await import(pathToFileURL(clientDistPath).href);

async function main() {
  await buildTerminalGateBinaries({ env: process.env, repoRoot: rootDir });
  const cliBinaryPath = path.resolve(
    process.env.LIME_CLI_BIN || defaultCliBinaryPath,
  );
  const appServerBinaryPath = path.resolve(
    process.env.APP_SERVER_BIN ||
      localAppServerBinaryPath({ repoRoot: rootDir }),
  );
  await Promise.all([
    assertExists(cliBinaryPath, "lime"),
    assertExists(appServerBinaryPath, "app-server"),
  ]);
  const runtimeEnv = await resolveRuntimeEnvironment();

  const tempDir = await mkdtemp(path.join(tmpdir(), "tui-history-pagination-"));
  const dataDir = path.join(tempDir, "data");
  const appDataDir = path.join(tempDir, "app-data");
  const backendPath = path.join(tempDir, "history-backend.mjs");
  const ledgerPath = path.join(tempDir, "history-backend.jsonl");
  try {
    await writeFile(backendPath, backendSource());
    const config = {
      ...stdioSidecar(appServerBinaryPath, undefined, dataDir),
      backendMode: "external",
      backendCommand: process.execPath,
      backendArgs: [backendPath, ledgerPath],
      backendTimeoutMs: 5_000,
    };
    const connected = await connectAppServerSidecar(
      config,
      {
        clientInfo: { name: "tui-history-pagination-fixture", version: "1" },
      },
      {
        initializeTimeoutMs: 10_000,
        expectedProtocolVersion: PROTOCOL_VERSION,
        args: [...sidecarArgs(config), "--app-data-dir", appDataDir],
        env: runtimeEnv,
      },
    );

    const connection = connected.connection;
    const started = await connection.startSession({
      cwd: tempDir,
      runtimeWorkspaceRoots: [tempDir],
      model: "fixture-model",
      modelProvider: "fixture-provider",
      historyMode: "paginated",
    });
    const threadId = started.result.thread.id;

    // Each completed turn contributes a user and assistant item. Keep the seed deterministic and
    // above the TUI's 100-item page size so the resume path must fetch an older page.
    for (let index = 0; index < 51; index += 1) {
      const result = await connection.startTurn({
        threadId,
        input: [{ type: "text", text: `SEED_${String(index).padStart(3, "0")}` }],
      });
      await drainTurnCompletion(connection, result, result.result.turn.id);
    }

    const review = await connection.startReview({
      threadId,
      delivery: "inline",
      target: { type: "commit", sha: "fixture-sha", title: "fixture review" },
    });
    await drainTurnCompletion(connection, review, review.result.turn.id);

    const blocked = connection.startTurn({
      threadId,
      input: [{ type: "text", text: "NESTED_REVIEW_PROMPT" }],
    });
    const blockedTurnId = await waitForInProgressTurn(connection, threadId);
    await connection.steerTurn({
      threadId,
      expectedTurnId: blockedTurnId,
      clientUserMessageId: "nested-review-duplicate",
      input: [{ type: "text", text: "NESTED_REVIEW_PROMPT" }],
    });
    // Fork while the tail turn is still in progress. The runtime snapshots that turn as
    // Interrupted with no completion timestamp, matching Codex's nested-review replay shape
    // while preserving the canonical duplicate UserMessage items.
    const forked = await connection.forkThread({
      threadId,
      excludeTurns: true,
    });
    const resumeThreadId = forked.result.thread.id;

    // Stop the seed sidecar before launching the independent TUI sidecar.
    connected.sidecar.child.kill("SIGKILL");
    await connected.sidecar.waitForExit(5_000);
    await connected.sidecar.close().catch(() => undefined);
    await blocked.catch(() => undefined);

    const metadataPath = path.join(tempDir, "thread.json");
    await writeFile(
      metadataPath,
      JSON.stringify({
        threadId: resumeThreadId,
        tempDir,
        dataDir,
        appDataDir,
        backendPath,
        ledgerPath,
      }),
    );
    await execFileAsync(
      process.env.CARGO || "cargo",
      [
        "test",
        "--locked",
        "--manifest-path",
        path.join(rootDir, "lime-rs", "Cargo.toml"),
        "-p",
        "tui",
        "--test",
        "all",
        "suite::history_pagination::older_pagination_reconciles_review_prompts_across_page_boundaries",
        "--",
        "--exact",
        "--nocapture",
      ],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          ...runtimeEnv,
          LIME_TEST_TUI_HISTORY_PAGINATION: "1",
          LIME_TEST_CLI_BIN: cliBinaryPath,
          LIME_TEST_APP_SERVER_BIN: appServerBinaryPath,
          LIME_TEST_TERMINAL_BACKEND: backendPath,
          LIME_TEST_TERMINAL_LEDGER: ledgerPath,
          LIME_TEST_TERMINAL_CWD: tempDir,
          LIME_TEST_NODE_BIN: process.execPath,
          LIME_TEST_TUI_HISTORY_THREAD_ID: resumeThreadId,
          LIME_TEST_TUI_GATE_B: "1",
        },
        maxBuffer: 4 * 1024 * 1024,
        timeout: 180_000,
        windowsHide: true,
      },
    );
      console.log(
      `[smoke:tui-history-pagination] ok thread=${resumeThreadId} items>100 review=nested-filtered alternate-screen=restored`,
    );
  } finally {
    if (process.env.LIME_KEEP_TUI_HISTORY_PAGINATION_TMP !== "1") {
      await rm(tempDir, { recursive: true, force: true });
    } else {
      console.error(`[smoke:tui-history-pagination] kept temp dir ${tempDir}`);
    }
  }
}

async function resolveRuntimeEnvironment() {
  if (process.platform !== "darwin") {
    return {};
  }
  const libraryDirs = [path.join(rootDir, "lime-rs", "target", "debug")];
  const prebuiltRoot = path.join(
    rootDir,
    "lime-rs",
    "target",
    "sherpa-onnx-prebuilt",
  );
  const entries = await readdir(prebuiltRoot, { withFileTypes: true }).catch(
    () => [],
  );
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const libDir = path.join(prebuiltRoot, entry.name, "lib");
    try {
      await access(libDir);
      libraryDirs.push(libDir);
    } catch {
      // Optional prebuilt variants are absent on most developer machines.
    }
  }
  if (process.env.DYLD_LIBRARY_PATH) {
    libraryDirs.push(process.env.DYLD_LIBRARY_PATH);
  }
  return { DYLD_LIBRARY_PATH: libraryDirs.join(path.delimiter) };
}

async function drainTurnCompletion(connection, initial, turnId) {
  const notifications = initial.notifications || [];
  if (notifications.some((entry) => entry.method === "turn/completed" && entry.params?.turn?.id === turnId)) {
    return;
  }
  while (true) {
    const notification = await connection.nextNotification(30_000);
    if (notification.method === "turn/completed" && notification.params?.turn?.id === turnId) {
      return;
    }
  }
}

async function waitForInProgressTurn(connection, threadId) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const read = await connection.readThread({ threadId, includeTurns: true });
    const turn = [...(read.result.thread.turns || [])]
      .reverse()
      .find((candidate) => candidate.status === "inProgress");
    if (turn?.id) return turn.id;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error("blocked turn did not become inProgress");
}

async function assertExists(target, label) {
  try {
    await access(target);
  } catch {
    throw new Error(`${label} binary not found: ${target}`);
  }
}

function backendSource() {
  return `#!/usr/bin/env node
import { appendFileSync, readFileSync } from "node:fs";
const input = JSON.parse(readFileSync(0, "utf8"));
const ledger = process.argv[2];
const request = input.request || {};
const text = JSON.stringify(request.input || []);
const turnId = request.turn?.turnId || null;
const threadId = request.session?.threadId || null;
const blocked = input.kind === "turnStart" && text.includes("NESTED_REVIEW_PROMPT");
let events = [{ type: "turn.started", payload: {} }];
if (input.kind === "turnStart" && !blocked) {
  const assistantText = text.includes("SEED_") ? text.match(/SEED_\\d{3}/)?.[0] || "seed" : "review complete";
  events.push({ type: "message.delta", payload: { itemId: "assistant-" + turnId, text: assistantText } });
  events.push({ type: "message.completed", payload: { itemId: "assistant-" + turnId, status: "completed", text: assistantText } });
  events.push({ type: "turn.completed", payload: { status: "completed" } });
} else if (input.kind === "turnCancel") {
  events = [{ type: "turn.canceled", payload: { status: "canceled" } }];
}
appendFileSync(ledger, JSON.stringify({ kind: input.kind, threadId, turnId, blocked, eventTypes: events.map((event) => event.type) }) + "\\n");
console.log(JSON.stringify({ events }));
if (blocked) { setInterval(() => {}, 1000); await new Promise(() => {}); }
`;
}

main().catch((error) => {
  console.error(`[smoke:tui-history-pagination] failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
