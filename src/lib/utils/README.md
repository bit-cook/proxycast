# 工具函数库

本目录包含前端通用工具函数。

## 文件索引

| 文件 | 说明 |
|------|------|
| `apiKeyMask.ts` | API Key 脱敏工具，用于安全显示 API Key |
| `apiKeyMask.test.ts` | API Key 脱敏属性测试 |
| `apiKeyValidation.ts` | API Key 格式验证，各 Provider 的验证规则 |
| `apiKeyValidation.test.ts` | API Key 格式验证属性测试 |
| `connectError.ts` | Connect 错误处理工具函数，Toast 通知 |
| `syntaxHighlight.ts` | 代码语法高亮工具 |

## 主要功能

### API Key 脱敏 (`apiKeyMask.ts`)

对 API Key 进行脱敏处理，保护敏感信息：
- 长度 > 8：显示前 4 位 + "..." + 后 4 位
- 长度 <= 8：返回 "****"

### API Key 验证 (`apiKeyValidation.ts`)

验证各 Provider 的 API Key 格式：
- 前缀验证（如 OpenAI 的 `sk-`）
- 长度验证
- 字符验证

### Connect 错误处理 (`connectError.ts`)

ProxyCast Connect 功能的错误提示 Toast 通知：
- `showDeepLinkError` - Deep Link 解析错误提示
- `showRegistryLoadError` - Registry 加载失败提示（已回退到缓存）
- `showRegistryNoCacheError` - Registry 不可用错误（无缓存）
- `showApiKeySaveError` - API Key 保存失败提示
