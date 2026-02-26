import React from "react";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelSelectorProps } from "@/components/input-kit";

const { mockModelSelector } = vi.hoisted(() => ({
  mockModelSelector: vi.fn<(props: ModelSelectorProps) => React.ReactNode>(),
}));

vi.mock("@/components/input-kit", () => ({
  ModelSelector: (props: ModelSelectorProps) => mockModelSelector(props),
}));

import { CompactModelSelector } from "./CompactModelSelector";

interface RenderResult {
  container: HTMLDivElement;
  root: Root;
}

const mountedRoots: Array<{ root: Root; container: HTMLDivElement }> = [];

function renderCompactModelSelector(
  props: Partial<React.ComponentProps<typeof CompactModelSelector>> = {},
): RenderResult {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  const mergedProps: React.ComponentProps<typeof CompactModelSelector> = {
    providerType: "gemini",
    model: "gemini-2.5-pro",
    setProviderType: vi.fn(),
    setModel: vi.fn(),
    hasAvailableProvider: true,
    ...props,
  };

  act(() => {
    root.render(<CompactModelSelector {...mergedProps} />);
  });

  mountedRoots.push({ root, container });
  return { container, root };
}

function findByTestId(container: HTMLElement, testId: string): HTMLElement {
  const node = container.querySelector(`[data-testid="${testId}"]`);
  if (!node) {
    throw new Error(`未找到节点: ${testId}`);
  }
  return node as HTMLElement;
}

beforeEach(() => {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;

  vi.clearAllMocks();

  mockModelSelector.mockImplementation((props) => (
    <button
      type="button"
      data-testid="mock-model-selector"
      onClick={() => {
        props.setProviderType("deepseek");
        props.setModel("deepseek-chat");
      }}
    >
      {props.providerType}/{props.model}
    </button>
  ));
});

afterEach(() => {
  while (mountedRoots.length > 0) {
    const mounted = mountedRoots.pop();
    if (!mounted) break;
    act(() => {
      mounted.root.unmount();
    });
    mounted.container.remove();
  }
  vi.clearAllMocks();
});

describe("CompactModelSelector", () => {
  it("应透传受控状态与回调到 ModelSelector", () => {
    const setProviderType = vi.fn<(providerType: string) => void>();
    const setModel = vi.fn<(model: string) => void>();
    const { container } = renderCompactModelSelector({
      providerType: "gemini",
      model: "gemini-2.5-pro",
      setProviderType,
      setModel,
      hasAvailableProvider: true,
    });

    expect(mockModelSelector).toHaveBeenCalledTimes(1);
    const props = mockModelSelector.mock.calls[0][0];
    expect(props.providerType).toBe("gemini");
    expect(props.model).toBe("gemini-2.5-pro");
    expect(props.compactTrigger).toBe(true);
    expect(props.popoverSide).toBe("top");
    expect(props.disabled).toBe(false);

    act(() => {
      findByTestId(container, "mock-model-selector").click();
    });

    expect(setProviderType).toHaveBeenCalledWith("deepseek");
    expect(setModel).toHaveBeenCalledWith("deepseek-chat");
  });

  it("加载中时应禁用 ModelSelector", () => {
    renderCompactModelSelector({
      hasAvailableProvider: true,
      disabled: false,
      isLoading: true,
    });

    expect(mockModelSelector).toHaveBeenCalledTimes(1);
    const props = mockModelSelector.mock.calls[0][0];
    expect(props.disabled).toBe(true);
  });

  it("无可用 Provider 且未加载时应显示引导并不渲染 ModelSelector", () => {
    const { container } = renderCompactModelSelector({
      hasAvailableProvider: false,
      isLoading: false,
    });

    expect(container.textContent).toContain("工具模型未配置");
    expect(mockModelSelector).not.toHaveBeenCalled();
  });

  it("无可用 Provider 时点击配置应触发回调", () => {
    const onManageProviders = vi.fn();
    const { container } = renderCompactModelSelector({
      hasAvailableProvider: false,
      isLoading: false,
      onManageProviders,
    });

    const button = Array.from(container.querySelectorAll("button")).find(
      (node) => node.textContent?.includes("配置"),
    ) as HTMLButtonElement | undefined;

    if (!button) {
      throw new Error("未找到配置按钮");
    }

    act(() => {
      button.click();
    });

    expect(onManageProviders).toHaveBeenCalledTimes(1);
  });

  it("错误状态应展示异常提示文案", () => {
    const { container } = renderCompactModelSelector({
      hasAvailableProvider: true,
      error: "network_error",
    });

    expect(container.textContent).toContain("模型加载异常");
  });
});
