import {
  access,
  mkdir,
  readFile,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { isIP } from "node:net";

const CREDENTIALS_DIRECTORY = "credentials";
const CREDENTIAL_FILE_NAME = "app-server-session.v1";
const CREDENTIAL_VERSION = 1;
const MAX_TENANT_ID_LENGTH = 256;
const MAX_ENDPOINT_LENGTH = 2_048;
const MAX_TOKEN_LENGTH = 64 * 1024;

export type CloudSessionCredential = {
  tenantId: string;
  endpoint: string;
  token: string;
};

export type CloudSessionCredentialMetadata = {
  available: boolean;
  exists: boolean;
  tenantId?: string;
  endpoint?: string;
  updatedAt?: string;
};

export interface SecureCredentialStore {
  setCloudSessionCredential(
    credential: CloudSessionCredential,
  ): Promise<CloudSessionCredentialMetadata>;
  readCloudSessionCredential(): Promise<CloudSessionCredential | null>;
  getCloudSessionCredentialMetadata(): Promise<CloudSessionCredentialMetadata>;
  deleteCloudSessionCredential(): Promise<{ deleted: boolean }>;
}

export interface SafeStorageLike {
  isEncryptionAvailable(): boolean;
  encryptString(plainText: string): Buffer;
  decryptString(encrypted: Buffer): string;
}

type CredentialEnvelope = {
  version: typeof CREDENTIAL_VERSION;
  credential: CloudSessionCredential;
  updatedAt: string;
};

export class ElectronSecureCredentialStore implements SecureCredentialStore {
  readonly #filePath: string;
  readonly #safeStorage: SafeStorageLike;

  constructor(appDataRoot: string, safeStorage: SafeStorageLike) {
    this.#filePath = path.join(
      path.resolve(appDataRoot),
      CREDENTIALS_DIRECTORY,
      CREDENTIAL_FILE_NAME,
    );
    this.#safeStorage = safeStorage;
  }

  async setCloudSessionCredential(
    credential: CloudSessionCredential,
  ): Promise<CloudSessionCredentialMetadata> {
    const normalized = normalizeCredential(credential);
    this.#assertEncryptionAvailable();

    const envelope: CredentialEnvelope = {
      version: CREDENTIAL_VERSION,
      credential: normalized,
      updatedAt: new Date().toISOString(),
    };
    const encrypted = this.#safeStorage.encryptString(JSON.stringify(envelope));
    const directory = path.dirname(this.#filePath);
    await mkdir(directory, { recursive: true, mode: 0o700 });

    const tempPath = `${this.#filePath}.${process.pid}.${Date.now()}.tmp`;
    let moved = false;
    try {
      await writeFile(tempPath, encrypted, { mode: 0o600 });
      try {
        await rename(tempPath, this.#filePath);
      } catch (error) {
        const code =
          error && typeof error === "object"
            ? (error as NodeJS.ErrnoException).code
            : undefined;
        if (code !== "EEXIST" && code !== "EPERM") {
          throw error;
        }
        await rm(this.#filePath, { force: true });
        await rename(tempPath, this.#filePath);
      }
      moved = true;
    } finally {
      if (!moved) {
        await rm(tempPath, { force: true }).catch(() => undefined);
      }
    }

    return metadataFromEnvelope(envelope, true);
  }

  async readCloudSessionCredential(): Promise<CloudSessionCredential | null> {
    const envelope = await this.#readEnvelope();
    return envelope?.credential ?? null;
  }

  async getCloudSessionCredentialMetadata(): Promise<CloudSessionCredentialMetadata> {
    const exists = await fileExists(this.#filePath);
    const available = this.#isEncryptionAvailable();
    if (!exists || !available) {
      return { available, exists };
    }

    const envelope = await this.#readEnvelope();
    return envelope
      ? metadataFromEnvelope(envelope, true)
      : { available, exists: false };
  }

  async deleteCloudSessionCredential(): Promise<{ deleted: boolean }> {
    const existed = await fileExists(this.#filePath);
    await rm(this.#filePath, { force: true });
    return { deleted: existed };
  }

  async #readEnvelope(): Promise<CredentialEnvelope | null> {
    if (!(await fileExists(this.#filePath))) {
      return null;
    }
    this.#assertEncryptionAvailable();

    try {
      const encrypted = await readFile(this.#filePath);
      const decoded = JSON.parse(
        this.#safeStorage.decryptString(encrypted),
      ) as unknown;
      return parseEnvelope(decoded);
    } catch {
      throw new Error("stored Cloud session credential is invalid");
    }
  }

  #isEncryptionAvailable(): boolean {
    try {
      return this.#safeStorage.isEncryptionAvailable();
    } catch {
      return false;
    }
  }

  #assertEncryptionAvailable(): void {
    if (!this.#isEncryptionAvailable()) {
      throw new Error("secure credential storage is unavailable");
    }
  }
}

function normalizeCredential(
  credential: CloudSessionCredential,
): CloudSessionCredential {
  const tenantId = normalizeField(
    credential.tenantId,
    "tenantId",
    MAX_TENANT_ID_LENGTH,
  );
  const endpoint = normalizeEndpoint(credential.endpoint);
  const token = normalizeField(credential.token, "token", MAX_TOKEN_LENGTH);
  return { tenantId, endpoint, token };
}

function normalizeField(
  value: unknown,
  name: string,
  maxLength: number,
): string {
  if (typeof value !== "string") {
    throw new Error(`${name} must be a string`);
  }
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`${name} must not be empty`);
  }
  if (normalized.length > maxLength) {
    throw new Error(`${name} exceeds the maximum length`);
  }
  if (/[\u0000-\u001f\u007f]/u.test(normalized)) {
    throw new Error(`${name} contains control characters`);
  }
  return normalized;
}

function normalizeEndpoint(value: unknown): string {
  const endpoint = normalizeField(value, "endpoint", MAX_ENDPOINT_LENGTH);
  let url: URL;
  try {
    url = new URL(endpoint);
  } catch {
    throw new Error("endpoint must be a valid WebSocket URL");
  }
  if (url.protocol !== "ws:" && url.protocol !== "wss:") {
    throw new Error("endpoint must use ws:// or wss://");
  }
  if (!url.hostname || url.username || url.password || url.search || url.hash) {
    throw new Error("endpoint must not contain userinfo, query, or fragment");
  }
  if (url.protocol === "ws:" && !isLoopbackUrl(url)) {
    throw new Error("insecure ws:// endpoints must be loopback URLs");
  }
  return url.toString();
}

function isLoopbackUrl(url: URL): boolean {
  const hostname = url.hostname.toLowerCase();
  return (
    hostname === "localhost" ||
    hostname === "[::1]" ||
    (isIP(hostname) === 4 && hostname.startsWith("127.")) ||
    (isIP(hostname) === 6 && hostname === "::1")
  );
}

function parseEnvelope(value: unknown): CredentialEnvelope {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("invalid credential envelope");
  }
  const record = value as Record<string, unknown>;
  if (
    record.version !== CREDENTIAL_VERSION ||
    typeof record.updatedAt !== "string"
  ) {
    throw new Error("invalid credential envelope");
  }
  const credential = record.credential;
  if (
    !credential ||
    typeof credential !== "object" ||
    Array.isArray(credential)
  ) {
    throw new Error("invalid credential envelope");
  }
  const normalized = normalizeCredential(credential as CloudSessionCredential);
  return {
    version: CREDENTIAL_VERSION,
    credential: normalized,
    updatedAt: record.updatedAt,
  };
}

function metadataFromEnvelope(
  envelope: CredentialEnvelope,
  available: boolean,
): CloudSessionCredentialMetadata {
  return {
    available,
    exists: true,
    tenantId: envelope.credential.tenantId,
    endpoint: envelope.credential.endpoint,
    updatedAt: envelope.updatedAt,
  };
}

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}
