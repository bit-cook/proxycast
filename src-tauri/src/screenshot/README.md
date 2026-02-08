# 截图对话模块 (screenshot/)

截图对话功能的 Rust 后端实现，提供全局快捷键、屏幕截图和悬浮窗口管理。

## 模块结构

```
screenshot/
├── mod.rs          # 模块入口，导出公共接口和初始化函数
├── capture.rs      # 截图桥接层（纯逻辑在 crates/services/src/screenshot_capture_service.rs）
├── config.rs       # 实验室功能配置管理
├── shortcut.rs     # 全局快捷键注册/注销/更新
├── window.rs       # 悬浮对话窗口管理
└── README.md       # 本文档
```

## 核心功能

### 1. 模块初始化 (mod.rs)

- `init(app)`: 初始化截图对话模块，根据配置注册快捷键
- `cleanup(app)`: 清理模块资源，注销快捷键

### 2. 配置管理 (config.rs)

- `load_experimental_config(app)`: 加载实验室功能配置
- `is_screenshot_chat_enabled(config)`: 检查功能是否启用
- `save_experimental_config(app, config)`: 保存配置到文件

### 3. 快捷键管理 (shortcut.rs)

- `register(app, shortcut)`: 注册全局快捷键
- `unregister(app)`: 注销当前快捷键
- `update(app, new_shortcut)`: 原子性更新快捷键
- `validate(shortcut)`: 验证快捷键格式

### 4. 截图服务 (capture.rs)

- `start_capture()`: 启动交互式截图
- 支持 macOS (`screencapture`)、Windows (PowerShell)、Linux (`gnome-screenshot` / `scrot`)

### 5. 窗口管理 (window.rs)

- `open_floating_window(app, image_path)`: 打开悬浮对话窗口
- `close_floating_window(app)`: 关闭悬浮窗口
- `is_floating_window_open(app)`: 检查窗口状态

## 依赖

- `tauri-plugin-global-shortcut`: 全局快捷键支持
- `chrono`: 时间戳生成
- `urlencoding`: URL 参数编码

## 配置示例

```yaml
experimental:
  screenshot_chat:
    enabled: true
    shortcut: "CommandOrControl+Shift+S"
```

## 相关需求

- 需求 1: 实验室功能开关
- 需求 2: 快捷键配置
- 需求 3: 屏幕截图
- 需求 4: 悬浮对话窗口
- 需求 7: 模块化代码组织
