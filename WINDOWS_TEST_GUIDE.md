# Windows 11 测试指南

## 修复说明

本次修复针对 Windows 平台的兼容性问题进行了以下改进：

### 1. 改进 Tokio Runtime 创建
- 使用 `Builder` 模式替代 `Runtime::new()`
- 限制工作线程数为 2（避免 Windows 资源问题）
- 添加平台特定的日志输出
- 提高跨平台兼容性

### 2. 添加 Windows 数据库验证
- 在启动时验证数据库文件权限
- 添加 Windows 特定的诊断日志

## Windows 11 测试步骤

### 准备工作

1. **安装最新代码**
   ```powershell
   git pull origin main
   git log --oneline -1
   # 应该看到: fix: 改进 Windows 平台兼容性
   ```

2. **启用详细日志**
   ```powershell
   # 设置环境变量
   $env:RUST_LOG=debug
   $env:RUST_BACKTRACE=1

   # 或者永久设置（管理员权限）
   [System.Environment]::SetEnvironmentVariable("RUST_LOG", "debug", "User")
   [System.Environment]::SetEnvironmentVariable("RUST_BACKTRACE", "1", "User")
   ```

### 测试流程

#### 测试 1: 启动测试
1. 双击启动 `ProxyCast.exe`
2. 查看控制台输出，应该看到：
   ```
   [INFO] [Bootstrap] Windows 平台 - 创建 Tokio Runtime (IOCP)
   [INFO] [Bootstrap] Windows 平台 - 验证数据库文件权限
   [INFO] [Bootstrap] Windows 数据库验证成功
   ```
3. 应用应该正常启动

#### 测试 2: 发送消息测试
1. 创建新对话
2. 发送第一条消息："你好"
3. **预期结果**：
   - ✅ 消息成功发送
   - ✅ 收到 AI 回复
   - ✅ 不会崩溃

#### 测试 3: 查看详细日志
如果仍然崩溃，请：
1. 打开 PowerShell
2. 运行：
   ```powershell
   $env:RUST_LOG=debug; $env:RUST_BACKTRACE=1; .\ProxyCast.exe
   ```
3. 复制所有输出

#### 测试 4: 检查事件查看器
1. 按 `Win + X`，选择"事件查看器"
2. 导航到：Windows 日志 → 应用程序
3. 查找来源为 "ProxyCast" 的错误事件
4. 导出日志（右键 → "将所有事件另存为..."）

## 常见问题排查

### 问题 1: 仍然崩溃
**请收集以下信息**：
```powershell
# 1. 系统信息
systeminfo | Select-String /C:"OS Name" /C:"OS Version"

# 2. Rust 版本
rustc --version

# 3. Cargo 版本
cargo --version

# 4. 运行应用（带详细日志）
$env:RUST_LOG=trace; .\ProxyCast.exe > proxycast.log 2>&1

# 5. 检查日志文件
Get-Content proxycast.log | Select-String -Pattern "ERROR|WARN|Bootstrap"
```

### 问题 2: 数据库错误
**症状**：启动时提示"数据库初始化失败"

**解决方案**：
```powershell
# 1. 删除现有数据库（会丢失数据，谨慎操作）
Remove-Item "$env:APPDATA\proxycast\*.db" -Force

# 2. 重新启动应用
.\ProxyCast.exe
```

### 问题 3: 权限错误
**症状**：提示"访问被拒绝"

**解决方案**：
```powershell
# 以管理员身份运行
# 右键 ProxyCast.exe → "以管理员身份运行"

# 或者修改文件夹权限
icacls "$env:APPDATA\proxycast" /grant "$($env:USERNAME):(OI)(CI)F" /T
```

## 预期行为

### 成功启动的日志示例
```
[INFO] [Bootstrap] Windows 平台 - 创建 Tokio Runtime (IOCP)
[INFO] [Bootstrap] Windows 平台 - 验证数据库文件权限
[INFO] [Bootstrap] Windows 数据库验证成功
[INFO] [启动] 插件安装器初始化成功
[INFO] [Bootstrap] 已设置 Aster 全局 session store
```

### 成功发送消息的日志示例
```
[INFO] [AsterAgent] 发送流式消息: session=xxx, event=xxx
[INFO] [AsterAgent] Agent 初始化状态: true
[INFO] [AsterAgent] 收到 provider_config: provider_name=xxx, model_name=xxx
```

## 性能对比

### macOS vs Windows

| 操作 | macOS | Windows |
|------|-------|---------|
| 渲染引擎 | WebKit | Chromium |
| I/O 模型 | kqueue | IOCP |
| 线程数 | 自动 (CPU核心数) | 限制为 2 |
| 文件锁 | POSIX | Windows 锁 |
| 路径格式 | `/` | `\` |

## 联系方式

如果测试后仍有问题，请提供：
1. 完整的启动日志（`$env:RUST_LOG=trace`）
2. 事件查看器中的错误日志
3. 系统信息（`systeminfo`）
4. 重现步骤的详细描述

## 相关文档
- [完整分析报告](./WINDOWS_CRASH_ANALYSIS.md)
- [修复验证清单](./test-crash-fix.md)
