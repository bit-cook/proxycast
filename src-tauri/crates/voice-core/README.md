# voice-core

语音输入核心库 - 音频录制、语音识别、文字输出。

## 功能

- **音频录制** - 使用 cpal 进行跨平台音频采集
- **本地识别** - 使用 whisper-rs 进行本地 Whisper 识别
- **云端 ASR** - 支持讯飞、百度、OpenAI Whisper API
- **文字输出** - 支持模拟键盘输入和剪贴板

## 模块

```
src/
├── lib.rs           # 库入口
├── types.rs         # 类型定义
├── error.rs         # 错误类型
├── device.rs        # 音频设备枚举
├── recorder.rs      # 音频录制
├── threaded_recorder.rs # 线程化录音服务（可跨线程控制）
├── text_polish.rs   # 文本润色与本地 LLM 调用
├── transcriber.rs   # Whisper 本地识别
├── output.rs        # 文字输出
└── asr_client/      # 云端 ASR
    ├── mod.rs
    ├── openai.rs    # OpenAI Whisper
    ├── xunfei.rs    # 讯飞语音
    └── baidu.rs     # 百度语音
```

## 使用示例

```rust
use voice_core::{AudioRecorder, WhisperTranscriber, OutputHandler, OutputMode};

// 录音
let mut recorder = AudioRecorder::new()?;
recorder.start()?;
// ... 等待用户说话 ...
let audio = recorder.stop()?;

// 识别
let transcriber = WhisperTranscriber::new(model_path, WhisperModel::Base, "zh")?;
let result = transcriber.transcribe(&audio)?;

// 输出
let mut output = OutputHandler::new()?;
output.output(&result.text, OutputMode::Type)?;
```

## 依赖

- `cpal` - 跨平台音频采集
- `whisper-rs` - Whisper.cpp Rust 绑定
- `enigo` - 跨平台键盘模拟
- `arboard` - 跨平台剪贴板
- `reqwest` - HTTP 客户端（云端 ASR）
