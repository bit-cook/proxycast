# Windows vs macOS 平台差异分析报告

## 问题背景
用户报告在 Windows 11 上发送第一条消息时崩溃，而 macOS 开发环境正常工作。

## Context7 MCP 文档分析结果

### 1. Tauri 平台差异

**渲染引擎差异**：
- **Windows**: 使用 Chromium
- **macOS/Linux**: 使用 WebKit

**重要发现**: Tauri 文档明确指出需要根据平台设置不同的构建目标：
```javascript
// Windows
chrome105  // 用于 Windows (Chromium)

// macOS/Linux
safari13   // 用于 macOS 和 Linux (WebKit)
```

### 2. Tokio Runtime 平台差异

**关键问题**: `Runtime::new()` 在不同平台上的行为可能不同

从 Context7 文档中发现：
- Tokio 在不同平台上使用不同的 I/O 驱动
- Linux 使用 `io-uring`（可选）
- macOS 使用 `kqueue`
- Windows 使用 `IOCP` (I/O Completion Ports)

**Windows 特定风险**：
```rust
// 我们的代码 (bootstrap.rs:147)
let rt = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
    tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime: 系统资源不足或配置错误")
        .handle()
        .clone()
});
```

**潜在问题**：
1. Windows 上的线程池创建可能更严格
2. Windows 上的 IOCP 初始化可能失败
3. Windows 上的栈大小默认值不同

### 3. Rust 平台特定代码

**条件编译示例**：
```rust
#[cfg(target_os = "windows")]
pub struct WindowsToken;

#[cfg(target_os = "macos")]
pub struct MacosToken;
```

**我们的代码检查结果**：
- ✅ 已正确使用 `#[cfg(target_os = "windows")]` 进行平台特定代码隔离
- ✅ 配置文件路径处理已正确处理 Windows 路径

## aster-rust 版本分析

### 当前使用的版本
```toml
aster = { package = "aster-core", git = "https://github.com/astercloud/aster-rust", tag = "v0.13.0" }
aster-models = { git = "https://github.com/astercloud/aster-rust", tag = "v0.13.0" }
```

### 版本历史
- **v0.13.0** (2025-02-18): ✅ 当前使用 - 最新版本
  - Commit: `4422f761`
  - 包含修复: "fix clippy warnings, fmt, bump version"

- **v0.12.0** (2025-02-16): 上一版本
  - 主要更新: "feat: add observability, supervisor, heartbeat"

**结论**: ✅ **aster-rust 版本是最新的，不需要更新**

## Windows 特定崩溃点分析

### 高风险点

#### 1. Tokio Runtime 创建 (bootstrap.rs:147)
```rust
tokio::runtime::Runtime::new()
    .expect("Failed to create tokio runtime: 系统资源不足或配置错误")
```

**Windows 风险**：
- 线程池创建可能失败
- IOCP 端口创建可能失败
- 栈内存分配可能更严格

**建议修复**：
```rust
tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)  // 限制线程数
    .thread_name("proxycast-runtime")
    .enable_io()
    .enable_time()
    .build()
    .expect("Failed to create tokio runtime")
```

#### 2. 数据库连接 (可能的问题)
```rust
let db = database::init_database()
    .map_err(|e| format!("数据库初始化失败: {e}"))?;
```

**Windows 风险**：
- SQLite 在 Windows 上的文件锁行为不同
- 路径长度限制 (MAX_PATH = 260 字符)
- 权限问题更严格

#### 3. 文件系统操作
**Windows 特定限制**：
- 路径分隔符: `\` vs `/`
- 文件名大小写不敏感
- 路径长度限制
- 文件锁更严格

### 中风险点

#### 4. 加密模块初始化
虽然加密模块只在测试中使用，但 Windows 上的加密 API 可能不同。

#### 5. MCP 服务器启动
Windows 上的进程创建和 socket 行为可能不同。

## 建议的修复方案

### 立即修复

#### 1. 改进 Tokio Runtime 创建
```rust
// bootstrap.rs:147
let rt = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
    // 使用 Builder 模式获得更多控制
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("proxycast-runtime-{}", id)
        })
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to create tokio runtime: please check system resources and permissions")
        .handle()
        .clone()
});
```

#### 2. 添加 Windows 特定日志
```rust
#[cfg(target_os = "windows")]
tracing::info!("[Bootstrap] Windows 平台 - 检查 IOCP 和线程池配置");

#[cfg(target_os = "macos")]
tracing::info!("[Bootstrap] macOS 平台 - 检查 kqueue 配置");
```

#### 3. 添加数据库初始化重试
```rust
let db = database::init_database()
    .map_err(|e| format!("数据库初始化失败: {e}"))?;

// Windows 特定：验证数据库可写性
#[cfg(target_os = "windows")]
{
    use crate::database::dao;
    let conn = db.lock().unwrap();
    if let Err(e) = dao::test_connection(&conn) {
        tracing::error!("[Bootstrap] Windows 数据库连接测试失败: {}", e);
    }
}
```

### 长期改进

1. **添加平台特定的集成测试**
2. **在 CI/CD 中添加 Windows 构建**
3. **添加 Windows 事件查看器日志支持**
4. **添加更详细的错误上下文**

## 测试清单

### Windows 特定测试
- [ ] 在 Windows 11 上启动应用
- [ ] 检查事件查看器 (Event Viewer) 中的应用日志
- [ ] 验证数据库文件创建位置
- [ ] 测试长路径支持
- [ ] 测试中文字符路径
- [ ] 验证防火墙权限

### 建议的 Windows 调试命令
```powershell
# 启用详细日志
$env:RUST_LOG=debug
$env:RUST_BACKTRACE=1
.\proxycast.exe

# 检查事件日志
Get-EventLog -LogName Application -Source "ProxyCast" -Newest 50
```

## 结论

### 主要发现
1. ✅ **aster-rust 版本是最新的** - 不需要更新
2. ⚠️ **Tokio Runtime 创建可能在 Windows 上失败** - 需要改进
3. ⚠️ **缺少 Windows 特定的错误处理** - 需要添加
4. ⚠️ **Windows 平台测试不足** - 需要加强

### 下一步行动
1. 实施上述建议的修复方案
2. 在 Windows 11 上测试
3. 添加 Windows CI/CD
4. 收集 Windows 用户的详细错误日志

## 参考资源
- [Tauri Windows 文档](https://tauri.app/v1/guides/building/windows)
- [Tokio Runtime 文档](https://tokio.rs/tokio/topics/runtime)
- [Rust Windows 平台支持](https://doc.rust-lang.org/rustc/platform-support/windows-pc-gnu-msvc.html)
