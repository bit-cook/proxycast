import type {
  OemCloudAuthCatalogProvider,
  OemCloudAuthPolicy,
  OemCloudAppServerConfig,
  OemCloudCurrentSession,
  OemCloudCustomScene,
  OemCloudDesktopAuthSessionStartResponse,
  OemCloudDesktopAuthSessionStatus,
  OemCloudDesktopAuthSessionStatusResponse,
  OemCloudFeatureFlags,
  OemCloudGatewayConfig,
  OemCloudProviderModelItem,
  OemCloudProviderOfferDetail,
  OemCloudProviderOfferState,
  OemCloudProviderOfferSummary,
  OemCloudProviderPreference,
  OemCloudProviderSource,
  OemCloudPublicAuthCatalog,
  OemCloudSceneSkillPreference,
  OemCloudSceneSkillTemplate,
} from "./oemCloudControlPlaneTypes";
import {
  OemCloudControlPlaneError,
  MODEL_MODALITY_SET,
  MODEL_RUNTIME_FEATURE_SET,
  MODEL_TASK_FAMILY_SET,
  isRecord,
  normalizeBoolean,
  normalizeNumber,
  normalizeStringArray,
  normalizeText,
  normalizeTypedStringArray,
  parseOptionalModelAliasSource,
  parseOptionalModelDeploymentSource,
  parseOptionalModelManagementPlane,
  parsePartnerHubAccessMode,
  parsePartnerHubConfigMode,
  parsePartnerHubModelsSource,
} from "./oemCloudControlPlaneRuntime";

export function parseCurrentSession(value: unknown): OemCloudCurrentSession {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("当前会话格式非法");
  }

  const tenant = isRecord(value.tenant) ? value.tenant : null;
  const user = isRecord(value.user) ? value.user : null;
  const session = isRecord(value.session) ? value.session : null;
  const tenantId = normalizeText(tenant?.id);
  const userId = normalizeText(user?.id);
  const sessionId = normalizeText(session?.id);

  if (!tenantId || !userId || !sessionId) {
    throw new OemCloudControlPlaneError("当前会话格式非法");
  }

  return {
    token: normalizeText(value.token),
    tenant: {
      id: tenantId,
      name: normalizeText(tenant?.name) ?? tenantId,
      slug: normalizeText(tenant?.slug) ?? tenantId,
    },
    user: {
      id: userId,
      email: normalizeText(user?.email) ?? "",
      displayName: normalizeText(user?.displayName) ?? userId,
      avatarUrl: normalizeText(user?.avatarUrl),
      username: normalizeText(user?.username),
      passwordConfigured: normalizeBoolean(user?.passwordConfigured),
      roles: normalizeStringArray(user?.roles),
    },
    session: {
      id: sessionId,
      tenantId: normalizeText(session?.tenantId) ?? tenantId,
      userId: normalizeText(session?.userId) ?? userId,
      provider: normalizeText(session?.provider) ?? "password",
      roles: normalizeStringArray(session?.roles),
      issuedAt: normalizeText(session?.issuedAt) ?? "",
      expiresAt: normalizeText(session?.expiresAt) ?? "",
    },
  };
}

function parseAuthCatalogProvider(value: unknown): OemCloudAuthCatalogProvider {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("登录方式格式非法");
  }

  const provider = normalizeText(value.provider);
  const displayName = normalizeText(value.displayName) ?? provider;
  if (!provider || !displayName) {
    throw new OemCloudControlPlaneError("登录方式格式非法");
  }

  return {
    provider,
    displayName,
    authorizeUrl: normalizeText(value.authorizeUrl) ?? undefined,
    redirectUri: normalizeText(value.redirectUri) ?? undefined,
    scopes: normalizeStringArray(value.scopes),
    enabled: normalizeBoolean(value.enabled, true),
    loginHint: normalizeText(value.loginHint) ?? undefined,
  };
}

export function parseAuthPolicy(value: unknown): OemCloudAuthPolicy {
  const record = isRecord(value) ? value : {};
  const startupTrigger = normalizeText(record.startupTrigger);

  return {
    required: normalizeBoolean(record.required),
    startupTrigger: startupTrigger === "oauth" ? "oauth" : "none",
    primaryProvider: normalizeText(record.primaryProvider) ?? undefined,
  };
}

export function parsePublicAuthCatalog(
  value: unknown,
): OemCloudPublicAuthCatalog {
  const record = isRecord(value) ? value : {};
  return {
    providers: Array.isArray(record.items)
      ? record.items.map(parseAuthCatalogProvider)
      : [],
    authPolicy: parseAuthPolicy(record.authPolicy),
  };
}

export function parseProviderOfferSummary(
  value: unknown,
): OemCloudProviderOfferSummary {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("服务商摘要格式非法");
  }

  const providerKey = normalizeText(value.providerKey);
  const displayName = normalizeText(value.displayName);
  const source = normalizeText(value.source) as
    | OemCloudProviderSource
    | undefined;
  const state = normalizeText(value.state) as
    | OemCloudProviderOfferState
    | undefined;

  if (!providerKey || !displayName || !source || !state) {
    throw new OemCloudControlPlaneError("服务商摘要格式非法");
  }

  return {
    providerKey,
    displayName,
    source,
    state,
    logoUrl: normalizeText(value.logoUrl),
    description: normalizeText(value.description),
    supportUrl: normalizeText(value.supportUrl),
    visible: normalizeBoolean(value.visible),
    loggedIn: normalizeBoolean(value.loggedIn),
    accountStatus: normalizeText(value.accountStatus) ?? "anonymous",
    subscriptionStatus: normalizeText(value.subscriptionStatus) ?? "none",
    quotaStatus: normalizeText(value.quotaStatus) ?? "ok",
    canInvoke: normalizeBoolean(value.canInvoke),
    defaultModel: normalizeText(value.defaultModel),
    effectiveAccessMode: parsePartnerHubAccessMode(
      value.effectiveAccessMode,
      "session",
    ),
    apiKeyModeEnabled: normalizeBoolean(value.apiKeyModeEnabled),
    tenantOverrideApplied: normalizeBoolean(value.tenantOverrideApplied),
    configMode: parsePartnerHubConfigMode(value.configMode, "managed"),
    modelsSource: parsePartnerHubModelsSource(
      value.modelsSource,
      "hub_catalog",
    ),
    developerAccessVisible: normalizeBoolean(value.developerAccessVisible),
    availableModelCount:
      typeof value.availableModelCount === "number"
        ? value.availableModelCount
        : 0,
    fallbackToLocalAllowed: normalizeBoolean(value.fallbackToLocalAllowed),
    currentPlan: normalizeText(value.currentPlan),
    creditsSummary: normalizeText(value.creditsSummary),
    statusReason: normalizeText(value.statusReason),
    tags: normalizeStringArray(value.tags),
  };
}

export function parseProviderOfferDetail(
  value: unknown,
): OemCloudProviderOfferDetail {
  const summary = parseProviderOfferSummary(value);
  const access = isRecord((value as Record<string, unknown>).access)
    ? ((value as Record<string, unknown>).access as Record<string, unknown>)
    : null;
  if (!access) {
    throw new OemCloudControlPlaneError("服务商详情格式非法");
  }

  const offerId = normalizeText(access.offerId);
  if (!offerId) {
    throw new OemCloudControlPlaneError("服务商详情格式非法");
  }

  return {
    ...summary,
    loginHint: normalizeText((value as Record<string, unknown>).loginHint),
    subscribeHint: normalizeText(
      (value as Record<string, unknown>).subscribeHint,
    ),
    unavailableHint: normalizeText(
      (value as Record<string, unknown>).unavailableHint,
    ),
    access: {
      offerId,
      accessMode: parsePartnerHubAccessMode(
        access.accessMode,
        summary.effectiveAccessMode,
      ),
      sessionTokenRef: normalizeText(access.sessionTokenRef),
      hubTokenRef: normalizeText(access.hubTokenRef),
      hubTokenEnabled: normalizeBoolean(access.hubTokenEnabled),
      lastIssuedAt: normalizeText(access.lastIssuedAt),
    },
  };
}

export function parseProviderModelItem(
  value: unknown,
): OemCloudProviderModelItem {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("服务商模型格式非法");
  }

  const id = normalizeText(value.id);
  const offerId = normalizeText(value.offerId);
  const modelId = normalizeText(value.modelId);
  const displayName = normalizeText(value.displayName);
  if (!id || !offerId || !modelId || !displayName) {
    throw new OemCloudControlPlaneError("服务商模型格式非法");
  }

  return {
    id,
    offerId,
    modelId,
    displayName,
    description: normalizeText(value.description),
    abilities: normalizeStringArray(value.abilities),
    task_families: normalizeTypedStringArray(
      value.task_families,
      MODEL_TASK_FAMILY_SET,
    ),
    input_modalities: normalizeTypedStringArray(
      value.input_modalities,
      MODEL_MODALITY_SET,
    ),
    output_modalities: normalizeTypedStringArray(
      value.output_modalities,
      MODEL_MODALITY_SET,
    ),
    runtime_features: normalizeTypedStringArray(
      value.runtime_features,
      MODEL_RUNTIME_FEATURE_SET,
    ),
    deployment_source: parseOptionalModelDeploymentSource(
      value.deployment_source,
    ),
    management_plane: parseOptionalModelManagementPlane(value.management_plane),
    canonical_model_id: normalizeText(value.canonical_model_id),
    provider_model_id: normalizeText(value.provider_model_id),
    alias_source: parseOptionalModelAliasSource(value.alias_source) ?? null,
    recommended: normalizeBoolean(value.recommended),
    status: normalizeText(value.status) ?? "active",
    sort: typeof value.sort === "number" ? value.sort : 0,
    upstreamMapping: normalizeText(value.upstreamMapping),
    createdAt: normalizeText(value.createdAt) ?? "",
    updatedAt: normalizeText(value.updatedAt) ?? "",
  };
}

export function parseProviderPreference(
  value: unknown,
): OemCloudProviderPreference {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("默认服务商配置格式非法");
  }

  const tenantId = normalizeText(value.tenantId);
  const userId = normalizeText(value.userId);
  const providerSource = normalizeText(value.providerSource) as
    | OemCloudProviderSource
    | undefined;
  const providerKey = normalizeText(value.providerKey);
  if (!tenantId || !userId || !providerSource || !providerKey) {
    throw new OemCloudControlPlaneError("默认服务商配置格式非法");
  }

  return {
    tenantId,
    userId,
    providerSource,
    providerKey,
    defaultModel: normalizeText(value.defaultModel),
    needsValidation: normalizeBoolean(value.needsValidation),
    lastValidatedAt: normalizeText(value.lastValidatedAt),
    updatedAt: normalizeText(value.updatedAt) ?? "",
  };
}

function parseSceneSkillTemplate(
  value: unknown,
): OemCloudSceneSkillTemplate | null {
  if (!isRecord(value)) {
    return null;
  }

  const id = normalizeText(value.id);
  const title = normalizeText(value.title);
  const prompt = normalizeText(value.prompt);
  if (!id || !title || !prompt) {
    return null;
  }

  return {
    id,
    title,
    description: normalizeText(value.description),
    prompt,
  };
}

function parseCustomScene(value: unknown): OemCloudCustomScene | null {
  if (!isRecord(value)) {
    return null;
  }

  const title = normalizeText(value.title);
  const linkedEntryId = normalizeText(value.linkedEntryId);
  const templates = Array.isArray(value.templates)
    ? value.templates
        .map(parseSceneSkillTemplate)
        .filter((item): item is OemCloudSceneSkillTemplate => Boolean(item))
    : [];
  if (!title || !linkedEntryId || templates.length === 0) {
    return null;
  }

  return {
    id: normalizeText(value.id),
    title,
    summary: normalizeText(value.summary),
    linkedEntryId,
    placeholder: normalizeText(value.placeholder),
    templates,
    enabled: normalizeBoolean(value.enabled),
  };
}

export function parseSceneSkillPreference(
  value: unknown,
): OemCloudSceneSkillPreference {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("场景技能偏好格式非法");
  }

  const tenantId = normalizeText(value.tenantId);
  const userId = normalizeText(value.userId);
  if (!tenantId || !userId) {
    throw new OemCloudControlPlaneError("场景技能偏好格式非法");
  }

  return {
    tenantId,
    userId,
    orderedEntryIds: normalizeStringArray(value.orderedEntryIds),
    hiddenEntryIds: normalizeStringArray(value.hiddenEntryIds),
    customScenes: Array.isArray(value.customScenes)
      ? value.customScenes
          .map(parseCustomScene)
          .filter((item): item is OemCloudCustomScene => Boolean(item))
      : [],
    updatedAt: normalizeText(value.updatedAt),
  };
}

export function parseFeatureFlags(value: unknown): OemCloudFeatureFlags {
  const record = isRecord(value) ? value : {};
  return {
    oauthLoginEnabled: normalizeBoolean(record.oauthLoginEnabled),
    emailCodeLoginEnabled: normalizeBoolean(record.emailCodeLoginEnabled),
    passwordLoginEnabled: normalizeBoolean(record.passwordLoginEnabled, true),
    profileEditable: normalizeBoolean(record.profileEditable),
    hubTokensEnabled: normalizeBoolean(record.hubTokensEnabled),
    billingEnabled: normalizeBoolean(record.billingEnabled),
    referralEnabled: normalizeBoolean(record.referralEnabled),
    gatewayEnabled: normalizeBoolean(record.gatewayEnabled),
  };
}

const DESKTOP_AUTH_SESSION_STATUS_SET =
  new Set<OemCloudDesktopAuthSessionStatus>([
    "pending_login",
    "pending_consent",
    "approved",
    "denied",
    "cancelled",
    "consumed",
    "expired",
  ]);

function parseDesktopAuthSessionStatus(
  value: unknown,
): OemCloudDesktopAuthSessionStatus {
  const status = normalizeText(value) as
    | OemCloudDesktopAuthSessionStatus
    | undefined;
  if (!status || !DESKTOP_AUTH_SESSION_STATUS_SET.has(status)) {
    throw new OemCloudControlPlaneError("桌面授权状态格式非法");
  }

  return status;
}

function parseDesktopAuthDuration(value: unknown, fieldName: string): number {
  const duration = normalizeNumber(value);
  if (duration === undefined) {
    throw new OemCloudControlPlaneError(`${fieldName} 格式非法`);
  }

  return duration;
}

export function parseDesktopAuthSessionStartResponse(
  value: unknown,
): OemCloudDesktopAuthSessionStartResponse {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("桌面授权会话创建结果格式非法");
  }

  const authSessionId = normalizeText(value.authSessionId);
  const deviceCode = normalizeText(value.deviceCode);
  const tenantId = normalizeText(value.tenantId);
  const clientId = normalizeText(value.clientId);
  const clientName = normalizeText(value.clientName);
  const authorizeUrl = normalizeText(value.authorizeUrl);

  if (
    !authSessionId ||
    !deviceCode ||
    !tenantId ||
    !clientId ||
    !clientName ||
    !authorizeUrl
  ) {
    throw new OemCloudControlPlaneError("桌面授权会话创建结果格式非法");
  }

  return {
    authSessionId,
    deviceCode,
    tenantId,
    clientId,
    clientName,
    provider: normalizeText(value.provider),
    desktopRedirectUri: normalizeText(value.desktopRedirectUri),
    status: parseDesktopAuthSessionStatus(value.status),
    expiresInSeconds: parseDesktopAuthDuration(
      value.expiresInSeconds,
      "桌面授权会话过期时间",
    ),
    pollIntervalSeconds: parseDesktopAuthDuration(
      value.pollIntervalSeconds,
      "桌面授权轮询间隔",
    ),
    authorizeUrl,
  };
}

export function parseDesktopAuthSessionStatusResponse(
  value: unknown,
): OemCloudDesktopAuthSessionStatusResponse {
  if (!isRecord(value)) {
    throw new OemCloudControlPlaneError("桌面授权状态格式非法");
  }

  const deviceCode = normalizeText(value.deviceCode);
  const tenantId = normalizeText(value.tenantId);
  const clientId = normalizeText(value.clientId);
  const clientName = normalizeText(value.clientName);

  if (!deviceCode || !tenantId || !clientId || !clientName) {
    throw new OemCloudControlPlaneError("桌面授权状态格式非法");
  }

  return {
    deviceCode,
    tenantId,
    clientId,
    clientName,
    provider: normalizeText(value.provider),
    desktopRedirectUri: normalizeText(value.desktopRedirectUri),
    status: parseDesktopAuthSessionStatus(value.status),
    expiresInSeconds: parseDesktopAuthDuration(
      value.expiresInSeconds,
      "桌面授权状态过期时间",
    ),
    pollIntervalSeconds: parseDesktopAuthDuration(
      value.pollIntervalSeconds,
      "桌面授权轮询间隔",
    ),
    sessionToken: normalizeText(value.sessionToken),
    sessionExpiresAt: normalizeText(value.sessionExpiresAt),
  };
}

export function parseGatewayConfig(value: unknown): OemCloudGatewayConfig {
  if (!isRecord(value)) {
    return {};
  }

  return {
    basePath: normalizeText(value.basePath),
    llmBaseUrl: normalizeText(value.llmBaseUrl),
    openAIBaseUrl: normalizeText(value.openAIBaseUrl),
    anthropicBaseUrl: normalizeText(value.anthropicBaseUrl),
    chatCompletionsPath: normalizeText(value.chatCompletionsPath),
    authorizationHeader: normalizeText(value.authorizationHeader),
    authorizationScheme: normalizeText(value.authorizationScheme),
    tenantHeader: normalizeText(value.tenantHeader),
  };
}

export function parseAppServerConfig(value: unknown): OemCloudAppServerConfig {
  const record = isRecord(value) ? value : {};
  const enabled = normalizeBoolean(record.enabled);
  const endpoint = normalizeText(record.endpoint);

  if (endpoint) {
    let parsed: URL;
    try {
      parsed = new URL(endpoint);
    } catch {
      throw new OemCloudControlPlaneError("App Server endpoint 格式非法");
    }
    if (
      (parsed.protocol !== "ws:" && parsed.protocol !== "wss:") ||
      !parsed.host ||
      parsed.username ||
      parsed.password ||
      parsed.search ||
      parsed.hash
    ) {
      throw new OemCloudControlPlaneError(
        "App Server endpoint 必须是无凭证、无 query/fragment 的 ws/wss URL",
      );
    }
  }

  if (enabled && !endpoint) {
    throw new OemCloudControlPlaneError("App Server 已启用但缺少 endpoint");
  }

  return {
    enabled: enabled && Boolean(endpoint),
    endpoint,
    transport: "websocket",
    protocol: "appserver.v0",
    authorizationHeader: "Authorization",
    authorizationScheme: "Bearer",
    tenantHeader: "X-Lime-Tenant-ID",
  };
}
