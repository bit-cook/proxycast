import { act, createElement, type ComponentType } from "react";
import { createRoot, type Root } from "react-dom/client";

export interface MountedRoot {
  container: HTMLDivElement;
  root: Root;
}

export interface MountedRenderResult<TProps> extends MountedRoot {
  rerender: (props: TProps) => void;
}

export function setupReactActEnvironment(): void {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
}

export function mountHarness<TProps>(
  Component: ComponentType<TProps>,
  initialProps: TProps,
  mountedRoots: MountedRoot[],
): MountedRenderResult<TProps> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  const rerender = (props: TProps) => {
    act(() => {
      root.render(
        createElement(
          Component as ComponentType<Record<string, unknown>>,
          props as Record<string, unknown>,
        ),
      );
    });
  };

  rerender(initialProps);
  mountedRoots.push({ container, root });

  return {
    container,
    root,
    rerender,
  };
}

export function cleanupMountedRoots(mountedRoots: MountedRoot[]): void {
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
}

export function clickElement(element: Element | null): void {
  act(() => {
    element?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

type QueryScope = {
  querySelectorAll: (selectors: string) => ArrayLike<Element>;
};

export function findButtonByText(
  scope: QueryScope,
  text: string,
  options?: {
    exact?: boolean;
  },
): HTMLButtonElement | undefined {
  const exact = options?.exact ?? false;
  return Array.from(scope.querySelectorAll("button")).find((button) => {
    const content = button.textContent?.trim() ?? "";
    return exact ? content === text : content.includes(text);
  }) as HTMLButtonElement | undefined;
}

export function clickButtonByText(
  scope: QueryScope,
  text: string,
  options?: {
    exact?: boolean;
  },
): HTMLButtonElement | undefined {
  const button = findButtonByText(scope, text, options);
  clickElement(button ?? null);
  return button;
}

export function clickByTestId(
  container: HTMLElement,
  testId: string,
): HTMLButtonElement | null {
  const button = container.querySelector(
    `button[data-testid='${testId}']`,
  ) as HTMLButtonElement | null;
  clickElement(button);
  return button;
}

export function getRootElement(container: HTMLElement): HTMLElement | null {
  return container.firstElementChild as HTMLElement | null;
}

export function findInputByPlaceholder(
  scope: QueryScope,
  placeholder: string,
): HTMLInputElement | HTMLTextAreaElement | null {
  const field = Array.from(scope.querySelectorAll("input,textarea")).find(
    (element) =>
      element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement
        ? element.placeholder === placeholder
        : false,
  );
  if (
    field instanceof HTMLInputElement ||
    field instanceof HTMLTextAreaElement
  ) {
    return field;
  }
  return null;
}

export function findInputById(
  scope: {
    querySelector: (selectors: string) => Element | null;
  },
  id: string,
): HTMLInputElement | HTMLTextAreaElement | null {
  const element = scope.querySelector(`#${id}`);
  if (
    element instanceof HTMLInputElement ||
    element instanceof HTMLTextAreaElement
  ) {
    return element;
  }
  return null;
}

export function findButtonByTitle(
  scope: {
    querySelector: (selectors: string) => Element | null;
  },
  title: string,
): HTMLButtonElement | null {
  const element = scope.querySelector(`button[title='${title}']`);
  if (element instanceof HTMLButtonElement) {
    return element;
  }
  return null;
}

export function clickButtonByTitle(
  scope: {
    querySelector: (selectors: string) => Element | null;
  },
  title: string,
): HTMLButtonElement | null {
  const button = findButtonByTitle(scope, title);
  clickElement(button);
  return button;
}

export function findAsideByClassFragment(
  scope: QueryScope,
  classFragment: string,
): HTMLElement | null {
  const matched = Array.from(scope.querySelectorAll("aside")).find((aside) =>
    aside.className.includes(classFragment),
  );
  if (matched instanceof HTMLElement) {
    return matched;
  }
  return null;
}

export function setTextInputValue(
  element: HTMLInputElement | HTMLTextAreaElement,
  value: string,
): void {
  const prototype =
    element instanceof HTMLTextAreaElement
      ? window.HTMLTextAreaElement.prototype
      : window.HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

export function fillTextInput(
  element: HTMLInputElement | HTMLTextAreaElement | null,
  value: string,
): void {
  act(() => {
    if (!element) {
      return;
    }
    setTextInputValue(element, value);
  });
}

export function triggerKeyboardShortcut(
  target: EventTarget,
  key: string,
  options?: {
    ctrlKey?: boolean;
    metaKey?: boolean;
    altKey?: boolean;
    shiftKey?: boolean;
    type?: "keydown" | "keyup";
    bubbles?: boolean;
  },
): void {
  const {
    type = "keydown",
    bubbles = true,
    ctrlKey,
    metaKey,
    altKey,
    shiftKey,
  } = options ?? {};
  act(() => {
    target.dispatchEvent(
      new KeyboardEvent(type, {
        key,
        bubbles,
        ctrlKey,
        metaKey,
        altKey,
        shiftKey,
      }),
    );
  });
}

export async function flushEffects(times = 4): Promise<void> {
  for (let i = 0; i < times; i += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}
