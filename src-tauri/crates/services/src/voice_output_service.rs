//! 语音文本输出服务
//!
//! 提供模拟键盘输入和剪贴板输出能力。

use proxycast_core::config::VoiceOutputMode;
use voice_core::{OutputHandler, OutputMode};

/// 输出文字到系统
///
/// 根据配置的输出模式，将文字输出到当前焦点应用。
pub fn output_text(text: &str, mode: VoiceOutputMode) -> Result<(), String> {
    let output_mode = match mode {
        VoiceOutputMode::Type => OutputMode::Type,
        VoiceOutputMode::Clipboard => OutputMode::Clipboard,
        VoiceOutputMode::Both => OutputMode::Both,
    };

    let mut handler = OutputHandler::new().map_err(|e| format!("初始化输出处理器失败: {e}"))?;
    handler
        .output(text, output_mode)
        .map_err(|e| format!("输出文本失败: {e}"))
}
