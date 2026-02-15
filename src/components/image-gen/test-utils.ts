import { act } from "react";
import type { ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { vi } from "vitest";

export interface MountedRoot {
  root: Root;
  container: HTMLDivElement;
}

export function setReactActEnvironment() {
  (
    globalThis as typeof globalThis & {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
}

export function renderIntoDom(
  element: ReactElement,
  mountedRoots: MountedRoot[],
): MountedRoot {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  act(() => {
    root.render(element);
  });

  const mounted = { root, container };
  mountedRoots.push(mounted);
  return mounted;
}

export function cleanupMountedRoots(mountedRoots: MountedRoot[]) {
  while (mountedRoots.length > 0) {
    const mounted = mountedRoots.pop();
    if (!mounted) break;
    act(() => {
      mounted.root.unmount();
    });
    mounted.container.remove();
  }
}

export async function flushEffects() {
  await act(async () => {
    await Promise.resolve();
  });
}

export async function waitForCondition(
  condition: () => boolean,
  timeout = 40,
  errorMessage = "等待条件超时",
): Promise<void> {
  for (let i = 0; i < timeout; i += 1) {
    if (condition()) {
      return;
    }
    await flushEffects();
  }
  throw new Error(errorMessage);
}

export function silenceConsole() {
  vi.spyOn(console, "log").mockImplementation(() => {});
  vi.spyOn(console, "warn").mockImplementation(() => {});
}
