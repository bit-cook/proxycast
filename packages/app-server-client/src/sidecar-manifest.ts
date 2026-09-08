import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

import {
  APP_SERVER_METHODS,
  JSONRPC_VERSION,
  PROTOCOL_VERSION,
  type AppServerMethodSpec,
  type AppServerProtocolSchemaManifest,
  type ProtocolSchemaFile,
  type ProtocolSchemaGroup,
} from "./protocol.js";
import {
  DEFAULT_LISTEN_URL,
  DEFAULT_PROTOCOL_SCHEMA_MANIFEST_NAME,
  DEFAULT_RELEASE_MANIFEST_NAME,
  DEFAULT_STANDALONE_BACKEND_MODE,
  type AppServerArtifactPlatform,
  type AppServerReleaseArtifact,
  type AppServerReleaseManifest,
  type ResolveSidecarBinaryPathOptions,
  type ResolveSidecarFromManifestOptions,
  type ResolvedSidecarLaunchConfig,
  type SidecarBinaryPathResolution,
  type SidecarLaunchConfig,
} from "./sidecar-types.js";

function normalizeMethodSpecs(
  methods: readonly AppServerMethodSpec[],
): string[] {
  return methods
    .map((spec) => `${spec.kind}:${spec.method}`)
    .sort((left, right) => left.localeCompare(right));
}

export function sidecarBinaryName(
  platform: NodeJS.Platform | string = process.platform,
): string {
  return platform === "win32" ? "app-server.exe" : "app-server";
}

export function defaultPackagedSidecarRelativePath(
  platform: NodeJS.Platform | string = process.platform,
  arch: NodeJS.Architecture | string = process.arch,
): string {
  return path.join(
    "app-server",
    platformKey(platform, arch),
    sidecarBinaryName(platform),
  );
}

export function resolveSidecarBinaryPath(
  options: ResolveSidecarBinaryPathOptions = {},
): SidecarBinaryPathResolution | undefined {
  const env = options.env ?? process.env;
  const envVarName = options.envVarName ?? "APP_SERVER_BIN";
  if (options.allowEnvOverride ?? true) {
    const envBinaryPath = env[envVarName]?.trim();
    if (envBinaryPath) {
      return {
        binaryPath: envBinaryPath,
        source: "env",
      };
    }
  }

  if (options.resourcesPath?.trim()) {
    return {
      binaryPath: path.join(
        options.resourcesPath,
        options.resourceRelativePath ??
          defaultPackagedSidecarRelativePath(options.platform, options.arch),
      ),
      source: "resources",
    };
  }

  if (options.devBinaryPath?.trim()) {
    return {
      binaryPath: options.devBinaryPath,
      source: "dev",
    };
  }

  return undefined;
}

export function stdioSidecar(
  binaryPath: string,
  appPolicyPath?: string,
  dataDir?: string,
): SidecarLaunchConfig {
  const normalizedDataDir = normalizeSidecarDataDir(dataDir);
  return {
    binaryPath,
    listenUrl: DEFAULT_LISTEN_URL,
    backendMode: DEFAULT_STANDALONE_BACKEND_MODE,
    ...(appPolicyPath ? { appPolicyPath } : {}),
    ...(normalizedDataDir ? { dataDir: normalizedDataDir } : {}),
  };
}

export function sidecarFromReleaseArtifact(
  binaryPath: string,
  artifact: AppServerReleaseArtifact,
  listenUrl = DEFAULT_LISTEN_URL,
  backendMode: SidecarLaunchConfig["backendMode"] = DEFAULT_STANDALONE_BACKEND_MODE,
  appPolicyPath?: string,
  dataDir?: string,
): SidecarLaunchConfig {
  const normalizedDataDir = normalizeSidecarDataDir(dataDir);
  return {
    binaryPath,
    listenUrl,
    backendMode,
    ...(appPolicyPath ? { appPolicyPath } : {}),
    ...(normalizedDataDir ? { dataDir: normalizedDataDir } : {}),
    expectedSha256: artifact.sha256,
    artifact,
  };
}

export function sidecarArgs(config: SidecarLaunchConfig): string[] {
  const args =
    config.listenUrl === DEFAULT_LISTEN_URL
      ? ["--stdio"]
      : ["--listen", config.listenUrl];
  args.push("--backend", config.backendMode ?? DEFAULT_STANDALONE_BACKEND_MODE);
  if (config.backendMode === "external" && config.backendCommand) {
    args.push("--backend-command", config.backendCommand);
  }
  for (const backendArg of config.backendMode === "external"
    ? (config.backendArgs ?? [])
    : []) {
    args.push("--backend-arg", backendArg);
  }
  if (
    config.backendMode === "external" &&
    config.backendTimeoutMs !== undefined
  ) {
    args.push("--backend-timeout-ms", String(config.backendTimeoutMs));
  }
  if (config.appPolicyPath) {
    args.push("--app-policy", config.appPolicyPath);
  }
  const dataDir = normalizeSidecarDataDir(config.dataDir);
  if (dataDir) {
    args.push("--data-dir", dataDir);
  }
  return args;
}

export function platformKey(
  platform: NodeJS.Platform | string = process.platform,
  arch: NodeJS.Architecture | string = process.arch,
): AppServerArtifactPlatform {
  if (platform === "win32") {
    return "win32-x64";
  }
  if (platform === "darwin" && arch === "arm64") {
    return "darwin-arm64";
  }
  if (platform === "darwin") {
    return "darwin-x64";
  }
  if (platform === "linux") {
    return "linux-x64";
  }
  return `${platform}-${arch}`;
}

export function findReleaseArtifact(
  manifest: AppServerReleaseManifest,
  platform: AppServerArtifactPlatform = platformKey(),
): AppServerReleaseArtifact | undefined {
  return manifest.artifacts.find((artifact) => artifact.platform === platform);
}

export function resolveSidecarFromReleaseManifest(
  manifest: AppServerReleaseManifest,
  options: ResolveSidecarFromManifestOptions = {},
): ResolvedSidecarLaunchConfig | undefined {
  assertCompatibleManifest(
    manifest,
    options.expectedProtocolVersion ?? PROTOCOL_VERSION,
  );
  const artifact = findReleaseArtifact(
    manifest,
    platformKey(options.platform, options.arch),
  );
  if (!artifact) {
    return undefined;
  }

  const binaryPath = resolveSidecarBinaryPath(options);
  if (!binaryPath) {
    return undefined;
  }

  const dataDir = normalizeSidecarDataDir(options.dataDir);

  return {
    config: {
      binaryPath: binaryPath.binaryPath,
      listenUrl: options.listenUrl ?? DEFAULT_LISTEN_URL,
      backendMode: options.backendMode ?? DEFAULT_STANDALONE_BACKEND_MODE,
      ...(options.backendCommand
        ? { backendCommand: options.backendCommand }
        : {}),
      ...(options.backendArgs ? { backendArgs: options.backendArgs } : {}),
      ...(options.backendTimeoutMs !== undefined
        ? { backendTimeoutMs: options.backendTimeoutMs }
        : {}),
      ...(options.appPolicyPath
        ? { appPolicyPath: options.appPolicyPath }
        : {}),
      ...(dataDir ? { dataDir } : {}),
      expectedSha256:
        binaryPath.source === "resources" ? artifact.sha256 : undefined,
      artifact,
    },
    artifact,
    binaryPathSource: binaryPath.source,
  };
}

/**
 * A sidecar data root must be an explicit filesystem path. In particular,
 * stringified optional values such as "undefined" must never become a
 * relative path under the caller's working directory.
 */
function normalizeSidecarDataDir(dataDir: string | undefined): string | undefined {
  if (dataDir === undefined) {
    return undefined;
  }
  const normalized = dataDir.trim();
  if (!normalized) {
    throw new Error("App Server data directory must not be empty");
  }
  if (normalized === "undefined" || normalized === "null") {
    throw new Error("App Server data directory is invalid");
  }
  if (/[\u0000-\u001f\u007f]/u.test(normalized)) {
    throw new Error("App Server data directory contains control characters");
  }
  if (!path.isAbsolute(normalized) && !path.win32.isAbsolute(normalized)) {
    throw new Error("App Server data directory must be an absolute path");
  }
  return normalized;
}

export async function readReleaseManifest(
  path: string,
): Promise<AppServerReleaseManifest> {
  return JSON.parse(await readFile(path, "utf8")) as AppServerReleaseManifest;
}

export async function readProtocolSchemaManifest(
  manifestPath: string,
): Promise<AppServerProtocolSchemaManifest> {
  return JSON.parse(
    await readFile(manifestPath, "utf8"),
  ) as AppServerProtocolSchemaManifest;
}

export async function resolveSidecarFromReleaseManifestFile(
  manifestPath: string,
  options: ResolveSidecarFromManifestOptions = {},
): Promise<ResolvedSidecarLaunchConfig | undefined> {
  return resolveSidecarFromReleaseManifest(
    await readReleaseManifest(manifestPath),
    options,
  );
}

export function defaultReleaseManifestPath(
  resourcesPath: string,
  manifestRelativePath = DEFAULT_RELEASE_MANIFEST_NAME,
): string {
  return path.join(resourcesPath, manifestRelativePath);
}

export function defaultProtocolSchemaManifestPath(
  schemaJsonRoot: string,
  manifestRelativePath = DEFAULT_PROTOCOL_SCHEMA_MANIFEST_NAME,
): string {
  return path.join(schemaJsonRoot, manifestRelativePath);
}

export function assertCompatibleManifest(
  manifest: AppServerReleaseManifest,
  expectedProtocolVersion = PROTOCOL_VERSION,
): void {
  if (manifest.protocolVersion !== expectedProtocolVersion) {
    throw new Error(
      `unsupported app-server protocol: expected ${expectedProtocolVersion}, got ${manifest.protocolVersion}`,
    );
  }
}

export function assertCompatibleProtocolSchemaManifest(
  manifest: AppServerProtocolSchemaManifest,
  expectedProtocolVersion = PROTOCOL_VERSION,
  expectedMethods: readonly AppServerMethodSpec[] = APP_SERVER_METHODS,
): void {
  if (manifest.protocolVersion !== expectedProtocolVersion) {
    throw new Error(
      `unsupported app-server schema protocol: expected ${expectedProtocolVersion}, got ${manifest.protocolVersion}`,
    );
  }
  if (manifest.jsonRpc.version !== JSONRPC_VERSION) {
    throw new Error(
      `unsupported JSON-RPC schema version: expected ${JSONRPC_VERSION}, got ${manifest.jsonRpc.version}`,
    );
  }
  const actualMethods = normalizeMethodSpecs(manifest.methods);
  const expectedMethodList = normalizeMethodSpecs(expectedMethods);
  if (actualMethods.join("\n") !== expectedMethodList.join("\n")) {
    throw new Error("app-server schema method catalog mismatch");
  }
}

export function protocolSchemaFilePath(
  schemaJsonRoot: string,
  group: ProtocolSchemaGroup,
  typeName: string,
): string {
  return path.join(schemaJsonRoot, group, `${typeName}.json`);
}

export function listProtocolSchemaFiles(
  manifest: AppServerProtocolSchemaManifest,
  schemaJsonRoot: string,
): ProtocolSchemaFile[] {
  return (["jsonrpc", "v0", "v2"] as const).flatMap((group) =>
    (manifest.schemas[group] ?? []).map((typeName) => ({
      group,
      typeName,
      path: protocolSchemaFilePath(schemaJsonRoot, group, typeName),
    })),
  );
}

export function sha256Hex(content: Buffer | Uint8Array | string): string {
  return createHash("sha256").update(content).digest("hex");
}

export async function sha256File(path: string): Promise<string> {
  return sha256Hex(await readFile(path));
}

function normalizeSha256(value: string): string {
  return value.trim().toLowerCase();
}

export function assertSha256(
  actualSha256: string,
  expectedSha256: string,
): void {
  if (normalizeSha256(actualSha256) !== normalizeSha256(expectedSha256)) {
    throw new Error("app-server sha256 mismatch");
  }
}

export async function assertSidecarFileSha256(
  config: SidecarLaunchConfig,
): Promise<void> {
  if (!config.expectedSha256) {
    throw new Error("sidecar expectedSha256 is required");
  }
  assertSha256(await sha256File(config.binaryPath), config.expectedSha256);
}
