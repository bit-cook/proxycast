//! 工具钩子管理相关的 Tauri 命令

use crate::services::tool_hooks_service::{
    HookAction, HookCondition, HookContext, HookExecutionStats, HookRule, HookTrigger,
    ToolHooksService,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, info};

pub struct ToolHooksServiceState(pub Arc<ToolHooksService>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteHooksRequest {
    pub trigger: HookTrigger,
    pub context: HookContextData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContextData {
    pub session_id: String,
    pub tool_name: Option<String>,
    pub tool_parameters: Option<HashMap<String, String>>,
    pub tool_result: Option<String>,
    pub message_content: Option<String>,
    pub message_count: usize,
    pub error_info: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl From<HookContextData> for HookContext {
    fn from(data: HookContextData) -> Self {
        Self {
            session_id: data.session_id,
            tool_name: data.tool_name,
            tool_parameters: data.tool_parameters,
            tool_result: data.tool_result,
            message_content: data.message_content,
            message_count: data.message_count,
            error_info: data.error_info,
            metadata: data.metadata,
        }
    }
}

#[tauri::command]
pub async fn execute_hooks(
    hooks_service: State<'_, ToolHooksServiceState>,
    request: ExecuteHooksRequest,
) -> Result<(), String> {
    debug!(
        "执行钩子: {:?} (会话: {})",
        request.trigger, request.context.session_id
    );
    let context: HookContext = request.context.into();
    hooks_service.0.execute_hooks(request.trigger, &context)?;
    info!("钩子执行完成");
    Ok(())
}

#[tauri::command]
pub async fn add_hook_rule(
    hooks_service: State<'_, ToolHooksServiceState>,
    rule: HookRule,
) -> Result<(), String> {
    debug!("添加钩子规则: {}", rule.name);
    hooks_service.0.add_hook_rule(rule.clone())?;
    info!("钩子规则添加成功: {}", rule.name);
    Ok(())
}

#[tauri::command]
pub async fn remove_hook_rule(
    hooks_service: State<'_, ToolHooksServiceState>,
    rule_id: String,
) -> Result<(), String> {
    debug!("移除钩子规则: {}", rule_id);
    hooks_service.0.remove_hook_rule(&rule_id)?;
    info!("钩子规则移除成功: {}", rule_id);
    Ok(())
}

#[tauri::command]
pub async fn toggle_hook_rule(
    hooks_service: State<'_, ToolHooksServiceState>,
    rule_id: String,
    enabled: bool,
) -> Result<(), String> {
    debug!("切换钩子规则状态: {} -> {}", rule_id, enabled);
    hooks_service.0.toggle_hook_rule(&rule_id, enabled)?;
    info!("钩子规则状态切换成功: {} -> {}", rule_id, enabled);
    Ok(())
}

#[tauri::command]
pub async fn get_hook_rules(
    hooks_service: State<'_, ToolHooksServiceState>,
) -> Result<Vec<HookRule>, String> {
    debug!("获取所有钩子规则");
    let rules = hooks_service.0.get_hook_rules()?;
    info!("获取到 {} 个钩子规则", rules.len());
    Ok(rules)
}

#[tauri::command]
pub async fn get_hook_execution_stats(
    hooks_service: State<'_, ToolHooksServiceState>,
) -> Result<HashMap<String, HookExecutionStats>, String> {
    debug!("获取钩子执行统计");
    let stats = hooks_service.0.get_execution_stats()?;
    info!("获取到 {} 个规则的执行统计", stats.len());
    Ok(stats)
}

#[tauri::command]
pub async fn clear_hook_execution_stats(
    hooks_service: State<'_, ToolHooksServiceState>,
) -> Result<(), String> {
    debug!("清理钩子执行统计");
    hooks_service.0.clear_execution_stats()?;
    info!("钩子执行统计清理完成");
    Ok(())
}
