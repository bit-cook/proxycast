# voice/ - 语音输入模块

语音输入功能的 Tauri 后端模块，提供全局快捷键、悬浮窗、ASR 识别、LLM 润色等功能。

## 文件索引

| 文件 | 说明 |
|------|------|
| `mod.rs` | 模块入口，导出子模块 |
| `asr_service.rs` | ASR 桥接层（纯逻辑已迁移到 `crates/services/src/voice_asr_service.rs`） |
| `commands.rs` | Tauri 命令，供前端调用 |
| `config.rs` | 配置桥接层（纯逻辑已迁移到 `crates/services/src/voice_config_service.rs`） |
| `output_service.rs` | 输出桥接层（纯逻辑已迁移到 `crates/services/src/voice_output_service.rs`） |
| `processor.rs` | 润色桥接层（纯逻辑已迁移到 `crates/services/src/voice_processor_service.rs`） |
| `recording_service.rs` | 录音桥接层（纯逻辑已迁移到 `crates/services/src/voice_recording_service.rs`） |
| `shortcut.rs` | 全局快捷键管理 |
| `window.rs` | 悬浮窗管理 |

## 录音服务架构

由于 `cpal::Stream` 不实现 `Send` trait，无法直接在 Tauri 的 async 命令中使用。
录音服务采用**独立线程 + channel 通信**的方案：

```
┌─────────────────┐     Command      ┌─────────────────┐
│  Tauri Command  │ ───────────────> │  Recording      │
│  (async)        │                  │  Thread         │
│                 │ <─────────────── │  (owns Stream)  │
└─────────────────┘     Response     └─────────────────┘
```

### 录音命令

| 命令 | 说明 |
|------|------|
| `start_recording` | 开始录音 |
| `stop_recording` | 停止录音，返回音频数据 |
| `cancel_recording` | 取消录音 |
| `get_recording_status` | 获取录音状态（是否录音中、音量、时长）|

## 依赖关系

```
voice/
├── asr_service.rs ──→ proxycast-services (voice_asr_service)
├── recording_service.rs ──→ proxycast-services (voice_recording_service)
├── config.rs ──→ proxycast-services (voice_config_service)
├── output_service.rs ──→ proxycast-services (voice_output_service)
├── processor.rs ──→ proxycast-services (voice_processor_service)
└── commands.rs ──→ 上述所有服务
```

## ASR 服务支持

| Provider | 状态 | 说明 |
|----------|------|------|
| Whisper Local | ✅ | 本地离线识别，需下载模型文件 |
| OpenAI Whisper | ✅ | 云端 API，支持自定义 base_url |
| 百度语音 | ✅ | 云端 API |
| 讯飞语音 | ✅ | WebSocket 流式识别 |

### 云端回退机制

当云端 ASR 服务（OpenAI、百度、讯飞）失败时，系统会自动回退到本地 Whisper 进行识别：

1. 首先尝试用户选择的云端服务
2. 如果云端失败，记录警告日志
3. 自动查找已配置的本地 Whisper 凭证
4. 使用本地 Whisper 进行回退识别
5. 如果回退也失败，返回详细错误信息

## Whisper 模型文件

模型文件存储路径：`~/Library/Application Support/proxycast/models/whisper/`

下载地址：https://huggingface.co/ggerganov/whisper.cpp/tree/main

| 模型 | 文件名 | 大小 |
|------|--------|------|
| tiny | `ggml-tiny.bin` | ~75MB |
| base | `ggml-base.bin` | ~142MB |
| small | `ggml-small.bin` | ~466MB |
| medium | `ggml-medium.bin` | ~1.5GB |
