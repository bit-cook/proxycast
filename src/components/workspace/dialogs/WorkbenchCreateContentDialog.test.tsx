import { act, type ComponentProps } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkbenchCreateContentDialog } from "./WorkbenchCreateContentDialog";

interface RenderResult {
  container: HTMLDivElement;
  root: Root;
}

const mountedRoots: RenderResult[] = [];

function setInputValue(input: HTMLInputElement, value: string): void {
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLInputElement.prototype,
    "value",
  )?.set;
  setter?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function renderDialog(
  overrides: Partial<ComponentProps<typeof WorkbenchCreateContentDialog>> = {},
): RenderResult {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  act(() => {
    root.render(
      <WorkbenchCreateContentDialog
        open={true}
        creatingContent={false}
        step="mode"
        selectedProjectId="project-1"
        creationModeOptions={[
          { value: "guided", label: "引导模式", description: "分步骤提问" },
          { value: "fast", label: "快速模式", description: "快速起稿" },
        ]}
        selectedCreationMode="guided"
        onCreationModeChange={() => {}}
        currentCreationIntentFields={[
          {
            key: "topic",
            label: "创作主题",
            placeholder: "请输入主题",
          },
        ]}
        creationIntentValues={{
          topic: "",
          targetAudience: "",
          goal: "",
          constraints: "",
          contentType: "",
          length: "",
          corePoints: "",
          tone: "",
          outline: "",
          mustInclude: "",
          extraRequirements: "",
        }}
        onCreationIntentValueChange={() => {}}
        currentIntentLength={0}
        minCreationIntentLength={10}
        creationIntentError=""
        onOpenChange={() => {}}
        onBackOrCancel={() => {}}
        onGoToIntentStep={() => {}}
        onCreateContent={() => {}}
        {...overrides}
      />,
    );
  });

  const rendered = { container, root };
  mountedRoots.push(rendered);
  return rendered;
}

beforeEach(() => {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
});

afterEach(() => {
  while (mountedRoots.length > 0) {
    const mounted = mountedRoots.pop();
    if (!mounted) {
      break;
    }
    act(() => {
      mounted.root.unmount();
    });
    mounted.container.remove();
  }
});

describe("WorkbenchCreateContentDialog", () => {
  it("模式步骤支持切换创作模式并进入下一步", () => {
    const onCreationModeChange = vi.fn();
    const onGoToIntentStep = vi.fn();
    renderDialog({ onCreationModeChange, onGoToIntentStep });

    expect(document.body.textContent).toContain("步骤 1/2");
    expect(document.body.textContent).toContain("引导模式");

    const fastModeButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("快速模式"),
    );
    const nextButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "下一步",
    );
    expect(fastModeButton).toBeDefined();
    expect(nextButton).toBeDefined();

    act(() => {
      fastModeButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    act(() => {
      nextButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onCreationModeChange).toHaveBeenCalledWith("fast");
    expect(onGoToIntentStep).toHaveBeenCalledTimes(1);
  });

  it("意图步骤长度不足时禁用创建按钮", () => {
    renderDialog({
      step: "intent",
      currentIntentLength: 6,
      minCreationIntentLength: 10,
      creationIntentError: "创作意图至少需要 10 个字",
    });

    expect(document.body.textContent).toContain("步骤 2/2");
    expect(document.body.textContent).toContain("创作意图字数：6/10");
    expect(document.body.textContent).toContain("创作意图至少需要 10 个字");

    const createButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "创建并进入作业",
    );
    expect(createButton).toBeDefined();
    expect(createButton).toHaveProperty("disabled", true);
  });

  it("意图步骤支持编辑输入并触发上一步与创建", () => {
    const onCreationIntentValueChange = vi.fn();
    const onBackOrCancel = vi.fn();
    const onCreateContent = vi.fn();
    renderDialog({
      step: "intent",
      currentIntentLength: 16,
      onCreationIntentValueChange,
      onBackOrCancel,
      onCreateContent,
    });

    const topicInput = document.body.querySelector(
      "input#creation-intent-topic",
    ) as HTMLInputElement | null;
    expect(topicInput).not.toBeNull();

    act(() => {
      if (!topicInput) {
        return;
      }
      setInputValue(topicInput, "新的主题");
    });

    const backButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "上一步",
    );
    const createButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "创建并进入作业",
    );
    expect(backButton).toBeDefined();
    expect(createButton).toBeDefined();

    act(() => {
      backButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    act(() => {
      createButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onCreationIntentValueChange).toHaveBeenCalledWith("topic", "新的主题");
    expect(onBackOrCancel).toHaveBeenCalledTimes(1);
    expect(onCreateContent).toHaveBeenCalledTimes(1);
  });
});
