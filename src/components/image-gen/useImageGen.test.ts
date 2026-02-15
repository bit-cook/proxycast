/**
 * @file useImageGen Fal 调用测试
 * @description 验证 Fal 图片生成关键回退链路
 * @module components/image-gen/useImageGen.test
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { __imageGenFalTestUtils } from "./useImageGen";
import { silenceConsole } from "./test-utils";

const { requestImageFromFal, resolveFalEndpointModelCandidates } =
  __imageGenFalTestUtils;

function createJsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function createTextResponse(body: string, status = 200): Response {
  return new Response(body, { status });
}

describe("useImageGen Fal 调用链路", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
    silenceConsole();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("同步接口成功时应直接返回图片", async () => {
    fetchMock.mockResolvedValueOnce(
      createJsonResponse({
        images: [{ url: "https://cdn.example.com/sync-ok.png" }],
      }),
    );

    const imageUrl = await requestImageFromFal(
      "https://fal.run",
      "test-fal-key",
      "fal-ai/nano-banana-pro",
      "a red apple",
      [],
      "1024x1024",
    );

    expect(imageUrl).toBe("https://cdn.example.com/sync-ok.png");
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "https://fal.run/fal-ai/nano-banana-pro",
    );

    const requestInit = fetchMock.mock.calls[0]?.[1] as {
      method?: string;
      headers?: Record<string, string>;
    };
    expect(requestInit?.method).toBe("POST");
    expect(requestInit?.headers).toMatchObject({
      Authorization: "Key test-fal-key",
    });
  });

  it("同步失败后应回退到队列并返回结果", async () => {
    fetchMock
      .mockResolvedValueOnce(createTextResponse("sync primary failed", 500))
      .mockResolvedValueOnce(createTextResponse("sync compact failed", 500))
      .mockResolvedValueOnce(createJsonResponse({ request_id: "req-1" }, 200))
      .mockResolvedValueOnce(
        createJsonResponse({
          status: "COMPLETED",
          response_url: "https://queue.fal.run/fal-ai/nano-banana-pro/requests/req-1",
        }),
      )
      .mockResolvedValueOnce(
        createJsonResponse({
          images: [{ url: "https://cdn.example.com/queue-ok.png" }],
        }),
      );

    const imageUrl = await requestImageFromFal(
      "https://fal.run",
      "test-fal-key",
      "fal-ai/nano-banana-pro",
      "a robot cat",
      [],
      "1024x1024",
    );

    expect(imageUrl).toBe("https://cdn.example.com/queue-ok.png");
    expect(fetchMock.mock.calls[2]?.[0]).toBe(
      "https://queue.fal.run/fal-ai/nano-banana-pro",
    );
    expect(fetchMock.mock.calls[3]?.[0]).toBe(
      "https://queue.fal.run/fal-ai/nano-banana-pro/requests/req-1/status",
    );
    expect(fetchMock.mock.calls[4]?.[0]).toBe(
      "https://queue.fal.run/fal-ai/nano-banana-pro/requests/req-1",
    );
  });

  it("带参考图时应先尝试 /edit，再回退基础端点", async () => {
    const endpointCandidates = resolveFalEndpointModelCandidates(
      "fal-ai/nano-banana-pro",
      true,
    );
    expect(endpointCandidates).toEqual([
      "fal-ai/nano-banana-pro/edit",
      "fal-ai/nano-banana-pro",
    ]);

    fetchMock
      .mockResolvedValueOnce(createTextResponse("edit primary failed", 404))
      .mockResolvedValueOnce(createTextResponse("edit compact failed", 404))
      .mockResolvedValueOnce(createTextResponse("edit queue failed", 500))
      .mockResolvedValueOnce(
        createJsonResponse({
          images: [{ url: "https://cdn.example.com/base-fallback.png" }],
        }),
      );

    const imageUrl = await requestImageFromFal(
      "https://fal.run",
      "test-fal-key",
      "fal-ai/nano-banana-pro",
      "edit this image",
      ["https://cdn.example.com/reference.png"],
      "1024x1024",
    );

    expect(imageUrl).toBe("https://cdn.example.com/base-fallback.png");
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "https://fal.run/fal-ai/nano-banana-pro/edit",
    );
    expect(fetchMock.mock.calls[3]?.[0]).toBe(
      "https://fal.run/fal-ai/nano-banana-pro",
    );
  });
});
