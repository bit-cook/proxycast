//! 工具钩子管理服务
//!
//! 提供工具执行前后的钩子机制，用于自动化上下文记忆管理

use crate::services::context_memory_service::{ContextMemoryService, MemoryEntry, MemoryFileType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// 钩子触发时机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    /// 会话开始时
    SessionStart,
    /// 工具使用前
    PreToolUse,
    /// 工具使用后
    PostToolUse,
    /// 会话停止时
    Stop,
}

/// 钩子动作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    /// 保存发现
    SaveFinding {
        title: String,
        content: String,
        tags: Vec<String>,
        priority: u8,
    },
    /// 更新任务计划
    UpdateTaskPlan {
        title: String,
        content: String,
        priority: u8,
    },
    /// 记录进度
    LogProgress { title: String, content: String },
    /// 记录错误
    RecordError {
        error_description: String,
        attempted_solution: String,
    },
    /// 自定义动作
    Custom {
        action_type: String,
        parameters: HashMap<String, String>,
    },
}

/// 钩子规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 触发时机
    pub trigger: HookTrigger,
    /// 触发条件
    pub conditions: Vec<HookCondition>,
    /// 执行动作
    pub actions: Vec<HookAction>,
    /// 是否启用
    pub enabled: bool,
    /// 优先级 (数字越小优先级越高)
    pub priority: u32,
    /// 创建时间
    pub created_at: i64,
}

/// 钩子条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookCondition {
    /// 工具名称匹配
    ToolNameEquals(String),
    /// 工具名称包含
    ToolNameContains(String),
    /// 消息内容包含
    MessageContains(String),
    /// 会话消息数量大于
    MessageCountGreaterThan(usize),
    /// 错误次数大于
    ErrorCountGreaterThan(u32),
    /// 自定义条件
    Custom {
        condition_type: String,
        parameters: HashMap<String, String>,
    },
}

/// 钩子执行上下文
#[derive(Debug, Clone)]
pub struct HookContext {
    /// 会话 ID
    pub session_id: String,
    /// 工具名称（如果适用）
    pub tool_name: Option<String>,
    /// 工具参数（如果适用）
    pub tool_parameters: Option<HashMap<String, String>>,
    /// 工具结果（如果适用）
    pub tool_result: Option<String>,
    /// 消息内容
    pub message_content: Option<String>,
    /// 会话消息数量
    pub message_count: usize,
    /// 错误信息（如果适用）
    pub error_info: Option<String>,
    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 工具钩子管理器
pub struct ToolHooksService {
    /// 钩子规则
    rules: Arc<Mutex<Vec<HookRule>>>,
    /// 上下文记忆服务
    memory_service: Arc<ContextMemoryService>,
    /// 执行统计
    execution_stats: Arc<Mutex<HashMap<String, HookExecutionStats>>>,
}

/// 钩子执行统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookExecutionStats {
    /// 执行次数
    pub execution_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 最后执行时间
    pub last_execution_at: i64,
    /// 平均执行时间（毫秒）
    pub average_execution_time_ms: f64,
}

impl ToolHooksService {
    /// 创建新的工具钩子服务
    pub fn new(memory_service: Arc<ContextMemoryService>) -> Self {
        let service = Self {
            rules: Arc::new(Mutex::new(Vec::new())),
            memory_service,
            execution_stats: Arc::new(Mutex::new(HashMap::new())),
        };

        // 注册默认钩子规则
        service.register_default_hooks();

        service
    }

    /// 注册默认钩子规则
    fn register_default_hooks(&self) {
        let default_rules = vec![
            // 会话开始时创建任务计划
            HookRule {
                id: "session-start-task-plan".to_string(),
                name: "会话开始任务计划".to_string(),
                description: "会话开始时自动创建任务计划记录".to_string(),
                trigger: HookTrigger::SessionStart,
                conditions: vec![],
                actions: vec![HookAction::UpdateTaskPlan {
                    title: "会话任务计划".to_string(),
                    content: "新会话开始，等待用户输入任务目标...".to_string(),
                    priority: 3,
                }],
                enabled: true,
                priority: 1,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
            // 工具使用后记录进度（2-Action 规则）
            HookRule {
                id: "post-tool-progress-log".to_string(),
                name: "工具使用进度记录".to_string(),
                description: "工具使用后自动记录进度（2-Action 规则）".to_string(),
                trigger: HookTrigger::PostToolUse,
                conditions: vec![],
                actions: vec![HookAction::LogProgress {
                    title: "工具执行记录".to_string(),
                    content: "工具执行完成，结果已记录".to_string(),
                }],
                enabled: true,
                priority: 2,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
            // 重要发现自动保存
            HookRule {
                id: "important-finding-save".to_string(),
                name: "重要发现保存".to_string(),
                description: "检测到重要信息时自动保存".to_string(),
                trigger: HookTrigger::PostToolUse,
                conditions: vec![
                    HookCondition::MessageContains("重要".to_string()),
                    HookCondition::MessageContains("发现".to_string()),
                ],
                actions: vec![HookAction::SaveFinding {
                    title: "重要发现".to_string(),
                    content: "检测到重要信息，已自动保存".to_string(),
                    tags: vec!["重要".to_string(), "自动保存".to_string()],
                    priority: 4,
                }],
                enabled: true,
                priority: 3,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
            // 错误自动记录
            HookRule {
                id: "error-auto-record".to_string(),
                name: "错误自动记录".to_string(),
                description: "检测到错误时自动记录".to_string(),
                trigger: HookTrigger::PostToolUse,
                conditions: vec![HookCondition::MessageContains("错误".to_string())],
                actions: vec![HookAction::RecordError {
                    error_description: "检测到错误".to_string(),
                    attempted_solution: "正在尝试解决".to_string(),
                }],
                enabled: true,
                priority: 1,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
            // 会话停止时保存摘要
            HookRule {
                id: "session-stop-summary".to_string(),
                name: "会话停止摘要".to_string(),
                description: "会话停止时保存会话摘要".to_string(),
                trigger: HookTrigger::Stop,
                conditions: vec![HookCondition::MessageCountGreaterThan(5)],
                actions: vec![HookAction::SaveFinding {
                    title: "会话摘要".to_string(),
                    content: "会话已结束，主要成果和发现已记录".to_string(),
                    tags: vec!["摘要".to_string(), "会话结束".to_string()],
                    priority: 3,
                }],
                enabled: true,
                priority: 2,
                created_at: chrono::Utc::now().timestamp_millis(),
            },
        ];

        let mut rules = self.rules.lock().unwrap();
        rules.extend(default_rules);
        info!("已注册 {} 个默认钩子规则", rules.len());
    }

    /// 执行钩子
    pub fn execute_hooks(&self, trigger: HookTrigger, context: &HookContext) -> Result<(), String> {
        let rules = self.rules.lock().map_err(|e| e.to_string())?;

        // 获取匹配的规则并按优先级排序
        let mut matching_rules: Vec<_> = rules
            .iter()
            .filter(|rule| rule.enabled && rule.trigger == trigger)
            .filter(|rule| self.evaluate_conditions(rule, context))
            .collect();

        matching_rules.sort_by_key(|rule| rule.priority);

        debug!(
            "触发钩子 {:?}，匹配到 {} 个规则 (会话: {})",
            trigger,
            matching_rules.len(),
            context.session_id
        );

        // 执行匹配的规则
        for rule in matching_rules {
            if let Err(e) = self.execute_rule(rule, context) {
                error!("执行钩子规则失败 {}: {}", rule.name, e);
                self.update_execution_stats(&rule.id, false, 0.0);
            } else {
                self.update_execution_stats(&rule.id, true, 0.0);
            }
        }

        Ok(())
    }

    /// 评估钩子条件
    fn evaluate_conditions(&self, rule: &HookRule, context: &HookContext) -> bool {
        if rule.conditions.is_empty() {
            return true;
        }

        for condition in &rule.conditions {
            if !self.evaluate_single_condition(condition, context) {
                return false;
            }
        }

        true
    }

    /// 评估单个条件
    fn evaluate_single_condition(&self, condition: &HookCondition, context: &HookContext) -> bool {
        match condition {
            HookCondition::ToolNameEquals(name) => {
                context.tool_name.as_ref().map_or(false, |tn| tn == name)
            }
            HookCondition::ToolNameContains(substring) => context
                .tool_name
                .as_ref()
                .map_or(false, |tn| tn.contains(substring)),
            HookCondition::MessageContains(substring) => {
                context
                    .message_content
                    .as_ref()
                    .map_or(false, |mc| mc.contains(substring))
                    || context
                        .tool_result
                        .as_ref()
                        .map_or(false, |tr| tr.contains(substring))
            }
            HookCondition::MessageCountGreaterThan(count) => context.message_count > *count,
            HookCondition::ErrorCountGreaterThan(_count) => {
                // 这里可以从 memory_service 获取错误计数
                context.error_info.is_some()
            }
            HookCondition::Custom {
                condition_type: _,
                parameters: _,
            } => {
                // 自定义条件的实现
                true
            }
        }
    }

    /// 执行钩子规则
    fn execute_rule(&self, rule: &HookRule, context: &HookContext) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        for action in &rule.actions {
            self.execute_action(action, context)?;
        }

        let execution_time = start_time.elapsed().as_millis() as f64;
        self.update_execution_stats(&rule.id, true, execution_time);

        debug!(
            "执行钩子规则成功: {} (耗时: {:.2}ms)",
            rule.name, execution_time
        );
        Ok(())
    }

    /// 执行钩子动作
    fn execute_action(&self, action: &HookAction, context: &HookContext) -> Result<(), String> {
        match action {
            HookAction::SaveFinding {
                title,
                content,
                tags,
                priority,
            } => {
                let entry = MemoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: context.session_id.clone(),
                    file_type: MemoryFileType::Findings,
                    title: self.interpolate_template(title, context),
                    content: self.interpolate_template(content, context),
                    tags: tags.clone(),
                    priority: *priority,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                    archived: false,
                };
                self.memory_service.save_memory_entry(&entry)?;
            }

            HookAction::UpdateTaskPlan {
                title,
                content,
                priority,
            } => {
                let entry = MemoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: context.session_id.clone(),
                    file_type: MemoryFileType::TaskPlan,
                    title: self.interpolate_template(title, context),
                    content: self.interpolate_template(content, context),
                    tags: vec!["任务计划".to_string()],
                    priority: *priority,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                    archived: false,
                };
                self.memory_service.save_memory_entry(&entry)?;
            }

            HookAction::LogProgress { title, content } => {
                let entry = MemoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: context.session_id.clone(),
                    file_type: MemoryFileType::Progress,
                    title: self.interpolate_template(title, context),
                    content: self.interpolate_template(content, context),
                    tags: vec!["进度".to_string()],
                    priority: 2,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                    archived: false,
                };
                self.memory_service.save_memory_entry(&entry)?;
            }

            HookAction::RecordError {
                error_description,
                attempted_solution,
            } => {
                let error_desc = self.interpolate_template(error_description, context);
                let solution = self.interpolate_template(attempted_solution, context);
                self.memory_service
                    .record_error(&context.session_id, &error_desc, &solution)?;
            }

            HookAction::Custom {
                action_type: _,
                parameters: _,
            } => {
                // 自定义动作的实现
                debug!("执行自定义钩子动作");
            }
        }

        Ok(())
    }

    /// 模板插值
    fn interpolate_template(&self, template: &str, context: &HookContext) -> String {
        let mut result = template.to_string();

        // 替换常见的模板变量
        result = result.replace("{session_id}", &context.session_id);

        if let Some(tool_name) = &context.tool_name {
            result = result.replace("{tool_name}", tool_name);
        }

        if let Some(message_content) = &context.message_content {
            let preview = if message_content.len() > 100 {
                format!("{}...", &message_content[..100])
            } else {
                message_content.clone()
            };
            result = result.replace("{message_preview}", &preview);
        }

        if let Some(tool_result) = &context.tool_result {
            let preview = if tool_result.len() > 200 {
                format!("{}...", &tool_result[..200])
            } else {
                tool_result.clone()
            };
            result = result.replace("{tool_result_preview}", &preview);
        }

        result = result.replace("{message_count}", &context.message_count.to_string());
        result = result.replace(
            "{timestamp}",
            &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        // 替换元数据变量
        for (key, value) in &context.metadata {
            result = result.replace(&format!("{{{}}}", key), value);
        }

        result
    }

    /// 更新执行统计
    fn update_execution_stats(&self, rule_id: &str, success: bool, execution_time_ms: f64) {
        let mut stats = self.execution_stats.lock().unwrap();
        let entry = stats
            .entry(rule_id.to_string())
            .or_insert_with(Default::default);

        entry.execution_count += 1;
        if success {
            entry.success_count += 1;
        } else {
            entry.failure_count += 1;
        }
        entry.last_execution_at = chrono::Utc::now().timestamp_millis();

        // 更新平均执行时间
        if execution_time_ms > 0.0 {
            let total_time = entry.average_execution_time_ms * (entry.execution_count - 1) as f64;
            entry.average_execution_time_ms =
                (total_time + execution_time_ms) / entry.execution_count as f64;
        }
    }

    /// 添加钩子规则
    pub fn add_hook_rule(&self, rule: HookRule) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;

        // 检查是否已存在相同 ID 的规则
        if rules.iter().any(|r| r.id == rule.id) {
            return Err(format!("钩子规则 ID 已存在: {}", rule.id));
        }

        rules.push(rule.clone());
        info!("已添加钩子规则: {}", rule.name);
        Ok(())
    }

    /// 移除钩子规则
    pub fn remove_hook_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;

        let initial_len = rules.len();
        rules.retain(|r| r.id != rule_id);

        if rules.len() == initial_len {
            return Err(format!("未找到钩子规则: {}", rule_id));
        }

        info!("已移除钩子规则: {}", rule_id);
        Ok(())
    }

    /// 启用/禁用钩子规则
    pub fn toggle_hook_rule(&self, rule_id: &str, enabled: bool) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;

        if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            info!(
                "钩子规则 {} 已{}",
                rule.name,
                if enabled { "启用" } else { "禁用" }
            );
            Ok(())
        } else {
            Err(format!("未找到钩子规则: {}", rule_id))
        }
    }

    /// 获取所有钩子规则
    pub fn get_hook_rules(&self) -> Result<Vec<HookRule>, String> {
        let rules = self.rules.lock().map_err(|e| e.to_string())?;
        Ok(rules.clone())
    }

    /// 获取执行统计
    pub fn get_execution_stats(&self) -> Result<HashMap<String, HookExecutionStats>, String> {
        let stats = self.execution_stats.lock().map_err(|e| e.to_string())?;
        Ok(stats.clone())
    }

    /// 清理执行统计
    pub fn clear_execution_stats(&self) -> Result<(), String> {
        let mut stats = self.execution_stats.lock().map_err(|e| e.to_string())?;
        stats.clear();
        info!("已清理钩子执行统计");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context_memory_service::ContextMemoryConfig;
    use tempfile::TempDir;

    fn create_test_services() -> (Arc<ContextMemoryService>, ToolHooksService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let memory_config = ContextMemoryConfig {
            memory_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let memory_service = Arc::new(ContextMemoryService::new(memory_config).unwrap());
        let hooks_service = ToolHooksService::new(memory_service.clone());

        (memory_service, hooks_service, temp_dir)
    }

    #[test]
    fn test_hooks_service_creation() {
        let (_memory_service, hooks_service, _temp_dir) = create_test_services();

        let rules = hooks_service.get_hook_rules().unwrap();
        assert!(!rules.is_empty()); // 应该有默认规则
    }

    #[test]
    fn test_session_start_hook() {
        let (_memory_service, hooks_service, _temp_dir) = create_test_services();

        let context = HookContext {
            session_id: "test-session".to_string(),
            tool_name: None,
            tool_parameters: None,
            tool_result: None,
            message_content: None,
            message_count: 0,
            error_info: None,
            metadata: HashMap::new(),
        };

        hooks_service
            .execute_hooks(HookTrigger::SessionStart, &context)
            .unwrap();

        // 验证任务计划是否被创建
        let memories = _memory_service
            .get_session_memories("test-session", Some(MemoryFileType::TaskPlan))
            .unwrap();
        assert!(!memories.is_empty());
    }

    #[test]
    fn test_error_recording_hook() {
        let (_memory_service, hooks_service, _temp_dir) = create_test_services();

        let context = HookContext {
            session_id: "test-session".to_string(),
            tool_name: Some("test_tool".to_string()),
            tool_parameters: None,
            tool_result: Some("发生了一个错误".to_string()),
            message_content: Some("这里有一个错误需要处理".to_string()),
            message_count: 5,
            error_info: Some("测试错误".to_string()),
            metadata: HashMap::new(),
        };

        hooks_service
            .execute_hooks(HookTrigger::PostToolUse, &context)
            .unwrap();

        // 验证错误是否被记录
        let stats = _memory_service.get_memory_stats("test-session").unwrap();
        assert!(stats.unresolved_errors > 0);
    }

    #[test]
    fn test_custom_hook_rule() {
        let (_memory_service, hooks_service, _temp_dir) = create_test_services();

        let custom_rule = HookRule {
            id: "custom-test-rule".to_string(),
            name: "自定义测试规则".to_string(),
            description: "测试自定义钩子规则".to_string(),
            trigger: HookTrigger::PostToolUse,
            conditions: vec![HookCondition::ToolNameEquals("custom_tool".to_string())],
            actions: vec![HookAction::SaveFinding {
                title: "自定义发现".to_string(),
                content: "这是一个自定义钩子触发的发现".to_string(),
                tags: vec!["自定义".to_string()],
                priority: 3,
            }],
            enabled: true,
            priority: 1,
            created_at: chrono::Utc::now().timestamp_millis(),
        };

        hooks_service.add_hook_rule(custom_rule).unwrap();

        let context = HookContext {
            session_id: "test-session".to_string(),
            tool_name: Some("custom_tool".to_string()),
            tool_parameters: None,
            tool_result: None,
            message_content: None,
            message_count: 0,
            error_info: None,
            metadata: HashMap::new(),
        };

        hooks_service
            .execute_hooks(HookTrigger::PostToolUse, &context)
            .unwrap();

        // 验证自定义发现是否被保存
        let memories = _memory_service
            .get_session_memories("test-session", Some(MemoryFileType::Findings))
            .unwrap();
        assert!(memories.iter().any(|m| m.title == "自定义发现"));
    }

    #[test]
    fn test_template_interpolation() {
        let (_memory_service, hooks_service, _temp_dir) = create_test_services();

        let context = HookContext {
            session_id: "test-session-123".to_string(),
            tool_name: Some("test_tool".to_string()),
            tool_parameters: None,
            tool_result: None,
            message_content: Some("这是测试消息".to_string()),
            message_count: 42,
            error_info: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("custom_var".to_string(), "custom_value".to_string());
                map
            },
        };

        let template = "会话 {session_id} 使用工具 {tool_name}，消息数量: {message_count}，自定义变量: {custom_var}";
        let result = hooks_service.interpolate_template(template, &context);

        assert!(result.contains("test-session-123"));
        assert!(result.contains("test_tool"));
        assert!(result.contains("42"));
        assert!(result.contains("custom_value"));
    }
}
