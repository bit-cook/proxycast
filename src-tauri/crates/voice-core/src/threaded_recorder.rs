//! 录音服务
//!
//! 管理录音状态，提供录音控制接口。
//!
//! ## 线程安全设计
//!
//! 由于 `cpal::Stream` 不实现 `Send` trait，无法直接在 Tauri 的 async 命令中使用。
//! 本模块采用**独立线程 + channel 通信**的方案：
//!
//! ```text
//! ┌─────────────────┐     Command      ┌─────────────────┐
//! │  Tauri Command  │ ───────────────> │  Recording      │
//! │  (async)        │                  │  Thread         │
//! │                 │ <─────────────── │  (owns Stream)  │
//! └─────────────────┘     Response     └─────────────────┘
//! ```
//!
//! - 录音线程拥有 `cpal::Stream`，在独立线程中运行
//! - Tauri 命令通过 channel 发送控制指令
//! - 录音线程通过 channel 返回结果

use crate::types::AudioData;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// 录音控制命令
#[derive(Debug)]
pub enum RecordingCommand {
    /// 开始录音（可选指定设备 ID）
    Start(Option<String>),
    /// 停止录音
    Stop,
    /// 取消录音
    Cancel,
    /// 关闭录音线程
    Shutdown,
}

/// 录音响应
#[derive(Debug)]
pub enum RecordingResponse {
    /// 操作成功
    Ok,
    /// 停止录音成功，返回音频数据
    AudioData(AudioData),
    /// 操作失败
    Error(String),
}

/// 录音服务
///
/// 使用独立线程管理 cpal::Stream，通过 channel 与 Tauri 命令通信
pub struct RecordingService {
    /// 命令发送端
    command_tx: Option<Sender<RecordingCommand>>,
    /// 响应接收端
    response_rx: Option<Receiver<RecordingResponse>>,
    /// 录音线程句柄
    thread_handle: Option<JoinHandle<()>>,
    /// 是否正在录音（共享状态，用于快速查询）
    is_recording: Arc<AtomicBool>,
    /// 当前音量级别（共享状态，用于快速查询）
    volume_level: Arc<AtomicU32>,
    /// 录音开始时间（共享状态）
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl RecordingService {
    /// 创建新的录音服务
    pub fn new() -> Self {
        Self {
            command_tx: None,
            response_rx: None,
            thread_handle: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            volume_level: Arc::new(AtomicU32::new(0)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    /// 确保录音线程已启动
    fn ensure_thread_started(&mut self) {
        if self.command_tx.is_some() {
            return;
        }

        let (cmd_tx, cmd_rx) = mpsc::channel::<RecordingCommand>();
        let (resp_tx, resp_rx) = mpsc::channel::<RecordingResponse>();

        let is_recording = Arc::clone(&self.is_recording);
        let volume_level = Arc::clone(&self.volume_level);
        let start_time = Arc::clone(&self.start_time);

        let handle = thread::spawn(move || {
            recording_thread_main(cmd_rx, resp_tx, is_recording, volume_level, start_time);
        });

        self.command_tx = Some(cmd_tx);
        self.response_rx = Some(resp_rx);
        self.thread_handle = Some(handle);

        tracing::info!("[录音服务] 录音线程已启动");
    }

    /// 开始录音（可选指定设备 ID）
    pub fn start(&mut self, device_id: Option<String>) -> Result<(), String> {
        self.ensure_thread_started();

        let tx = self.command_tx.as_ref().ok_or("录音线程未启动")?;
        let rx = self.response_rx.as_ref().ok_or("录音线程未启动")?;

        tx.send(RecordingCommand::Start(device_id))
            .map_err(|e| format!("发送命令失败: {e}"))?;

        match rx.recv() {
            Ok(RecordingResponse::Ok) => {
                tracing::info!("[录音服务] 开始录音");
                Ok(())
            }
            Ok(RecordingResponse::Error(e)) => Err(e),
            Ok(_) => Err("意外的响应".to_string()),
            Err(e) => Err(format!("接收响应失败: {e}")),
        }
    }

    /// 停止录音并返回音频数据
    pub fn stop(&mut self) -> Result<AudioData, String> {
        let tx = self.command_tx.as_ref().ok_or("录音线程未启动")?;
        let rx = self.response_rx.as_ref().ok_or("录音线程未启动")?;

        tx.send(RecordingCommand::Stop)
            .map_err(|e| format!("发送命令失败: {e}"))?;

        match rx.recv() {
            Ok(RecordingResponse::AudioData(audio)) => {
                tracing::info!("[录音服务] 停止录音，时长: {:.2}s", audio.duration_secs);
                Ok(audio)
            }
            Ok(RecordingResponse::Error(e)) => Err(e),
            Ok(_) => Err("意外的响应".to_string()),
            Err(e) => Err(format!("接收响应失败: {e}")),
        }
    }

    /// 取消录音
    pub fn cancel(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(RecordingCommand::Cancel);
            // 使用 try_recv 避免阻塞，或者设置超时
            if let Some(rx) = &self.response_rx {
                // 尝试接收响应，但不阻塞太久
                use std::time::Duration;
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(_) => tracing::info!("[录音服务] 取消录音成功"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        tracing::warn!("[录音服务] 取消录音超时，强制继续");
                    }
                    Err(e) => {
                        tracing::warn!("[录音服务] 取消录音响应错误: {}", e);
                    }
                }
            }
        }
        // 无论如何都重置状态
        self.is_recording.store(false, Ordering::SeqCst);
        self.volume_level.store(0, Ordering::SeqCst);
        *self.start_time.lock() = None;
    }

    /// 获取当前音量级别（0-100）
    pub fn get_volume(&self) -> u32 {
        self.volume_level.load(Ordering::SeqCst)
    }

    /// 获取录音时长（秒）
    pub fn get_duration(&self) -> f32 {
        self.start_time
            .lock()
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }

    /// 是否正在录音
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    /// 关闭录音服务
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.command_tx.take() {
            let _ = tx.send(RecordingCommand::Shutdown);
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.response_rx = None;
        tracing::info!("[录音服务] 已关闭");
    }
}

impl Default for RecordingService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RecordingService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 录音线程主函数
///
/// 在独立线程中运行，拥有 cpal::Stream
fn recording_thread_main(
    cmd_rx: Receiver<RecordingCommand>,
    resp_tx: Sender<RecordingResponse>,
    is_recording: Arc<AtomicBool>,
    volume_level: Arc<AtomicU32>,
    start_time: Arc<Mutex<Option<Instant>>>,
) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    // 录音数据缓冲区
    let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    // 当前活跃的音频流
    let mut active_stream: Option<cpal::Stream> = None;
    // 实际使用的采样率和声道数
    let mut actual_sample_rate: u32 = 16000;
    #[allow(unused_assignments)]
    let mut actual_channels: u16 = 1;

    tracing::debug!("[录音线程] 开始运行");

    loop {
        match cmd_rx.recv() {
            Ok(RecordingCommand::Start(device_id)) => {
                // 如果已在录音，返回错误
                if is_recording.load(Ordering::SeqCst) {
                    let _ = resp_tx.send(RecordingResponse::Error("已在录音中".to_string()));
                    continue;
                }

                // 清空缓冲区
                samples.lock().clear();

                // 获取输入设备
                let host = cpal::default_host();
                let device = if let Some(ref id) = device_id {
                    // 查找指定设备
                    host.input_devices()
                        .ok()
                        .and_then(|mut devices| {
                            devices.find(|d| d.name().ok().as_ref() == Some(id))
                        })
                        .or_else(|| {
                            tracing::warn!("[录音线程] 未找到指定设备 {}，使用默认设备", id);
                            host.default_input_device()
                        })
                } else {
                    host.default_input_device()
                };

                let device = match device {
                    Some(d) => d,
                    None => {
                        let _ =
                            resp_tx.send(RecordingResponse::Error("未找到麦克风设备".to_string()));
                        continue;
                    }
                };

                tracing::info!("[录音线程] 使用麦克风: {:?}", device.name());

                // 获取设备支持的配置
                let supported_config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = resp_tx
                            .send(RecordingResponse::Error(format!("获取音频配置失败: {e}")));
                        continue;
                    }
                };

                tracing::info!(
                    "[录音线程] 设备支持配置: 采样率={}, 声道={}",
                    supported_config.sample_rate().0,
                    supported_config.channels()
                );

                // 使用设备默认配置
                actual_sample_rate = supported_config.sample_rate().0;
                actual_channels = supported_config.channels();

                let config = cpal::StreamConfig {
                    channels: actual_channels,
                    sample_rate: supported_config.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                };

                // 创建共享状态的克隆
                let samples_clone = Arc::clone(&samples);
                let volume_clone = Arc::clone(&volume_level);
                let is_rec_clone = Arc::clone(&is_recording);
                let channels = actual_channels;

                // 回调计数器（用于调试）
                let callback_count = Arc::new(AtomicU32::new(0));
                let callback_count_clone = Arc::clone(&callback_count);

                // 创建输入流
                let stream = match device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !is_rec_clone.load(Ordering::SeqCst) {
                            return;
                        }

                        // 增加回调计数
                        let count = callback_count_clone.fetch_add(1, Ordering::SeqCst);
                        if count == 0 {
                            tracing::info!("[录音线程] 首次收到音频数据，数据长度: {}", data.len());
                        } else if count % 100 == 0 {
                            tracing::debug!("[录音线程] 已收到 {} 次音频回调", count);
                        }

                        // 计算音量级别（使用 RMS 均方根，更准确反映音量）
                        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                        let rms = (sum_sq / data.len() as f32).sqrt();
                        // 将 RMS 值映射到 0-100 范围
                        // 静音时 RMS 约 0.001-0.01，说话时约 0.02-0.1
                        // 使用更高的系数来提高灵敏度
                        let level = ((rms * 1500.0).min(100.0)) as u32;

                        // 每 50 次回调打印一次音量（用于调试）
                        if count % 50 == 0 {
                            tracing::debug!("[录音线程] RMS: {:.6}, 音量: {}%", rms, level);
                        }

                        volume_clone.store(level, Ordering::SeqCst);

                        // 如果是多声道，转换为单声道
                        let mono_data: Vec<f32> = if channels > 1 {
                            data.chunks(channels as usize)
                                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                                .collect()
                        } else {
                            data.to_vec()
                        };

                        // 转换为 i16 并存储
                        let i16_samples: Vec<i16> = mono_data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                            .collect();

                        samples_clone.lock().extend(i16_samples);
                    },
                    |err| {
                        tracing::error!("[录音线程] 录音流错误: {}", err);
                    },
                    None,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ =
                            resp_tx.send(RecordingResponse::Error(format!("创建音频流失败: {e}")));
                        continue;
                    }
                };

                // 开始播放（录音）
                if let Err(e) = stream.play() {
                    let _ = resp_tx.send(RecordingResponse::Error(format!("启动录音失败: {e}")));
                    continue;
                }

                tracing::info!("[录音线程] stream.play() 成功，等待音频数据...");

                // 保存流和状态
                active_stream = Some(stream);
                is_recording.store(true, Ordering::SeqCst);
                *start_time.lock() = Some(Instant::now());

                let _ = resp_tx.send(RecordingResponse::Ok);
                tracing::info!(
                    "[录音线程] 开始录音，采样率: {}, 声道: {}",
                    actual_sample_rate,
                    actual_channels
                );
            }

            Ok(RecordingCommand::Stop) => {
                if !is_recording.load(Ordering::SeqCst) {
                    let _ = resp_tx.send(RecordingResponse::Error("未在录音中".to_string()));
                    continue;
                }

                // 停止录音
                is_recording.store(false, Ordering::SeqCst);

                // 停止并释放流
                if let Some(stream) = active_stream.take() {
                    drop(stream);
                }

                // 获取录音数据（已转换为单声道）
                let audio_samples = samples.lock().clone();
                let audio = AudioData::new(audio_samples, actual_sample_rate, 1);

                // 重置开始时间
                *start_time.lock() = None;
                volume_level.store(0, Ordering::SeqCst);

                // 检查录音时长
                if !audio.is_valid() {
                    let _ = resp_tx.send(RecordingResponse::Error(
                        "录音时间过短（需要至少 0.5 秒）".to_string(),
                    ));
                    continue;
                }

                let _ = resp_tx.send(RecordingResponse::AudioData(audio));
                tracing::info!("[录音线程] 停止录音");
            }

            Ok(RecordingCommand::Cancel) => {
                // 停止录音
                is_recording.store(false, Ordering::SeqCst);

                // 停止并释放流
                if let Some(stream) = active_stream.take() {
                    drop(stream);
                }

                // 清空缓冲区
                samples.lock().clear();

                // 重置状态
                *start_time.lock() = None;
                volume_level.store(0, Ordering::SeqCst);

                let _ = resp_tx.send(RecordingResponse::Ok);
                tracing::info!("[录音线程] 取消录音");
            }

            Ok(RecordingCommand::Shutdown) => {
                // 清理资源
                is_recording.store(false, Ordering::SeqCst);
                if let Some(stream) = active_stream.take() {
                    drop(stream);
                }
                tracing::info!("[录音线程] 收到关闭命令，退出");
                break;
            }

            Err(_) => {
                // channel 已关闭，退出线程
                tracing::info!("[录音线程] channel 已关闭，退出");
                break;
            }
        }
    }
}
