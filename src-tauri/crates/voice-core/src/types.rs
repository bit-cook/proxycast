//! 类型定义
//!
//! 定义语音输入相关的核心类型。

use serde::{Deserialize, Serialize};

/// 音频数据
#[derive(Debug, Clone)]
pub struct AudioData {
    /// PCM 采样数据（16-bit signed）
    pub samples: Vec<i16>,
    /// 采样率（默认 16000）
    pub sample_rate: u32,
    /// 声道数（默认 1）
    pub channels: u16,
    /// 录音时长（秒）
    pub duration_secs: f32,
}

impl AudioData {
    /// 创建新的音频数据
    pub fn new(samples: Vec<i16>, sample_rate: u32, channels: u16) -> Self {
        let duration_secs = samples.len() as f32 / sample_rate as f32 / channels as f32;
        Self {
            samples,
            sample_rate,
            channels,
            duration_secs,
        }
    }

    /// 检查音频是否有效（时长 >= 0.5 秒）
    pub fn is_valid(&self) -> bool {
        self.duration_secs >= 0.5
    }

    /// 从 PCM16 LE 字节创建音频数据
    pub fn from_pcm16le_bytes(bytes: &[u8], sample_rate: u32, channels: u16) -> Self {
        let samples = bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        Self::new(samples, sample_rate, channels)
    }

    /// 转换为 PCM16 LE 字节
    pub fn to_pcm16le_bytes(&self) -> Vec<u8> {
        self.samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    /// 转换为 WAV 格式字节
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        if let Ok(mut writer) = hound::WavWriter::new(&mut cursor, spec) {
            for sample in &self.samples {
                let _ = writer.write_sample(*sample);
            }
            let _ = writer.finalize();
        }

        cursor.into_inner()
    }
}

/// 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeResult {
    /// 识别文本
    pub text: String,
    /// 语言（如 "zh", "en"）
    pub language: Option<String>,
    /// 置信度（0.0 - 1.0）
    pub confidence: Option<f32>,
    /// 分段信息
    pub segments: Vec<Segment>,
}

/// 识别分段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// 开始时间（秒）
    pub start: f32,
    /// 结束时间（秒）
    pub end: f32,
    /// 文本内容
    pub text: String,
}

/// ASR 引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrEngine {
    /// 本地 Whisper
    WhisperLocal,
    /// 讯飞语音
    Xunfei,
    /// 百度语音
    Baidu,
    /// OpenAI Whisper API
    OpenAI,
}

/// Whisper 模型大小
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModel {
    /// tiny - 最小，最快
    Tiny,
    /// base - 基础
    Base,
    /// small - 小型
    Small,
    /// medium - 中型
    Medium,
    /// large - 大型，最准确
    Large,
}

impl WhisperModel {
    /// 获取模型文件名
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large.bin",
        }
    }
}

/// 输出模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OutputMode {
    /// 模拟键盘输入
    #[default]
    Type,
    /// 复制到剪贴板
    Clipboard,
    /// 两者都做
    Both,
}
