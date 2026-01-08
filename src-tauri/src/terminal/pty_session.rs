//! PTY 会话封装
//!
//! 封装单个 PTY 进程，处理输入输出和生命周期管理。
//!
//! ## 功能
//! - 创建和管理 PTY 子进程（使用默认大小预创建）
//! - 异步读取 PTY 输出并通过 Tauri Event 推送
//! - 处理 PTY 输入写入
//! - 监控进程退出状态
//! - 保存输出历史（循环缓冲区）
//!
//! ## 架构说明
//! PTY 在后端预创建，使用默认大小 (24x80)。前端连接后通过 resize 同步实际大小。
//! 输出历史保存在循环缓冲区中，前端连接时可以获取历史数据。

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tauri::Emitter;
use tokio::sync::RwLock;

use super::error::TerminalError;
use super::events::{event_names, SessionStatus, TerminalOutputEvent, TerminalStatusEvent};

/// 默认终端行数
pub const DEFAULT_ROWS: u16 = 24;
/// 默认终端列数
pub const DEFAULT_COLS: u16 = 80;
/// 输出历史缓冲区最大大小 (1MB)
const OUTPUT_BUFFER_MAX_SIZE: usize = 1024 * 1024;

/// 循环缓冲区，用于存储终端输出历史
struct CircularBuffer {
    data: Vec<u8>,
    max_size: usize,
}

impl CircularBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn append(&mut self, new_data: &[u8]) {
        // 如果新数据超过最大大小，只保留最后 max_size 字节
        if new_data.len() >= self.max_size {
            self.data.clear();
            self.data
                .extend_from_slice(&new_data[new_data.len() - self.max_size..]);
            return;
        }

        // 追加新数据
        self.data.extend_from_slice(new_data);

        // 如果超过最大大小，移除开头的数据
        if self.data.len() > self.max_size {
            let excess = self.data.len() - self.max_size;
            self.data.drain(0..excess);
        }
    }

    fn get_all(&self) -> Vec<u8> {
        self.data.clone()
    }
}

/// PTY 会话
pub struct PtySession {
    /// 会话 ID
    id: String,
    /// PTY 写入器（使用 Mutex 保证线程安全）
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// PTY Master（用于调整大小，使用 Mutex 保证线程安全）
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    /// 会话状态
    status: Arc<RwLock<SessionStatus>>,
    /// 关闭标志
    shutdown_flag: Arc<AtomicBool>,
    /// 输出历史缓冲区
    output_buffer: Arc<Mutex<CircularBuffer>>,
}

impl PtySession {
    /// 创建新的 PTY 会话（使用默认大小）
    ///
    /// PTY 使用默认大小 (24x80) 预创建，前端连接后通过 resize 同步实际大小。
    ///
    /// # 参数
    /// - `id`: 会话 ID
    /// - `app_handle`: Tauri 应用句柄
    ///
    /// # 返回
    /// - `Ok(PtySession)`: 创建成功
    /// - `Err(TerminalError)`: 创建失败
    pub fn new(id: String, app_handle: tauri::AppHandle) -> Result<Self, TerminalError> {
        Self::with_size(id, DEFAULT_ROWS, DEFAULT_COLS, app_handle)
    }

    /// 创建新的 PTY 会话（指定大小）
    ///
    /// # 参数
    /// - `id`: 会话 ID
    /// - `rows`: 终端行数
    /// - `cols`: 终端列数
    /// - `app_handle`: Tauri 应用句柄
    ///
    /// # 返回
    /// - `Ok(PtySession)`: 创建成功
    /// - `Err(TerminalError)`: 创建失败
    pub fn with_size(
        id: String,
        rows: u16,
        cols: u16,
        app_handle: tauri::AppHandle,
    ) -> Result<Self, TerminalError> {
        Self::with_size_and_cwd(id, rows, cols, None, app_handle)
    }

    /// 创建新的 PTY 会话（指定大小和工作目录）
    ///
    /// # 参数
    /// - `id`: 会话 ID
    /// - `rows`: 终端行数
    /// - `cols`: 终端列数
    /// - `cwd`: 工作目录（可选，默认为用户主目录）
    /// - `app_handle`: Tauri 应用句柄
    ///
    /// # 返回
    /// - `Ok(PtySession)`: 创建成功
    /// - `Err(TerminalError)`: 创建失败
    pub fn with_size_and_cwd(
        id: String,
        rows: u16,
        cols: u16,
        cwd: Option<String>,
        app_handle: tauri::AppHandle,
    ) -> Result<Self, TerminalError> {
        tracing::info!(
            "[终端] 创建 PTY 会话 {}, 大小: {}x{}, cwd: {:?}",
            id,
            cols,
            rows,
            cwd
        );

        let pty_system = native_pty_system();

        // 创建 PTY
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        // 获取用户默认 shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        tracing::info!("[终端] 使用 shell: {}", shell);

        // 构建命令
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");

        // 设置工作目录
        if let Some(dir) = cwd {
            // 展开 ~ 为用户主目录
            let expanded_dir = if dir.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&dir[2..])
                } else {
                    std::path::PathBuf::from(&dir)
                }
            } else if dir == "~" {
                dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(&dir))
            } else {
                std::path::PathBuf::from(&dir)
            };

            if expanded_dir.exists() && expanded_dir.is_dir() {
                tracing::info!("[终端] 设置工作目录: {:?}", expanded_dir);
                cmd.cwd(expanded_dir);
            } else {
                tracing::warn!(
                    "[终端] 工作目录不存在或不是目录: {:?}, 使用主目录",
                    expanded_dir
                );
                if let Some(home) = dirs::home_dir() {
                    cmd.cwd(home);
                }
            }
        } else if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        // 启动子进程
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        // 获取写入器
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        // 获取读取器
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        let status = Arc::new(RwLock::new(SessionStatus::Running));
        let status_clone = status.clone();
        let id_clone = id.clone();

        // 创建关闭标志
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_flag_clone = shutdown_flag.clone();

        // 创建输出缓冲区
        let output_buffer = Arc::new(Mutex::new(CircularBuffer::new(OUTPUT_BUFFER_MAX_SIZE)));
        let output_buffer_clone = output_buffer.clone();

        // 获取当前 tokio runtime handle（在主线程中获取）
        let runtime_handle = tokio::runtime::Handle::current();

        // 启动输出读取任务（使用独立线程）
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];

            loop {
                // 检查关闭标志
                if shutdown_flag_clone.load(Ordering::Relaxed) {
                    tracing::debug!("[终端] 会话 {} 收到关闭信号", id_clone);
                    break;
                }

                // 读取输出
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF，进程已退出
                        tracing::info!("[终端] 会话 {} 进程已退出", id_clone);
                        runtime_handle.block_on(async {
                            *status_clone.write().await = SessionStatus::Done;
                        });

                        // 发送状态事件
                        let _ = app_handle.emit(
                            event_names::TERMINAL_STATUS,
                            TerminalStatusEvent {
                                session_id: id_clone.clone(),
                                status: SessionStatus::Done,
                                exit_code: Some(0),
                                error: None,
                            },
                        );
                        break;
                    }
                    Ok(n) => {
                        let output_data = &buffer[..n];

                        // 保存到输出缓冲区
                        output_buffer_clone.lock().append(output_data);

                        // 发送输出事件
                        let data = BASE64.encode(output_data);
                        let _ = app_handle.emit(
                            event_names::TERMINAL_OUTPUT,
                            TerminalOutputEvent {
                                session_id: id_clone.clone(),
                                data,
                            },
                        );
                    }
                    Err(e) => {
                        // 检查是否是因为关闭导致的错误
                        if shutdown_flag_clone.load(Ordering::Relaxed) {
                            break;
                        }

                        tracing::error!("[终端] 会话 {} 读取错误: {}", id_clone, e);
                        runtime_handle.block_on(async {
                            *status_clone.write().await = SessionStatus::Error;
                        });

                        let _ = app_handle.emit(
                            event_names::TERMINAL_STATUS,
                            TerminalStatusEvent {
                                session_id: id_clone.clone(),
                                status: SessionStatus::Error,
                                exit_code: None,
                                error: Some(e.to_string()),
                            },
                        );
                        break;
                    }
                }
            }
        });

        tracing::info!("[终端] 会话 {} 已创建 ({}x{})", id, cols, rows);

        Ok(Self {
            id,
            writer: Arc::new(Mutex::new(writer)),
            master: Arc::new(Mutex::new(pair.master)),
            status,
            shutdown_flag,
            output_buffer,
        })
    }

    /// 获取会话 ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 写入数据到 PTY
    pub fn write(&self, data: &[u8]) -> Result<(), TerminalError> {
        let mut writer = self.writer.lock();
        writer
            .write_all(data)
            .map_err(|e| TerminalError::WriteFailed(e.to_string()))?;
        writer
            .flush()
            .map_err(|e| TerminalError::WriteFailed(e.to_string()))?;
        Ok(())
    }

    /// 调整 PTY 大小
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), TerminalError> {
        let master = self.master.lock();
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::ResizeFailed(e.to_string()))?;
        tracing::debug!("[终端] 会话 {} 调整大小为 {}x{}", self.id, cols, rows);
        Ok(())
    }

    /// 获取当前状态
    pub async fn status(&self) -> SessionStatus {
        *self.status.read().await
    }

    /// 获取输出历史数据（Base64 编码）
    pub fn get_output_history(&self) -> String {
        let buffer = self.output_buffer.lock();
        let data = buffer.get_all();
        BASE64.encode(&data)
    }

    /// 关闭会话
    pub async fn close(&self) -> Result<(), TerminalError> {
        // 设置关闭标志
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // 更新状态
        *self.status.write().await = SessionStatus::Done;

        tracing::info!("[终端] 会话 {} 已关闭", self.id);
        Ok(())
    }
}
