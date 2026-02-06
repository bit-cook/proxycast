/**
 * @file 画布工具函数测试
 * @description 测试画布相关的工具函数
 * @module components/content-creator/canvas/canvasUtils.test
 */

import { describe, it, expect } from "vitest";
import {
  getCanvasTypeForTheme,
  isCanvasSupported,
  createInitialCanvasState,
} from "./canvasUtils";
import type { ThemeType } from "../types";

// ============================================================================
// getCanvasTypeForTheme 测试
// ============================================================================

describe("getCanvasTypeForTheme", () => {
  it("应该为每种主题返回正确的画布类型", () => {
    expect(getCanvasTypeForTheme("video")).toBe("script");
    expect(getCanvasTypeForTheme("novel")).toBe("novel");
    expect(getCanvasTypeForTheme("poster")).toBe("poster");
    expect(getCanvasTypeForTheme("music")).toBe("music");
    expect(getCanvasTypeForTheme("social-media")).toBe("document");
    expect(getCanvasTypeForTheme("document")).toBe("document");
    // 所有主题现在都支持 document 画布
    expect(getCanvasTypeForTheme("general")).toBe("document");
    expect(getCanvasTypeForTheme("knowledge")).toBe("document");
    expect(getCanvasTypeForTheme("planning")).toBe("document");
  });

  it("应该覆盖所有 ThemeType", () => {
    const allThemes: ThemeType[] = [
      "general",
      "social-media",
      "poster",
      "music",
      "knowledge",
      "planning",
      "document",
      "video",
      "novel",
    ];

    allThemes.forEach((theme) => {
      // 不应该抛出错误
      expect(() => getCanvasTypeForTheme(theme)).not.toThrow();
    });
  });
});

// ============================================================================
// isCanvasSupported 测试
// ============================================================================

describe("isCanvasSupported", () => {
  it("应该正确判断主题是否支持画布", () => {
    expect(isCanvasSupported("video")).toBe(true);
    expect(isCanvasSupported("novel")).toBe(true);
    expect(isCanvasSupported("poster")).toBe(true);
    expect(isCanvasSupported("music")).toBe(true);
    expect(isCanvasSupported("social-media")).toBe(true);
    expect(isCanvasSupported("document")).toBe(true);
    // 所有主题现在都支持画布
    expect(isCanvasSupported("general")).toBe(true);
    expect(isCanvasSupported("knowledge")).toBe(true);
    expect(isCanvasSupported("planning")).toBe(true);
  });

  it("所有 9 种主题都应该支持画布", () => {
    const allThemes: ThemeType[] = [
      "general",
      "social-media",
      "poster",
      "music",
      "knowledge",
      "planning",
      "document",
      "video",
      "novel",
    ];

    const supportedCount = allThemes.filter((theme) =>
      isCanvasSupported(theme),
    ).length;
    expect(supportedCount).toBe(9);
  });
});

// ============================================================================
// createInitialCanvasState 测试
// ============================================================================

describe("createInitialCanvasState", () => {
  it("应该为支持画布的主题创建初始状态", () => {
    const docState = createInitialCanvasState("document", "test content");
    expect(docState).not.toBeNull();
    expect(docState?.type).toBe("document");

    const novelState = createInitialCanvasState("novel", "test content");
    expect(novelState).not.toBeNull();
    expect(novelState?.type).toBe("novel");

    const scriptState = createInitialCanvasState("video", "test content");
    expect(scriptState).not.toBeNull();
    expect(scriptState?.type).toBe("script");

    const posterState = createInitialCanvasState("poster");
    expect(posterState).not.toBeNull();
    expect(posterState?.type).toBe("poster");

    const musicState = createInitialCanvasState("music");
    expect(musicState).not.toBeNull();
    expect(musicState?.type).toBe("music");

    const socialState = createInitialCanvasState("social-media", "test");
    expect(socialState).not.toBeNull();
    expect(socialState?.type).toBe("document");
  });

  it("所有主题都应该返回有效的画布状态", () => {
    // general、knowledge、planning 现在也支持 document 画布
    const generalState = createInitialCanvasState("general", "test");
    expect(generalState).not.toBeNull();
    expect(generalState?.type).toBe("document");

    const knowledgeState = createInitialCanvasState("knowledge", "test");
    expect(knowledgeState).not.toBeNull();
    expect(knowledgeState?.type).toBe("document");

    const planningState = createInitialCanvasState("planning", "test");
    expect(planningState).not.toBeNull();
    expect(planningState?.type).toBe("document");
  });

  it("应该正确处理空内容参数", () => {
    const docState = createInitialCanvasState("document");
    expect(docState).not.toBeNull();

    const novelState = createInitialCanvasState("novel");
    expect(novelState).not.toBeNull();
  });
});
