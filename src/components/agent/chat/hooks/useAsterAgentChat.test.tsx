import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const {
  mockInitAsterAgent,
  mockSendAsterMessageStream,
  mockCreateAsterSession,
  mockListAsterSessions,
  mockGetAsterSession,
  mockRenameAsterSession,
  mockDeleteAsterSession,
  mockStopAsterSession,
  mockConfirmAsterAction,
  mockSubmitAsterElicitationResponse,
  mockParseStreamEvent,
  mockSafeListen,
  mockToast,
} = vi.hoisted(() => ({
  mockInitAsterAgent: vi.fn(),
  mockSendAsterMessageStream: vi.fn(),
  mockCreateAsterSession: vi.fn(),
  mockListAsterSessions: vi.fn(),
  mockGetAsterSession: vi.fn(),
  mockRenameAsterSession: vi.fn(),
  mockDeleteAsterSession: vi.fn(),
  mockStopAsterSession: vi.fn(),
  mockConfirmAsterAction: vi.fn(),
  mockSubmitAsterElicitationResponse: vi.fn(),
  mockParseStreamEvent: vi.fn((payload: unknown) => payload),
  mockSafeListen: vi.fn(),
  mockToast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("@/lib/api/agent", () => ({
  initAsterAgent: mockInitAsterAgent,
  sendAsterMessageStream: mockSendAsterMessageStream,
  createAsterSession: mockCreateAsterSession,
  listAsterSessions: mockListAsterSessions,
  getAsterSession: mockGetAsterSession,
  renameAsterSession: mockRenameAsterSession,
  deleteAsterSession: mockDeleteAsterSession,
  stopAsterSession: mockStopAsterSession,
  confirmAsterAction: mockConfirmAsterAction,
  submitAsterElicitationResponse: mockSubmitAsterElicitationResponse,
  parseStreamEvent: mockParseStreamEvent,
}));

vi.mock("@/lib/dev-bridge", () => ({
  safeListen: mockSafeListen,
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

import { useAsterAgentChat } from "./useAsterAgentChat";

interface HookHarness {
  getValue: () => ReturnType<typeof useAsterAgentChat>;
  unmount: () => void;
}

function mountHook(workspaceId = "ws-test"): HookHarness {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  let hookValue: ReturnType<typeof useAsterAgentChat> | null = null;

  function TestComponent() {
    hookValue = useAsterAgentChat({ workspaceId });
    return null;
  }

  act(() => {
    root.render(<TestComponent />);
  });

  return {
    getValue: () => {
      if (!hookValue) {
        throw new Error("hook 尚未初始化");
      }
      return hookValue;
    },
    unmount: () => {
      act(() => {
        root.unmount();
      });
      container.remove();
    },
  };
}

async function flushEffects() {
  await act(async () => {
    await Promise.resolve();
  });
}

function seedSession(workspaceId: string, sessionId: string) {
  sessionStorage.setItem(
    `aster_curr_sessionId_${workspaceId}`,
    JSON.stringify(sessionId),
  );
  sessionStorage.setItem(
    `aster_messages_${workspaceId}`,
    JSON.stringify([
      {
        id: "m-1",
        role: "assistant",
        content: "hello",
        timestamp: new Date().toISOString(),
      },
    ]),
  );
}

beforeEach(() => {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;

  vi.clearAllMocks();
  localStorage.clear();
  sessionStorage.clear();

  mockInitAsterAgent.mockResolvedValue(undefined);
  mockSendAsterMessageStream.mockResolvedValue(undefined);
  mockCreateAsterSession.mockResolvedValue("created-session");
  mockListAsterSessions.mockResolvedValue([]);
  mockGetAsterSession.mockResolvedValue({
    id: "session-from-api",
    messages: [],
  });
  mockRenameAsterSession.mockResolvedValue(undefined);
  mockDeleteAsterSession.mockResolvedValue(undefined);
  mockStopAsterSession.mockResolvedValue(undefined);
  mockConfirmAsterAction.mockResolvedValue(undefined);
  mockSubmitAsterElicitationResponse.mockResolvedValue(undefined);
  mockSafeListen.mockResolvedValue(() => {});
});

afterEach(() => {
  localStorage.clear();
  sessionStorage.clear();
});

describe("useAsterAgentChat.confirmAction", () => {
  it("tool_confirmation 应调用 confirmAsterAction", async () => {
    const workspaceId = "ws-tool";
    seedSession(workspaceId, "session-tool");
    const harness = mountHook(workspaceId);

    try {
      await flushEffects();
      await act(async () => {
        await harness.getValue().confirmAction({
          requestId: "req-tool-1",
          confirmed: true,
          response: "允许",
          actionType: "tool_confirmation",
        });
      });

      expect(mockConfirmAsterAction).toHaveBeenCalledTimes(1);
      expect(mockConfirmAsterAction).toHaveBeenCalledWith(
        "req-tool-1",
        true,
        "允许",
      );
      expect(mockSubmitAsterElicitationResponse).not.toHaveBeenCalled();
    } finally {
      harness.unmount();
    }
  });

  it("elicitation 应调用 submitAsterElicitationResponse 并透传 userData", async () => {
    const workspaceId = "ws-elicitation";
    seedSession(workspaceId, "session-elicitation");
    const harness = mountHook(workspaceId);

    try {
      await flushEffects();
      await act(async () => {
        await harness.getValue().confirmAction({
          requestId: "req-elicitation-1",
          confirmed: true,
          actionType: "elicitation",
          userData: { answer: "A" },
        });
      });

      expect(mockSubmitAsterElicitationResponse).toHaveBeenCalledTimes(1);
      expect(mockSubmitAsterElicitationResponse).toHaveBeenCalledWith(
        "session-elicitation",
        "req-elicitation-1",
        { answer: "A" },
      );
      expect(mockConfirmAsterAction).not.toHaveBeenCalled();
    } finally {
      harness.unmount();
    }
  });

  it("ask_user 应解析 response JSON 后提交", async () => {
    const workspaceId = "ws-ask-user";
    seedSession(workspaceId, "session-ask-user");
    const harness = mountHook(workspaceId);

    try {
      await flushEffects();
      await act(async () => {
        await harness.getValue().confirmAction({
          requestId: "req-ask-user-1",
          confirmed: true,
          actionType: "ask_user",
          response: '{"answer":"选项A"}',
        });
      });

      expect(mockSubmitAsterElicitationResponse).toHaveBeenCalledTimes(1);
      expect(mockSubmitAsterElicitationResponse).toHaveBeenCalledWith(
        "session-ask-user",
        "req-ask-user-1",
        { answer: "选项A" },
      );
    } finally {
      harness.unmount();
    }
  });
});

describe("useAsterAgentChat 偏好持久化", () => {
  it("应将旧全局偏好迁移到当前工作区", async () => {
    localStorage.setItem("agent_pref_provider", JSON.stringify("gemini"));
    localStorage.setItem("agent_pref_model", JSON.stringify("gemini-2.5-pro"));

    const workspaceId = "ws-migrate";
    const harness = mountHook(workspaceId);

    try {
      await flushEffects();

      const value = harness.getValue();
      expect(value.providerType).toBe("gemini");
      expect(value.model).toBe("gemini-2.5-pro");
      expect(
        JSON.parse(
          localStorage.getItem(`agent_pref_provider_${workspaceId}`) || "null",
        ),
      ).toBe("gemini");
      expect(
        JSON.parse(
          localStorage.getItem(`agent_pref_model_${workspaceId}`) || "null",
        ),
      ).toBe("gemini-2.5-pro");
      expect(
        JSON.parse(
          localStorage.getItem(`agent_pref_migrated_${workspaceId}`) ||
            "false",
        ),
      ).toBe(true);
    } finally {
      harness.unmount();
    }
  });

  it("应优先使用工作区偏好而不是旧全局偏好", async () => {
    localStorage.setItem("agent_pref_provider", JSON.stringify("claude"));
    localStorage.setItem("agent_pref_model", JSON.stringify("claude-legacy"));
    localStorage.setItem(
      "agent_pref_provider_ws-prefer-scoped",
      JSON.stringify("deepseek"),
    );
    localStorage.setItem(
      "agent_pref_model_ws-prefer-scoped",
      JSON.stringify("deepseek-reasoner"),
    );

    const harness = mountHook("ws-prefer-scoped");

    try {
      await flushEffects();

      const value = harness.getValue();
      expect(value.providerType).toBe("deepseek");
      expect(value.model).toBe("deepseek-reasoner");
    } finally {
      harness.unmount();
    }
  });

  it("无工作区时应保留全局模型偏好（切主题不丢失）", async () => {
    const firstMount = mountHook("");

    try {
      await flushEffects();
      act(() => {
        firstMount.getValue().setProviderType("gemini");
        firstMount.getValue().setModel("gemini-2.5-pro");
      });
      await flushEffects();
    } finally {
      firstMount.unmount();
    }

    const secondMount = mountHook("");
    try {
      await flushEffects();
      const value = secondMount.getValue();
      expect(value.providerType).toBe("gemini");
      expect(value.model).toBe("gemini-2.5-pro");
      expect(JSON.parse(localStorage.getItem("agent_pref_provider_global") || "null")).toBe(
        "gemini",
      );
      expect(JSON.parse(localStorage.getItem("agent_pref_model_global") || "null")).toBe(
        "gemini-2.5-pro",
      );
    } finally {
      secondMount.unmount();
    }
  });
});

describe("useAsterAgentChat 兼容接口", () => {
  it("triggerAIGuide 应仅生成 assistant 占位消息", async () => {
    const harness = mountHook("ws-guide");

    try {
      await flushEffects();
      await act(async () => {
        await harness.getValue().triggerAIGuide();
      });

      const value = harness.getValue();
      expect(value.messages).toHaveLength(1);
      expect(value.messages[0]?.role).toBe("assistant");
      expect(mockSendAsterMessageStream).toHaveBeenCalledTimes(1);
      expect(mockSendAsterMessageStream.mock.calls[0]?.[0]).toBe("");
    } finally {
      harness.unmount();
    }
  });

  it("renameTopic 应调用后端并刷新话题标题", async () => {
    const createdAt = Math.floor(Date.now() / 1000);
    mockListAsterSessions
      .mockResolvedValue([
        {
          id: "topic-1",
          name: "新标题",
          created_at: createdAt,
          messages_count: 2,
        },
      ])
      .mockResolvedValueOnce([
        {
          id: "topic-1",
          name: "旧标题",
          created_at: createdAt,
          messages_count: 2,
        },
      ]);

    const harness = mountHook("ws-rename");

    try {
      await flushEffects();
      await flushEffects();

      await act(async () => {
        await harness.getValue().renameTopic("topic-1", "新标题");
      });

      expect(mockRenameAsterSession).toHaveBeenCalledTimes(1);
      expect(mockRenameAsterSession).toHaveBeenCalledWith("topic-1", "新标题");

      const renamedTopic = harness
        .getValue()
        .topics.find((topic) => topic.id === "topic-1");
      expect(renamedTopic?.title).toBe("新标题");
    } finally {
      harness.unmount();
    }
  });

  it("deleteTopic 应调用后端并刷新话题列表", async () => {
    const createdAt = Math.floor(Date.now() / 1000);
    let currentSessions = [
      {
        id: "topic-1",
        name: "旧标题",
        created_at: createdAt,
        messages_count: 2,
      },
    ];

    mockListAsterSessions.mockImplementation(async () => currentSessions);
    mockDeleteAsterSession.mockImplementation(async () => {
      currentSessions = [];
    });

    const harness = mountHook("ws-delete");

    try {
      await flushEffects();
      await flushEffects();

      await act(async () => {
        await harness.getValue().deleteTopic("topic-1");
      });

      expect(mockDeleteAsterSession).toHaveBeenCalledTimes(1);
      expect(mockDeleteAsterSession).toHaveBeenCalledWith("topic-1");

      const deletedTopic = harness
        .getValue()
        .topics.find((topic) => topic.id === "topic-1");
      expect(deletedTopic).toBeUndefined();
    } finally {
      harness.unmount();
    }
  });
});
