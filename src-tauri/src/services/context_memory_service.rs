//! 上下文记忆管理服务
//!
//! 基于文件系统的持久化记忆系统，解决 AI Agent 的上下文丢失、目标漂移、错误重复问题
//! 核心理念：Context Window = RAM, Filesystem = Disk

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// 记忆文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryFileType {
    /// 任务计划和阶段跟踪
    TaskPlan,
    /// 研究发现和重要信息
    Findings,
    /// 会话进度日志
    Progress,
    /// 错误跟踪记录
    ErrorLog,
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 条目 ID
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 文件类型
    pub file_type: MemoryFileType,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 标签
    pub tags: Vec<String>,
    /// 优先级 (1-5)
    pub priority: u8,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
    /// 是否已归档
    pub archived: bool,
}

/// 错误跟踪条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// 错误 ID
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 错误描述
    pub error_description: String,
    /// 尝试的解决方案
    pub attempted_solutions: Vec<String>,
    /// 失败次数
    pub failure_count: u32,
    /// 最后失败时间
    pub last_failure_at: i64,
    /// 是否已解决
    pub resolved: bool,
    /// 解决方案
    pub resolution: Option<String>,
}

/// 上下文记忆配置
#[derive(Debug, Clone)]
pub struct ContextMemoryConfig {
    /// 记忆文件存储目录
    pub memory_dir: PathBuf,
    /// 最大记忆条目数量
    pub max_entries_per_session: usize,
    /// 自动归档天数
    pub auto_archive_days: u32,
    /// 启用错误跟踪
    pub enable_error_tracking: bool,
    /// 最大错误重试次数
    pub max_error_retries: u32,
}

impl Default for ContextMemoryConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            memory_dir: home_dir.join(".proxycast").join("memory"),
            max_entries_per_session: 100,
            auto_archive_days: 30,
            enable_error_tracking: true,
            max_error_retries: 3,
        }
    }
}

/// 上下文记忆管理器
pub struct ContextMemoryService {
    /// 配置
    config: ContextMemoryConfig,
    /// 内存缓存
    memory_cache: Arc<Mutex<HashMap<String, Vec<MemoryEntry>>>>,
    /// 错误跟踪缓存
    error_cache: Arc<Mutex<HashMap<String, Vec<ErrorEntry>>>>,
}

impl Clone for ContextMemoryService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            memory_cache: Arc::clone(&self.memory_cache),
            error_cache: Arc::clone(&self.error_cache),
        }
    }
}

impl ContextMemoryService {
    /// 创建新的上下文记忆服务
    pub fn new(config: ContextMemoryConfig) -> Result<Self, String> {
        // 确保目录存在
        fs::create_dir_all(&config.memory_dir).map_err(|e| format!("创建记忆目录失败: {}", e))?;

        let service = Self {
            config,
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
            error_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        // 加载现有记忆
        service.load_all_memories()?;

        Ok(service)
    }

    /// 获取会话的记忆文件路径
    fn get_session_memory_dir(&self, session_id: &str) -> PathBuf {
        self.config.memory_dir.join(session_id)
    }

    /// 获取记忆文件路径
    fn get_memory_file_path(&self, session_id: &str, file_type: MemoryFileType) -> PathBuf {
        let filename = match file_type {
            MemoryFileType::TaskPlan => "task_plan.md",
            MemoryFileType::Findings => "findings.md",
            MemoryFileType::Progress => "progress.md",
            MemoryFileType::ErrorLog => "error_log.json",
        };
        self.get_session_memory_dir(session_id).join(filename)
    }

    /// 保存记忆条目
    pub fn save_memory_entry(&self, entry: &MemoryEntry) -> Result<(), String> {
        // 确保会话目录存在
        let session_dir = self.get_session_memory_dir(&entry.session_id);
        fs::create_dir_all(&session_dir).map_err(|e| format!("创建会话目录失败: {}", e))?;

        // 更新缓存
        {
            let mut cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
            let entries = cache
                .entry(entry.session_id.clone())
                .or_insert_with(Vec::new);

            // 查找并更新现有条目，或添加新条目
            if let Some(existing) = entries.iter_mut().find(|e| e.id == entry.id) {
                *existing = entry.clone();
            } else {
                entries.push(entry.clone());
            }

            // 限制条目数量
            if entries.len() > self.config.max_entries_per_session {
                entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                entries.truncate(self.config.max_entries_per_session);
            }
        }

        // 保存到文件
        self.save_memory_to_file(&entry.session_id, entry.file_type)?;

        info!(
            "已保存记忆条目: {} (会话: {})",
            entry.title, entry.session_id
        );
        Ok(())
    }

    /// 保存记忆到文件
    fn save_memory_to_file(
        &self,
        session_id: &str,
        file_type: MemoryFileType,
    ) -> Result<(), String> {
        let file_path = self.get_memory_file_path(session_id, file_type);

        let cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let empty_vec = Vec::new();
        let entries = cache.get(session_id).unwrap_or(&empty_vec);

        let filtered_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.file_type == file_type && !e.archived)
            .collect();

        match file_type {
            MemoryFileType::ErrorLog => {
                // 错误日志保存为 JSON
                let error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;
                let empty_error_vec = Vec::new();
                let error_entries = error_cache.get(session_id).unwrap_or(&empty_error_vec);
                let json_data = serde_json::to_string_pretty(error_entries)
                    .map_err(|e| format!("序列化错误日志失败: {}", e))?;
                fs::write(&file_path, json_data)
                    .map_err(|e| format!("写入错误日志文件失败: {}", e))?;
            }
            _ => {
                // 其他文件保存为 Markdown
                let markdown_content = self.generate_markdown_content(&filtered_entries, file_type);
                fs::write(&file_path, markdown_content)
                    .map_err(|e| format!("写入记忆文件失败: {}", e))?;
            }
        }

        Ok(())
    }

    /// 生成 Markdown 内容
    fn generate_markdown_content(
        &self,
        entries: &[&MemoryEntry],
        file_type: MemoryFileType,
    ) -> String {
        let mut content = String::new();

        let title = match file_type {
            MemoryFileType::TaskPlan => "# 任务计划与阶段跟踪",
            MemoryFileType::Findings => "# 研究发现与重要信息",
            MemoryFileType::Progress => "# 会话进度日志",
            MemoryFileType::ErrorLog => "# 错误跟踪记录",
        };

        content.push_str(title);
        content.push_str("\n\n");

        let description = match file_type {
            MemoryFileType::TaskPlan => "记录当前任务的计划、目标、阶段和进度状态。",
            MemoryFileType::Findings => "记录重要的研究发现、关键信息和决策依据。",
            MemoryFileType::Progress => "记录会话的进展历史和重要节点。",
            MemoryFileType::ErrorLog => "记录遇到的错误和解决方案。",
        };

        content.push_str(description);
        content.push_str("\n\n");

        // 按优先级和时间排序
        let mut sorted_entries = entries.to_vec();
        sorted_entries.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });

        for entry in sorted_entries {
            content.push_str(&format!("## {}\n\n", entry.title));

            // 添加元数据
            content.push_str(&format!(
                "**优先级**: {} | **标签**: {} | **更新时间**: {}\n\n",
                entry.priority,
                entry.tags.join(", "),
                chrono::DateTime::from_timestamp_millis(entry.updated_at)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "未知".to_string())
            ));

            content.push_str(&entry.content);
            content.push_str("\n\n---\n\n");
        }

        content
    }

    /// 获取会话记忆
    pub fn get_session_memories(
        &self,
        session_id: &str,
        file_type: Option<MemoryFileType>,
    ) -> Result<Vec<MemoryEntry>, String> {
        let cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let empty_vec = Vec::new();
        let entries = cache.get(session_id).unwrap_or(&empty_vec);

        let filtered_entries: Vec<_> = entries
            .iter()
            .filter(|e| !e.archived && file_type.map_or(true, |ft| e.file_type == ft))
            .cloned()
            .collect();

        Ok(filtered_entries)
    }

    /// 获取记忆文件内容（用于 AI 上下文）
    pub fn get_memory_context(&self, session_id: &str) -> Result<String, String> {
        let mut context = String::new();

        // 读取各类记忆文件
        for file_type in [
            MemoryFileType::TaskPlan,
            MemoryFileType::Findings,
            MemoryFileType::Progress,
        ] {
            let file_path = self.get_memory_file_path(session_id, file_type);
            if file_path.exists() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if !content.trim().is_empty() {
                        context.push_str(&content);
                        context.push_str("\n\n");
                    }
                }
            }
        }

        // 添加错误跟踪信息
        if self.config.enable_error_tracking {
            if let Ok(error_context) = self.get_error_context(session_id) {
                if !error_context.trim().is_empty() {
                    context.push_str("# 错误跟踪摘要\n\n");
                    context.push_str(&error_context);
                    context.push_str("\n\n");
                }
            }
        }

        Ok(context)
    }

    /// 记录错误
    pub fn record_error(
        &self,
        session_id: &str,
        error_description: &str,
        attempted_solution: &str,
    ) -> Result<(), String> {
        if !self.config.enable_error_tracking {
            return Ok(());
        }

        let mut error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;
        let errors = error_cache
            .entry(session_id.to_string())
            .or_insert_with(Vec::new);

        // 查找现有错误
        if let Some(existing_error) = errors
            .iter_mut()
            .find(|e| e.error_description == error_description)
        {
            existing_error
                .attempted_solutions
                .push(attempted_solution.to_string());
            existing_error.failure_count += 1;
            existing_error.last_failure_at = chrono::Utc::now().timestamp_millis();

            warn!(
                "重复错误记录 (第{}次): {} (会话: {})",
                existing_error.failure_count, error_description, session_id
            );
        } else {
            let error_entry = ErrorEntry {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session_id.to_string(),
                error_description: error_description.to_string(),
                attempted_solutions: vec![attempted_solution.to_string()],
                failure_count: 1,
                last_failure_at: chrono::Utc::now().timestamp_millis(),
                resolved: false,
                resolution: None,
            };

            errors.push(error_entry);
            info!("记录新错误: {} (会话: {})", error_description, session_id);
        }

        // 保存到文件
        self.save_memory_to_file(session_id, MemoryFileType::ErrorLog)?;

        Ok(())
    }

    /// 检查是否应该避免某个操作（3次错误协议）
    pub fn should_avoid_operation(&self, session_id: &str, operation_description: &str) -> bool {
        if !self.config.enable_error_tracking {
            return false;
        }

        let error_cache = self.error_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(errors) = error_cache.get(session_id) {
            for error in errors {
                if !error.resolved
                    && error.error_description.contains(operation_description)
                    && error.failure_count >= self.config.max_error_retries
                {
                    warn!(
                        "避免重复失败操作: {} (已失败{}次)",
                        operation_description, error.failure_count
                    );
                    return true;
                }
            }
        }

        false
    }

    /// 标记错误已解决
    pub fn mark_error_resolved(
        &self,
        session_id: &str,
        error_description: &str,
        resolution: &str,
    ) -> Result<(), String> {
        let mut error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;
        if let Some(errors) = error_cache.get_mut(session_id) {
            for error in errors {
                if error.error_description == error_description && !error.resolved {
                    error.resolved = true;
                    error.resolution = Some(resolution.to_string());
                    info!("错误已解决: {} (会话: {})", error_description, session_id);

                    // 保存到文件
                    drop(error_cache);
                    self.save_memory_to_file(session_id, MemoryFileType::ErrorLog)?;
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// 获取错误上下文摘要
    fn get_error_context(&self, session_id: &str) -> Result<String, String> {
        let error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;
        let empty_vec = Vec::new();
        let errors = error_cache.get(session_id).unwrap_or(&empty_vec);

        if errors.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::new();

        // 未解决的错误
        let unresolved_errors: Vec<_> = errors.iter().filter(|e| !e.resolved).collect();
        if !unresolved_errors.is_empty() {
            context.push_str("## 需要注意的错误\n\n");
            for error in unresolved_errors {
                context.push_str(&format!(
                    "- **{}** (失败{}次): {}\n",
                    error.error_description,
                    error.failure_count,
                    error
                        .attempted_solutions
                        .last()
                        .unwrap_or(&"无解决方案".to_string())
                ));
            }
            context.push_str("\n");
        }

        // 已解决的错误（最近的几个）
        let mut resolved_errors: Vec<_> = errors.iter().filter(|e| e.resolved).collect();
        resolved_errors.sort_by(|a, b| b.last_failure_at.cmp(&a.last_failure_at));
        resolved_errors.truncate(3);

        if !resolved_errors.is_empty() {
            context.push_str("## 已解决的错误\n\n");
            for error in resolved_errors {
                context.push_str(&format!(
                    "- **{}**: {}\n",
                    error.error_description,
                    error.resolution.as_ref().unwrap_or(&"已解决".to_string())
                ));
            }
        }

        Ok(context)
    }

    /// 加载所有记忆
    fn load_all_memories(&self) -> Result<(), String> {
        if !self.config.memory_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.config.memory_dir)
            .map_err(|e| format!("读取记忆目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(session_id) = path.file_name().and_then(|n| n.to_str()) {
                    self.load_session_memories(session_id)?;
                }
            }
        }

        Ok(())
    }

    /// 加载会话记忆
    fn load_session_memories(&self, session_id: &str) -> Result<(), String> {
        // 加载错误日志
        let error_file = self.get_memory_file_path(session_id, MemoryFileType::ErrorLog);
        if error_file.exists() {
            if let Ok(content) = fs::read_to_string(&error_file) {
                if let Ok(errors) = serde_json::from_str::<Vec<ErrorEntry>>(&content) {
                    let mut error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;
                    error_cache.insert(session_id.to_string(), errors);
                }
            }
        }

        debug!("已加载会话记忆: {}", session_id);
        Ok(())
    }

    /// 清理过期记忆
    pub fn cleanup_expired_memories(&self) -> Result<(), String> {
        let cutoff_time = chrono::Utc::now().timestamp_millis()
            - (self.config.auto_archive_days as i64 * 24 * 60 * 60 * 1000);

        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let mut archived_count = 0;

        for entries in memory_cache.values_mut() {
            for entry in entries.iter_mut() {
                if entry.updated_at < cutoff_time && !entry.archived {
                    entry.archived = true;
                    archived_count += 1;
                }
            }
        }

        if archived_count > 0 {
            info!("已归档 {} 个过期记忆条目", archived_count);
        }

        Ok(())
    }

    /// 获取记忆统计信息
    pub fn get_memory_stats(&self, session_id: &str) -> Result<MemoryStats, String> {
        let memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let error_cache = self.error_cache.lock().map_err(|e| e.to_string())?;

        let empty_memories = Vec::new();
        let empty_errors = Vec::new();
        let memories = memory_cache.get(session_id).unwrap_or(&empty_memories);
        let errors = error_cache.get(session_id).unwrap_or(&empty_errors);

        let active_memories = memories.iter().filter(|m| !m.archived).count();
        let archived_memories = memories.iter().filter(|m| m.archived).count();
        let unresolved_errors = errors.iter().filter(|e| !e.resolved).count();
        let resolved_errors = errors.iter().filter(|e| e.resolved).count();

        let memory_by_type = {
            let mut map = HashMap::new();
            for memory in memories.iter().filter(|m| !m.archived) {
                *map.entry(memory.file_type).or_insert(0) += 1;
            }
            map
        };

        Ok(MemoryStats {
            session_id: session_id.to_string(),
            active_memories,
            archived_memories,
            unresolved_errors,
            resolved_errors,
            memory_by_type,
            last_updated: memories.iter().map(|m| m.updated_at).max().unwrap_or(0),
        })
    }
}

/// 记忆统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub session_id: String,
    pub active_memories: usize,
    pub archived_memories: usize,
    pub unresolved_errors: usize,
    pub resolved_errors: usize,
    pub memory_by_type: HashMap<MemoryFileType, usize>,
    pub last_updated: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (ContextMemoryConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ContextMemoryConfig {
            memory_dir: temp_dir.path().to_path_buf(),
            max_entries_per_session: 10,
            auto_archive_days: 1,
            enable_error_tracking: true,
            max_error_retries: 3,
        };
        (config, temp_dir)
    }

    #[test]
    fn test_memory_service_creation() {
        let (config, _temp_dir) = create_test_config();
        let service = ContextMemoryService::new(config).unwrap();

        assert!(service.memory_cache.lock().unwrap().is_empty());
        assert!(service.error_cache.lock().unwrap().is_empty());
    }

    #[test]
    fn test_save_and_get_memory_entry() {
        let (config, _temp_dir) = create_test_config();
        let service = ContextMemoryService::new(config).unwrap();

        let entry = MemoryEntry {
            id: "test-entry".to_string(),
            session_id: "test-session".to_string(),
            file_type: MemoryFileType::TaskPlan,
            title: "测试任务".to_string(),
            content: "这是一个测试任务的内容".to_string(),
            tags: vec!["测试".to_string()],
            priority: 3,
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            archived: false,
        };

        service.save_memory_entry(&entry).unwrap();

        let memories = service
            .get_session_memories("test-session", Some(MemoryFileType::TaskPlan))
            .unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].title, "测试任务");
    }

    #[test]
    fn test_error_tracking() {
        let (config, _temp_dir) = create_test_config();
        let service = ContextMemoryService::new(config).unwrap();

        let session_id = "test-session";
        let error_desc = "测试错误";
        let solution = "尝试解决方案";

        // 记录错误
        service
            .record_error(session_id, error_desc, solution)
            .unwrap();
        assert!(!service.should_avoid_operation(session_id, error_desc));

        // 记录多次相同错误
        service
            .record_error(session_id, error_desc, "解决方案2")
            .unwrap();
        service
            .record_error(session_id, error_desc, "解决方案3")
            .unwrap();

        // 应该避免该操作
        assert!(service.should_avoid_operation(session_id, error_desc));

        // 标记为已解决
        service
            .mark_error_resolved(session_id, error_desc, "最终解决方案")
            .unwrap();
        assert!(!service.should_avoid_operation(session_id, error_desc));
    }

    #[test]
    fn test_memory_context_generation() {
        let (config, _temp_dir) = create_test_config();
        let service = ContextMemoryService::new(config).unwrap();

        let entry = MemoryEntry {
            id: "test-entry".to_string(),
            session_id: "test-session".to_string(),
            file_type: MemoryFileType::Findings,
            title: "重要发现".to_string(),
            content: "这是一个重要的研究发现".to_string(),
            tags: vec!["重要".to_string()],
            priority: 5,
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            archived: false,
        };

        service.save_memory_entry(&entry).unwrap();

        let context = service.get_memory_context("test-session").unwrap();
        assert!(context.contains("重要发现"));
        assert!(context.contains("这是一个重要的研究发现"));
    }

    #[test]
    fn test_memory_stats() {
        let (config, _temp_dir) = create_test_config();
        let service = ContextMemoryService::new(config).unwrap();

        let session_id = "test-session";

        // 添加记忆条目
        let entry = MemoryEntry {
            id: "test-entry".to_string(),
            session_id: session_id.to_string(),
            file_type: MemoryFileType::TaskPlan,
            title: "测试任务".to_string(),
            content: "内容".to_string(),
            tags: vec![],
            priority: 1,
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
            archived: false,
        };
        service.save_memory_entry(&entry).unwrap();

        // 添加错误
        service
            .record_error(session_id, "测试错误", "解决方案")
            .unwrap();

        let stats = service.get_memory_stats(session_id).unwrap();
        assert_eq!(stats.active_memories, 1);
        assert_eq!(stats.unresolved_errors, 1);
        assert_eq!(
            stats.memory_by_type.get(&MemoryFileType::TaskPlan),
            Some(&1)
        );
    }
}
