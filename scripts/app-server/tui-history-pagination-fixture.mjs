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
  let connected;
  try {
    await writeFile(backendPath, backendSource());
    const config = {
      ...stdioSidecar(appServerBinaryPath, undefined, dataDir),
      backendMode: "external",
      backendCommand: process.execPath,
      backendArgs: [backendPath, ledgerPath],
      backendTimeoutMs: 5_000,
    };
    connected = await connectAppServerSidecar(
      config,
      {
        clientInfo: { name: "tui-history-pagination-fixture", version: "1" },
        capabilities: { experimentalApi: true },
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
    assertHistoryMode(started, "paginated");
    const threadId = started.result.thread.id;

    // Each completed turn contributes a user and assistant item. The paginated thread spans at
    // least three 100-item pages, matching Codex's complete transcript contract; legacy still
    // crosses its single-page boundary without paying the extra setup cost.
    await seedCompletedTurns(connection, threadId, "SEED", 101);

    const legacyStarted = await connection.startSession({
      cwd: tempDir,
      runtimeWorkspaceRoots: [tempDir],
      model: "fixture-model",
      modelProvider: "fixture-provider",
      historyMode: "legacy",
    });
    assertHistoryMode(legacyStarted, "legacy");
    const legacyThreadId = legacyStarted.result.thread.id;
    await seedCompletedTurns(connection, legacyThreadId, "LEGACY", 51);

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
    assertHistoryMode(forked, "paginated");
    const resumeThreadId = forked.result.thread.id;
    const paginatedStats = await threadItemStats(connection, resumeThreadId);
    assertPagination(paginatedStats, { minimumItems: 201, minimumPages: 3 });
    const legacyStats = await threadItemStats(connection, legacyThreadId);
    assertPagination(legacyStats, { minimumItems: 101, minimumPages: 2 });

    // Stop the seed sidecar before launching the independent TUI sidecar.
    await connected.sidecar.close("SIGKILL");
    connected = undefined;
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
        legacyThreadId,
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
        "suite::history_pagination::",
        "--",
        "--test-threads=1",
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
          ...(runtimeEnv.DYLD_LIBRARY_PATH
            ? { LIME_TEST_DYLD_LIBRARY_PATH: runtimeEnv.DYLD_LIBRARY_PATH }
            : {}),
          LIME_TEST_TUI_HISTORY_THREAD_ID: resumeThreadId,
          LIME_TEST_TUI_LEGACY_THREAD_ID: legacyThreadId,
          LIME_TEST_TUI_GATE_B: "1",
        },
        maxBuffer: 4 * 1024 * 1024,
        timeout: 180_000,
        windowsHide: true,
      },
    );
    console.log(
      `[smoke:tui-history-pagination] ok paginated=${resumeThreadId} pages=${paginatedStats.pages} items=${paginatedStats.items} legacy=${legacyThreadId} pages=${legacyStats.pages} items=${legacyStats.items} review=nested-filtered underfilled=auto-filled alternate-screen=restored`,
    );
  } finally {
    await connected?.sidecar.close("SIGKILL").catch(() => undefined);
    if (process.env.LIME_KEEP_TUI_HISTORY_PAGINATION_TMP !== "1") {
      await rm(tempDir, { recursive: true, force: true });
    } else {
      console.error(`[smoke:tui-history-pagination] kept temp dir ${tempDir}`);
    }
  }
}

function assertHistoryMode(started, expected) {
  const actual = started?.result?.thread?.historyMode;
  if (actual !== expected) {
    throw new Error(`thread/start historyMode mismatch: expected=${expected} actual=${actual}`);
  }
}

async function seedCompletedTurns(connection, threadId, prefix, count) {
  for (let index = 0; index < count; index += 1) {
    const result = await connection.startTurn({
      threadId,
      input: [
        { type: "text", text: `${prefix}_${String(index).padStart(3, "0")}` },
      ],
    });
    await drainTurnCompletion(connection, result, result.result.turn.id);
  }
}

async function threadItemStats(connection, threadId) {
  const seenCursors = new Set();
  let cursor;
  let items = 0;
  let pages = 0;
  while (true) {
    const response = await connection.listThreadItems({
      threadId,
      cursor,
      limit: 100,
      sortDirection: "desc",
    });
    pages += 1;
    items += response.result.data.length;
    const nextCursor = response.result.nextCursor;
    if (!nextCursor) break;
    if (seenCursors.has(nextCursor)) {
      throw new Error(`thread/items/list repeated cursor for ${threadId}: ${nextCursor}`);
    }
    seenCursors.add(nextCursor);
    cursor = nextCursor;
  }
  return { items, pages };
}

function assertPagination(actual, expected) {
  if (
    actual.items < expected.minimumItems ||
    actual.pages < expected.minimumPages
  ) {
    throw new Error(
      `history pagination too small: expected>=${expected.minimumItems} items/${expected.minimumPages} pages actual=${actual.items} items/${actual.pages} pages`,
    );
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
  const assistantText = text.match(/(?:SEED|LEGACY)_\\d{3}/)?.[0] || "review complete";
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
