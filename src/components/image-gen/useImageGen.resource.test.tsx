import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanupMountedRoots,
  flushEffects,
  renderIntoDom,
  setReactActEnvironment,
  silenceConsole,
  type MountedRoot,
} from "./test-utils";

const { mockGetNextApiKey, mockInvoke } = vi.hoisted(() => ({
  mockGetNextApiKey: vi.fn(),
  mockInvoke: vi.fn(),
}));

vi.mock("@/hooks/useApiKeyProvider", () => ({
  useApiKeyProvider: () => ({
    providers: [
      {
        id: "zhipuai",
        type: "zhipuai",
        name: "智谱AI",
        enabled: true,
        api_key_count: 1,
        api_host: "https://api.zhipu.test",
      },
    ],
    loading: false,
  }),
}));

vi.mock("@/lib/api/apiKeyProvider", () => ({
  apiKeyProviderApi: {
    getNextApiKey: mockGetNextApiKey,
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

import { useImageGen } from "./useImageGen";

interface HookHarness {
  getValue: () => ReturnType<typeof useImageGen>;
}

const mountedRoots: MountedRoot[] = [];

function mountHook(): HookHarness {
  let hookValue: ReturnType<typeof useImageGen> | null = null;

  function TestComponent() {
    hookValue = useImageGen();
    return null;
  }

  renderIntoDom(<TestComponent />, mountedRoots);

  return {
    getValue: () => {
      if (!hookValue) {
        throw new Error("hook 尚未初始化");
      }
      return hookValue;
    },
  };
}

async function waitForReady(
  harness: HookHarness,
  timeout = 40,
): Promise<void> {
  for (let i = 0; i < timeout; i += 1) {
    const value = harness.getValue();
    if (value.selectedProvider && value.selectedModelId) {
      return;
    }
    await flushEffects();
  }
  throw new Error("useImageGen 未在预期时间内就绪");
}

function createSuccessResponse() {
  return new Response(
    JSON.stringify({
      data: [{ url: "https://cdn.example.com/generated.png" }],
    }),
    {
      status: 200,
      headers: { "Content-Type": "application/json" },
    },
  );
}

beforeEach(() => {
  setReactActEnvironment();

  localStorage.clear();
  vi.clearAllMocks();
  silenceConsole();
  mockGetNextApiKey.mockResolvedValue("test-api-key");
  mockInvoke.mockResolvedValue({ id: "material-1" });

  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(createSuccessResponse()) as unknown as typeof fetch,
  );
});

afterEach(() => {
  cleanupMountedRoots(mountedRoots);

  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  localStorage.clear();
});

describe("useImageGen 资源入库", () => {
  it("自动入库成功时应回写素材字段", async () => {
    const harness = mountHook();
    await waitForReady(harness);

    await act(async () => {
      await harness.getValue().generateImage("生成一张测试图", {
        targetProjectId: "project-1",
      });
    });

    const completed = harness
      .getValue()
      .images.find((image) => image.status === "complete");

    expect(completed).toBeDefined();
    expect(completed?.resourceMaterialId).toBe("material-1");
    expect(completed?.resourceProjectId).toBe("project-1");
    expect(typeof completed?.resourceSavedAt).toBe("number");
    expect(completed?.resourceSaveError).toBeUndefined();

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith(
      "import_material_from_url",
      expect.objectContaining({
        req: expect.objectContaining({
          projectId: "project-1",
          type: "image",
          url: "https://cdn.example.com/generated.png",
        }),
      }),
    );
  });

  it("自动入库失败时应保留图片并写入错误信息", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("resource save failed"));
    const harness = mountHook();
    await waitForReady(harness);

    await act(async () => {
      await harness.getValue().generateImage("生成一张失败回写图", {
        targetProjectId: "project-1",
      });
    });

    const completed = harness
      .getValue()
      .images.find((image) => image.status === "complete");

    expect(completed).toBeDefined();
    expect(completed?.resourceMaterialId).toBeUndefined();
    expect(completed?.resourceProjectId).toBeUndefined();
    expect(completed?.resourceSaveError).toBe("resource save failed");
  });
});
