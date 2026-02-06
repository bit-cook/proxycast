/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import svgr from "vite-plugin-svgr";
import path from "path";
import { fileURLToPath } from "url";

// ES 模块中获取 __dirname 的方式
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 获取 Tauri mock 目录路径
const tauriMockDir = path.resolve(__dirname, "./src/lib/tauri-mock");

export default defineConfig(({ mode }) => {
  // 检查是否在 Tauri 环境中运行（通过环境变量判断）
  const isTauri = process.env.TAURI_ENV_PLATFORM !== undefined;
  
  // 只在非 Tauri 环境（纯浏览器开发）下使用 mock
  const tauriAliases = isTauri ? {} : {
    "@tauri-apps/api/core": path.resolve(tauriMockDir, "core.ts"),
    "@tauri-apps/api/event": path.resolve(tauriMockDir, "event.ts"),
    "@tauri-apps/api/window": path.resolve(tauriMockDir, "window.ts"),
    "@tauri-apps/api/app": path.resolve(tauriMockDir, "window.ts"),
    "@tauri-apps/api/path": path.resolve(tauriMockDir, "window.ts"),
    "@tauri-apps/plugin-dialog": path.resolve(tauriMockDir, "plugin-dialog.ts"),
    "@tauri-apps/plugin-shell": path.resolve(tauriMockDir, "plugin-shell.ts"),
    "@tauri-apps/plugin-deep-link": path.resolve(tauriMockDir, "plugin-deep-link.ts"),
    "@tauri-apps/plugin-global-shortcut": path.resolve(tauriMockDir, "plugin-global-shortcut.ts"),
  };

  return {
  plugins: [
    react({
      jsxRuntime: mode === "development" ? "automatic" : "automatic",
      jsxImportSource: "react",
      babel: {
        compact: true,
      },
    }),
    svgr(),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      // 只在非 Tauri 环境下拦截 @tauri-apps/* 导入
      ...tauriAliases,
    },
  },
  optimizeDeps: {
    // 只在非 Tauri 环境下排除 Tauri 包的预构建
    exclude: isTauri ? [] : [
      "@tauri-apps/api",
      "@tauri-apps/plugin-dialog",
      "@tauri-apps/plugin-shell",
      "@tauri-apps/plugin-deep-link",
      "@tauri-apps/plugin-global-shortcut",
    ],
  },
  build: {
    chunkSizeWarningLimit: 12000,
    rollupOptions: {
      onwarn(warning, defaultHandler) {
        const isMixedImportWarning =
          warning.message.includes("dynamically imported by") &&
          warning.message.includes("also statically imported by");

        if (isMixedImportWarning) {
          return;
        }

        defaultHandler(warning);
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: false,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    globals: true,
    environment: "jsdom",
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "**/scripts/playwright-login/**",
      "**/src-tauri/**",
    ],
  },
};
});
