//! 终端会话管理器
//!
//! 管理所有终端会话的完整生命周期，集成 BlockController 和 BlockFile。
//!
//! ## 功能
//! - 会话创建、恢复、关闭的完整流程
//! - 集成 BlockController 进行进程管理
//! - 集成 BlockFile 进行输出持久化
//! - 集成 SessionMetadataStore 进行元数据存储
//! - 支持会话状态生命周期管理
//!
//! ## Requirements
//! - 3.1: 终端会话创建时创建对应的 Block_File
//! - 3.5: 应用启动时从数据库加载已保存的会话元数据
//! - 3.6: 用户请求恢复会话时从 Block_File 读取历史数据并重建会话
//! - 3.8: Block_File 读取失败时返回错误并允许创建新会话
//! - 3.9: 会话关闭时更新会话元数据状态为已完成

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::database::DbConnection;

use super::block_controller::ControllerRegistry;
use super::error::TerminalError;
use super::events::SessionStatus;
use super::persistence::{BlockFile, SessionMetadataStore, SessionRecord};
use super::pty_session::{PtySession, DEFAULT_COLS, DEFAULT_ROWS};

/// 会话元数据（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// 会话 ID
    pub id: String,
    /// 块 ID（与会话 ID 相同）
    pub block_id: String,
    /// 标签页 ID
    pub tab_id: String,
    /// 控制器类型
    pub controller_type: String,
    /// 连接名称
    pub connection: Option<String>,
    /// 会话状态
    pub status: SessionStatus,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: i64,
    /// 终端行数
    pub rows: u16,
    /// 终端列数
    pub cols: u16,
    /// 退出码
    pub exit_code: Option<i32>,
}

impl SessionMetadata {
    /// 从 SessionRecord 创建
    pub fn from_record(record: &SessionRecord, rows: u16, cols: u16) -> Self {
        let status = match record.status.as_str() {
            "running" => SessionStatus::Running,
            "done" => SessionStatus::Done,
            "error" => SessionStatus::Error,
            _ => SessionStatus::Connecting,
        };

        Self {
            id: record.id.clone(),
            block_id: record.block_id.clone(),
            tab_id: record.tab_id.clone(),
            controller_type: record.controller_type.clone(),
            connection: record.connection.clone(),
            status,
            created_at: record.created_at,
            rows,
            cols,
            exit_code: record.exit_code,
        }
    }
}

/// 内部会话数据
struct SessionData {
    /// 会话元数据
    metadata: SessionMetadata,
    /// 块文件存储
    block_file: Arc<BlockFile>,
    /// 旧版 PTY 会话（兼容模式）
    legacy_pty: Option<PtySession>,
}

/// 终端会话管理器
///
/// 管理所有终端会话的完整生命周期，集成 BlockController 和 BlockFile。
///
/// ## 架构说明
/// - 使用 ControllerRegistry 管理所有 BlockController
/// - 使用 SessionMetadataStore 持久化会话元数据
/// - 使用 BlockFile 持久化终端输出
/// - 支持新旧两种模式：
///   - 新模式：使用 BlockController + BlockFile
///   - 兼容模式：使用旧版 PtySession
pub struct TerminalSessionManager {
    /// 会话映射表
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    /// 控制器注册表
    controller_registry: Arc<ControllerRegistry>,
    /// 会话元数据存储
    session_store: Option<Arc<SessionMetadataStore>>,
    /// 块文件基础目录
    block_file_base_dir: PathBuf,
    /// Tauri 应用句柄
    app_handle: tauri::AppHandle,
}

impl TerminalSessionManager {
    /// 创建新的会话管理器
    ///
    /// # 参数
    /// - `app_handle`: Tauri 应用句柄
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        let block_file_base_dir = BlockFile::default_base_dir()
            .unwrap_or_else(|_| PathBuf::from(".proxycast/terminal_blocks"));

        tracing::info!(
            "[终端] 会话管理器已初始化，块文件目录: {:?}",
            block_file_base_dir
        );

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            controller_registry: Arc::new(ControllerRegistry::new()),
            session_store: None,
            block_file_base_dir,
            app_handle,
        }
    }

    /// 创建带数据库连接的会话管理器
    ///
    /// # 参数
    /// - `app_handle`: Tauri 应用句柄
    /// - `db`: 数据库连接
    ///
    /// _Requirements: 3.5_
    pub fn with_database(
        app_handle: tauri::AppHandle,
        db: DbConnection,
    ) -> Result<Self, TerminalError> {
        let mut manager = Self::new(app_handle);

        // 创建会话存储服务
        let session_store = SessionMetadataStore::new(db);
        session_store.init_tables()?;

        manager.session_store = Some(Arc::new(session_store));

        tracing::info!("[终端] 会话管理器已初始化（带数据库支持）");
        Ok(manager)
    }

    /// 获取控制器注册表
    pub fn controller_registry(&self) -> &Arc<ControllerRegistry> {
        &self.controller_registry
    }

    /// 获取会话存储服务
    pub fn session_store(&self) -> Option<&Arc<SessionMetadataStore>> {
        self.session_store.as_ref()
    }

    /// 创建新的终端会话
    ///
    /// 使用默认大小 (24x80) 创建 PTY 会话。
    ///
    /// # 返回
    /// - `Ok(String)`: 会话 ID
    /// - `Err(TerminalError)`: 创建失败
    ///
    /// _Requirements: 3.1_
    pub async fn create_session(&self) -> Result<String, TerminalError> {
        self.create_session_with_size(DEFAULT_ROWS, DEFAULT_COLS)
            .await
    }

    /// 创建新的终端会话（指定大小）
    ///
    /// # 参数
    /// - `rows`: 终端行数
    /// - `cols`: 终端列数
    ///
    /// # 返回
    /// - `Ok(String)`: 会话 ID
    /// - `Err(TerminalError)`: 创建失败
    ///
    /// _Requirements: 3.1_
    pub async fn create_session_with_size(
        &self,
        rows: u16,
        cols: u16,
    ) -> Result<String, TerminalError> {
        self.create_session_with_options(rows, cols, None).await
    }

    /// 创建新的终端会话（指定大小和工作目录）
    ///
    /// # 参数
    /// - `rows`: 终端行数
    /// - `cols`: 终端列数
    /// - `cwd`: 工作目录（可选）
    ///
    /// # 返回
    /// - `Ok(String)`: 会话 ID
    /// - `Err(TerminalError)`: 创建失败
    ///
    /// _Requirements: 3.1_
    pub async fn create_session_with_options(
        &self,
        rows: u16,
        cols: u16,
        cwd: Option<String>,
    ) -> Result<String, TerminalError> {
        let session_id = Uuid::new_v4().to_string();
        let block_id = session_id.clone();
        let tab_id = "default".to_string(); // TODO: 支持多标签页

        tracing::info!(
            "[终端] 创建会话 {}, 大小: {}x{}, cwd: {:?}",
            session_id,
            cols,
            rows,
            cwd
        );

        // 创建块文件
        let block_file = BlockFile::with_default_size(&block_id, &self.block_file_base_dir)?;
        let block_file = Arc::new(block_file);

        // 创建旧版 PTY 会话（兼容模式）
        let pty_session = PtySession::with_size_and_cwd(
            session_id.clone(),
            rows,
            cols,
            cwd,
            self.app_handle.clone(),
        )?;

        // 创建会话元数据
        let metadata = SessionMetadata {
            id: session_id.clone(),
            block_id: block_id.clone(),
            tab_id: tab_id.clone(),
            controller_type: "shell".to_string(),
            connection: None,
            status: SessionStatus::Running,
            created_at: Utc::now().timestamp_millis(),
            rows,
            cols,
            exit_code: None,
        };

        // 保存到数据库
        if let Some(store) = &self.session_store {
            let record = SessionRecord {
                id: session_id.clone(),
                block_id: block_id.clone(),
                tab_id: tab_id.clone(),
                controller_type: "shell".to_string(),
                connection: None,
                status: "running".to_string(),
                created_at: metadata.created_at,
                updated_at: metadata.created_at,
                exit_code: None,
            };
            store.save(&record)?;
        }

        // 创建会话数据
        let session_data = SessionData {
            metadata,
            block_file,
            legacy_pty: Some(pty_session),
        };

        // 添加到会话映射表
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_data);

        tracing::info!("[终端] 会话 {} 创建成功", session_id);
        Ok(session_id)
    }

    /// 向会话写入数据（Base64 编码）
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `data`: Base64 编码的数据
    ///
    /// _Requirements: 3.2_
    pub async fn write_to_session_base64(
        &self,
        session_id: &str,
        data: &str,
    ) -> Result<(), TerminalError> {
        let decoded = BASE64
            .decode(data)
            .map_err(|e| TerminalError::WriteFailed(format!("Base64 解码失败: {}", e)))?;
        self.write_to_session(session_id, &decoded).await
    }

    /// 向会话写入数据
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `data`: 原始数据
    ///
    /// _Requirements: 3.2_
    pub async fn write_to_session(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), TerminalError> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 使用旧版 PTY 会话写入
        if let Some(pty) = &session.legacy_pty {
            pty.write(data)?;
        }

        // 同时写入块文件（用于持久化）
        session.block_file.append_data(data)?;

        Ok(())
    }

    /// 调整会话终端大小
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    /// - `rows`: 新的行数
    /// - `cols`: 新的列数
    pub async fn resize_session(
        &self,
        session_id: &str,
        rows: u16,
        cols: u16,
    ) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 使用旧版 PTY 会话调整大小
        if let Some(pty) = &session.legacy_pty {
            pty.resize(rows, cols)?;
        }

        // 更新元数据
        session.metadata.rows = rows;
        session.metadata.cols = cols;

        tracing::debug!("[终端] 会话 {} 调整大小为 {}x{}", session_id, cols, rows);

        Ok(())
    }

    /// 关闭会话
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    ///
    /// _Requirements: 3.9_
    pub async fn close_session(&self, session_id: &str) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.write().await;

        if let Some(mut session) = sessions.remove(session_id) {
            // 关闭旧版 PTY 会话
            if let Some(pty) = session.legacy_pty.take() {
                pty.close().await?;
            }

            // 更新数据库状态
            if let Some(store) = &self.session_store {
                store.update_status(session_id, "done", None)?;
            }

            tracing::info!("[终端] 会话 {} 已关闭", session_id);
        }

        Ok(())
    }

    /// 获取所有会话列表
    pub async fn list_sessions(&self) -> Vec<SessionMetadata> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|s| s.metadata.clone()).collect()
    }

    /// 获取单个会话信息
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    pub async fn get_session(&self, session_id: &str) -> Option<SessionMetadata> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.metadata.clone())
    }

    /// 恢复会话（从持久化存储）
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    ///
    /// _Requirements: 3.6, 3.8_
    pub async fn restore_session(
        &self,
        session_id: &str,
    ) -> Result<SessionMetadata, TerminalError> {
        // 从数据库加载会话记录
        let store = self
            .session_store
            .as_ref()
            .ok_or_else(|| TerminalError::DatabaseError("会话存储未初始化".to_string()))?;

        let record = store
            .get_by_id(session_id)?
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 检查块文件是否存在
        let block_file_path = self
            .block_file_base_dir
            .join(format!("{}.block", session_id));
        if !block_file_path.exists() {
            return Err(TerminalError::BlockFileError(format!(
                "块文件不存在: {:?}",
                block_file_path
            )));
        }

        // 创建块文件引用
        let block_file = BlockFile::with_default_size(&record.block_id, &self.block_file_base_dir)?;
        let block_file = Arc::new(block_file);

        // 读取历史数据
        let _history = block_file.read_all()?;

        // 创建新的 PTY 会话
        let rows = DEFAULT_ROWS;
        let cols = DEFAULT_COLS;
        let pty_session =
            PtySession::with_size(session_id.to_string(), rows, cols, self.app_handle.clone())?;

        // 创建会话元数据
        let metadata = SessionMetadata::from_record(&record, rows, cols);

        // 创建会话数据
        let session_data = SessionData {
            metadata: metadata.clone(),
            block_file,
            legacy_pty: Some(pty_session),
        };

        // 添加到会话映射表
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session_data);

        // 更新数据库状态
        store.update_status(session_id, "running", None)?;

        tracing::info!("[终端] 会话 {} 已恢复", session_id);
        Ok(metadata)
    }

    /// 加载所有已保存的会话（应用启动时调用）
    ///
    /// _Requirements: 3.5_
    pub async fn load_saved_sessions(&self) -> Result<Vec<SessionMetadata>, TerminalError> {
        let store = match &self.session_store {
            Some(s) => s,
            None => return Ok(vec![]),
        };

        let records = store.get_all()?;
        let mut result = Vec::new();

        for record in records {
            // 只加载运行中的会话
            if record.status == "running" {
                let metadata = SessionMetadata::from_record(&record, DEFAULT_ROWS, DEFAULT_COLS);
                result.push(metadata);
            }
        }

        tracing::info!("[终端] 加载了 {} 个已保存的会话", result.len());
        Ok(result)
    }
}
