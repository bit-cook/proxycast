//! Skill 执行 Tauri 命令模块
//!
//! 本模块提供 Skill 执行相关的 Tauri 命令，包括：
//! - `execute_skill`: 执行指定的 Skill
//! - `list_executable_skills`: 列出所有可执行的 Skills
//! - `get_skill_detail`: 获取 Skill 详情
//!
//! ## 依赖
//! - `AsterAgentState`: Aster Agent 状态管理，提供完整的工具集支持
//! - `TauriExecutionCallback`: 执行进度回调
//! - `ProviderPoolService`: 凭证池服务
//!
//! ## Requirements
//! - 3.1: execute_skill 命令接受 skill_name 和 user_input 参数
//! - 4.1: list_executable_skills 返回所有可执行的 skills
//! - 5.1: get_skill_detail 接受 skill_name 参数

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use uuid::Uuid;

use aster::conversation::message::Message;

use crate::agent::aster_state::SessionConfigBuilder;
use crate::agent::{AsterAgentState, TauriAgentEvent};
use crate::commands::skill_error::{
    format_skill_error, map_find_skill_error, SKILL_ERR_CATALOG_UNAVAILABLE,
    SKILL_ERR_EXECUTE_FAILED, SKILL_ERR_PROVIDER_UNAVAILABLE, SKILL_ERR_SESSION_INIT_FAILED,
    SKILL_ERR_STREAM_FAILED,
};
use crate::database::DbConnection;
use crate::skills::TauriExecutionCallback;
use proxycast_agent::event_converter::convert_agent_event;
use proxycast_skills::{
    find_skill_by_name, get_proxycast_skills_dir, load_skills_from_directory, ExecutionCallback,
};
#[cfg(test)]
use proxycast_skills::{
    load_skill_from_file, parse_allowed_tools, parse_boolean, parse_skill_frontmatter,
};

// ============================================================================
// 公开类型定义
// ============================================================================

/// 可执行 Skill 信息
///
/// 用于 list_executable_skills 命令的返回类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableSkillInfo {
    /// Skill 名称（唯一标识）
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// Skill 描述
    pub description: String,
    /// 执行模式：prompt, workflow, agent
    pub execution_mode: String,
    /// 是否有 workflow 定义
    pub has_workflow: bool,
    /// 指定的 Provider（可选）
    pub provider: Option<String>,
    /// 指定的 Model（可选）
    pub model: Option<String>,
    /// 参数提示（可选）
    pub argument_hint: Option<String>,
}

/// Workflow 步骤信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepInfo {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 依赖的步骤 ID 列表
    pub dependencies: Vec<String>,
}

/// Skill 详情信息
///
/// 用于 get_skill_detail 命令的返回类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetailInfo {
    /// 基本信息
    #[serde(flatten)]
    pub basic: ExecutableSkillInfo,
    /// Markdown 内容
    pub markdown_content: String,
    /// Workflow 步骤（如果有）
    pub workflow_steps: Option<Vec<WorkflowStepInfo>>,
    /// 允许的工具列表（可选）
    pub allowed_tools: Option<Vec<String>>,
    /// 使用场景说明（可选）
    pub when_to_use: Option<String>,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// 步骤 ID
    pub step_id: String,
    /// 步骤名称
    pub step_name: String,
    /// 是否成功
    pub success: bool,
    /// 输出内容
    pub output: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// Skill 执行结果
///
/// 用于 execute_skill 命令的返回类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 最终输出
    pub output: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 已完成的步骤结果
    pub steps_completed: Vec<StepResult>,
}

/// 执行 Skill
///
/// 加载并执行指定的 Skill，使用 Aster Agent 系统提供完整的工具集支持。
///
/// # Arguments
/// * `app_handle` - Tauri AppHandle，用于发送事件
/// * `db` - 数据库连接
/// * `aster_state` - Aster Agent 状态
/// * `skill_name` - Skill 名称
/// * `user_input` - 用户输入
/// * `provider_override` - 可选的 Provider 覆盖
/// * `session_id` - 可选的会话 ID（用于复用当前聊天上下文）
///
/// # Returns
/// * `Ok(SkillExecutionResult)` - 执行结果
/// * `Err(String)` - 错误信息
///
/// # Requirements
/// - 3.1: 接受 skill_name 和 user_input 参数
/// - 3.2: 从 registry 加载 skill
/// - 3.3: 使用 Aster Agent 执行（支持工具调用）
/// - 3.5: 返回 SkillExecutionResult
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn execute_skill(
    app_handle: tauri::AppHandle,
    db: State<'_, DbConnection>,
    aster_state: State<'_, AsterAgentState>,
    skill_name: String,
    user_input: String,
    provider_override: Option<String>,
    model_override: Option<String>,
    execution_id: Option<String>,
    session_id: Option<String>,
) -> Result<SkillExecutionResult, String> {
    // 生成执行 ID，并优先复用前端会话 ID（提升 /skill 与主会话上下文一致性）
    let execution_id = execution_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let session_id = session_id.unwrap_or_else(|| format!("skill-exec-{}", Uuid::new_v4()));

    tracing::info!(
        "[execute_skill] 开始执行 Skill: name={}, execution_id={}, session_id={}, provider_override={:?}, model_override={:?}",
        skill_name,
        execution_id,
        session_id,
        provider_override,
        model_override
    );

    // 1. 从 registry 加载 skill（Requirements 3.2）
    let skill = find_skill_by_name(&skill_name).map_err(map_find_skill_error)?;

    // 检查是否禁用了模型调用
    if skill.disable_model_invocation {
        return Err(format_skill_error(
            SKILL_ERR_EXECUTE_FAILED,
            format!("Skill '{skill_name}' 已禁用模型调用，无法执行"),
        ));
    }

    // 2. 创建 TauriExecutionCallback
    let callback = TauriExecutionCallback::new(app_handle.clone(), execution_id.clone());

    // 3. 初始化 Agent（如果未初始化）
    if !aster_state.is_initialized().await {
        tracing::info!("[execute_skill] Agent 未初始化，开始初始化...");
        aster_state.init_agent_with_db(&db).await.map_err(|e| {
            format_skill_error(
                SKILL_ERR_SESSION_INIT_FAILED,
                format!("初始化 Agent 失败: {e}"),
            )
        })?;
        tracing::info!("[execute_skill] Agent 初始化完成");
    }

    // 4. 配置 Provider（从凭证池选择，支持 fallback）
    let preferred_provider = provider_override
        .or_else(|| skill.provider.clone())
        .unwrap_or_else(|| "anthropic".to_string());

    let preferred_model = model_override
        .or_else(|| skill.model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    // 支持工具调用的 Provider fallback 列表
    // 注意：provider 名称需要与 ProviderType::FromStr 匹配
    let fallback_providers: Vec<(&str, &str)> = vec![
        ("anthropic", "claude-sonnet-4-20250514"),
        ("openai", "gpt-4o"),
        ("gemini", "gemini-2.0-flash"),
    ];

    let mut configure_result = aster_state
        .configure_provider_from_pool(&db, &preferred_provider, &preferred_model, &session_id)
        .await;

    if configure_result.is_err() {
        tracing::warn!(
            "[execute_skill] 首选 Provider {} 配置失败: {:?}，尝试 fallback",
            preferred_provider,
            configure_result.as_ref().err()
        );

        for (fb_provider, fb_model) in &fallback_providers {
            if *fb_provider == preferred_provider {
                continue;
            }
            match aster_state
                .configure_provider_from_pool(&db, fb_provider, fb_model, &session_id)
                .await
            {
                Ok(config) => {
                    tracing::info!(
                        "[execute_skill] Fallback 到 {} / {} 成功",
                        fb_provider,
                        fb_model
                    );
                    configure_result = Ok(config);
                    break;
                }
                Err(e) => {
                    tracing::warn!("[execute_skill] Fallback {} 也失败: {}", fb_provider, e);
                }
            }
        }
    }

    configure_result.map_err(|e| {
        format_skill_error(
            SKILL_ERR_PROVIDER_UNAVAILABLE,
            format!(
                "无法配置任何可用的 Provider（需要支持工具调用的 Provider，如 Anthropic、OpenAI 或 Google）: {e}"
            ),
        )
    })?;

    tracing::info!(
        "[execute_skill] Provider 配置成功: preferred={}, model={}",
        preferred_provider,
        preferred_model
    );

    // 5. 根据 execution_mode 分支执行
    if skill.execution_mode == "workflow" && !skill.workflow_steps.is_empty() {
        // ========== Workflow 模式：按步骤顺序执行 ==========
        execute_skill_workflow(
            &app_handle,
            &aster_state,
            &skill,
            &user_input,
            &execution_id,
            &session_id,
            &callback,
        )
        .await
    } else {
        // ========== Prompt 模式：单次执行 ==========
        execute_skill_prompt(
            &app_handle,
            &aster_state,
            &skill,
            &user_input,
            &execution_id,
            &session_id,
            &callback,
        )
        .await
    }
}

/// Prompt 模式执行（单步）
async fn execute_skill_prompt(
    app_handle: &tauri::AppHandle,
    aster_state: &AsterAgentState,
    skill: &proxycast_skills::LoadedSkillDefinition,
    user_input: &str,
    execution_id: &str,
    session_id: &str,
    callback: &TauriExecutionCallback,
) -> Result<SkillExecutionResult, String> {
    // 发送步骤开始事件
    callback.on_step_start("main", &skill.display_name, 1, 1);

    // 构建 SessionConfig
    let session_config = SessionConfigBuilder::new(session_id)
        .system_prompt(&skill.markdown_content)
        .build();

    let user_message = Message::user().with_text(user_input);

    // 获取 Agent 并执行
    let agent_arc = aster_state.get_agent_arc();
    let guard = agent_arc.read().await;
    let agent = guard.as_ref().ok_or_else(|| {
        format_skill_error(SKILL_ERR_SESSION_INIT_FAILED, "Agent not initialized")
    })?;

    let cancel_token = aster_state.create_cancel_token(session_id).await;
    let stream_result = agent
        .reply(user_message, session_config, Some(cancel_token.clone()))
        .await;

    let mut final_output = String::new();
    let mut has_error = false;
    let mut error_message: Option<String> = None;
    let event_name = format!("skill-exec-{execution_id}");

    match stream_result {
        Ok(mut stream) => {
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(agent_event) => {
                        let tauri_events = convert_agent_event(agent_event);
                        for tauri_event in tauri_events {
                            if let TauriAgentEvent::TextDelta { ref text } = tauri_event {
                                final_output.push_str(text);
                            }
                            if let Err(e) = app_handle.emit(&event_name, &tauri_event) {
                                tracing::error!("[execute_skill] 发送事件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        has_error = true;
                        error_message = Some(format_skill_error(
                            SKILL_ERR_STREAM_FAILED,
                            format!("Stream error: {e}"),
                        ));
                        tracing::error!("[execute_skill] 流处理错误: {}", e);
                    }
                }
            }

            let done_event = TauriAgentEvent::FinalDone { usage: None };
            if let Err(e) = app_handle.emit(&event_name, &done_event) {
                tracing::error!("[execute_skill] 发送完成事件失败: {}", e);
            }
        }
        Err(e) => {
            has_error = true;
            error_message = Some(format_skill_error(
                SKILL_ERR_STREAM_FAILED,
                format!("Agent error: {e}"),
            ));
            tracing::error!("[execute_skill] Agent 错误: {}", e);
        }
    }

    aster_state.remove_cancel_token(session_id).await;

    if has_error {
        let err_msg = error_message
            .unwrap_or_else(|| format_skill_error(SKILL_ERR_EXECUTE_FAILED, "Unknown error"));
        callback.on_step_error("main", &err_msg, false);
        callback.on_complete(false, None, Some(&err_msg));

        Ok(SkillExecutionResult {
            success: false,
            output: None,
            error: Some(err_msg.clone()),
            steps_completed: vec![StepResult {
                step_id: "main".to_string(),
                step_name: skill.display_name.clone(),
                success: false,
                output: None,
                error: Some(err_msg),
            }],
        })
    } else {
        callback.on_step_complete("main", &final_output);
        callback.on_complete(true, Some(&final_output), None);

        Ok(SkillExecutionResult {
            success: true,
            output: Some(final_output.clone()),
            error: None,
            steps_completed: vec![StepResult {
                step_id: "main".to_string(),
                step_name: skill.display_name.clone(),
                success: true,
                output: Some(final_output),
                error: None,
            }],
        })
    }
}

/// Workflow 模式执行（多步骤顺序执行）
async fn execute_skill_workflow(
    app_handle: &tauri::AppHandle,
    aster_state: &AsterAgentState,
    skill: &proxycast_skills::LoadedSkillDefinition,
    user_input: &str,
    execution_id: &str,
    session_id: &str,
    callback: &TauriExecutionCallback,
) -> Result<SkillExecutionResult, String> {
    let steps = &skill.workflow_steps;
    let total_steps = steps.len();
    let event_name = format!("skill-exec-{execution_id}");
    let mut steps_completed = Vec::new();
    let mut accumulated_context = user_input.to_string();
    let mut final_output = String::new();

    tracing::info!(
        "[execute_skill_workflow] 开始 workflow 执行: steps={}, skill={}",
        total_steps,
        skill.skill_name
    );

    for (idx, step) in steps.iter().enumerate() {
        let step_num = idx + 1;

        // 发送步骤开始事件
        callback.on_step_start(&step.id, &step.name, step_num, total_steps);

        tracing::info!(
            "[execute_skill_workflow] 执行步骤 {}/{}: id={}, name={}",
            step_num,
            total_steps,
            step.id,
            step.name
        );

        // 构建该步骤的 system_prompt：基础 skill prompt + 步骤 prompt
        let step_system_prompt = format!(
            "{}\n\n---\n\n## 当前步骤: {} ({}/{})\n\n{}",
            skill.markdown_content, step.name, step_num, total_steps, step.prompt
        );

        let step_session_id = format!("{}-step-{}", session_id, step.id);
        let session_config = SessionConfigBuilder::new(&step_session_id)
            .system_prompt(&step_system_prompt)
            .build();

        // 用户消息 = 原始输入 + 前序步骤的累积上下文
        let step_input = if idx == 0 {
            accumulated_context.clone()
        } else {
            format!("原始需求：{user_input}\n\n前序步骤输出：\n{accumulated_context}")
        };

        let user_message = Message::user().with_text(&step_input);

        // 获取 Agent 并执行
        let agent_arc = aster_state.get_agent_arc();
        let guard = agent_arc.read().await;
        let agent = guard.as_ref().ok_or_else(|| {
            format_skill_error(SKILL_ERR_SESSION_INIT_FAILED, "Agent not initialized")
        })?;

        let cancel_token = aster_state.create_cancel_token(&step_session_id).await;
        let stream_result = agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await;

        let mut step_output = String::new();
        let mut step_error: Option<String> = None;

        match stream_result {
            Ok(mut stream) => {
                while let Some(event_result) = stream.next().await {
                    match event_result {
                        Ok(agent_event) => {
                            let tauri_events = convert_agent_event(agent_event);
                            for tauri_event in tauri_events {
                                if let TauriAgentEvent::TextDelta { ref text } = tauri_event {
                                    step_output.push_str(text);
                                }
                                if let Err(e) = app_handle.emit(&event_name, &tauri_event) {
                                    tracing::error!("[execute_skill_workflow] 发送事件失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            step_error = Some(format!("Stream error: {e}"));
                            tracing::error!(
                                "[execute_skill_workflow] 步骤 {} 流处理错误: {}",
                                step.id,
                                e
                            );
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                step_error = Some(format!("Agent error: {e}"));
                tracing::error!(
                    "[execute_skill_workflow] 步骤 {} Agent 错误: {}",
                    step.id,
                    e
                );
            }
        }

        aster_state.remove_cancel_token(&step_session_id).await;

        if let Some(err) = &step_error {
            callback.on_step_error(&step.id, err, false);
            steps_completed.push(StepResult {
                step_id: step.id.clone(),
                step_name: step.name.clone(),
                success: false,
                output: None,
                error: Some(err.clone()),
            });

            // 步骤失败，终止 workflow
            let err_msg = format_skill_error(
                SKILL_ERR_EXECUTE_FAILED,
                format!("步骤 '{}' 执行失败: {}", step.name, err),
            );
            callback.on_complete(false, None, Some(&err_msg));

            let done_event = TauriAgentEvent::FinalDone { usage: None };
            let _ = app_handle.emit(&event_name, &done_event);

            return Ok(SkillExecutionResult {
                success: false,
                output: None,
                error: Some(err_msg),
                steps_completed,
            });
        }

        // 步骤成功
        callback.on_step_complete(&step.id, &step_output);
        steps_completed.push(StepResult {
            step_id: step.id.clone(),
            step_name: step.name.clone(),
            success: true,
            output: Some(step_output.clone()),
            error: None,
        });

        // 累积上下文供下一步使用
        accumulated_context = step_output.clone();
        final_output = step_output;
    }

    // 所有步骤完成
    callback.on_complete(true, Some(&final_output), None);

    let done_event = TauriAgentEvent::FinalDone { usage: None };
    let _ = app_handle.emit(&event_name, &done_event);

    tracing::info!(
        "[execute_skill_workflow] Workflow 执行完成: skill={}, steps_completed={}",
        skill.skill_name,
        steps_completed.len()
    );

    Ok(SkillExecutionResult {
        success: true,
        output: Some(final_output),
        error: None,
        steps_completed,
    })
}

/// 列出可执行的 Skills
///
/// 返回所有可以执行的 Skills 列表，过滤掉 disable_model_invocation=true 的 Skills。
///
/// # Returns
/// * `Ok(Vec<ExecutableSkillInfo>)` - 可执行的 Skills 列表
/// * `Err(String)` - 错误信息
///
/// # Requirements
/// - 4.1: 返回所有可执行的 skills
/// - 4.2: 包含 name, description, execution_mode
/// - 4.3: 指示是否有 workflow 定义
/// - 4.4: 过滤 disable_model_invocation=true 的 skills
#[tauri::command]
pub async fn list_executable_skills() -> Result<Vec<ExecutableSkillInfo>, String> {
    let skills_dir = get_proxycast_skills_dir()
        .ok_or_else(|| format_skill_error(SKILL_ERR_CATALOG_UNAVAILABLE, "无法获取 Skills 目录"))?;

    // 加载所有 skills
    let all_skills = load_skills_from_directory(&skills_dir);

    // 过滤掉 disable_model_invocation=true 的 skills（Requirements 4.4）
    let executable_skills: Vec<ExecutableSkillInfo> = all_skills
        .into_iter()
        .filter(|s| !s.disable_model_invocation)
        .map(|s| ExecutableSkillInfo {
            name: s.skill_name,
            display_name: s.display_name,
            description: s.description,
            execution_mode: s.execution_mode.clone(),
            has_workflow: s.execution_mode == "workflow",
            provider: s.provider,
            model: s.model,
            argument_hint: s.argument_hint,
        })
        .collect();

    tracing::info!(
        "[list_executable_skills] 返回 {} 个可执行 Skills",
        executable_skills.len()
    );

    Ok(executable_skills)
}

/// 获取 Skill 详情
///
/// 根据 skill_name 返回完整的 Skill 详情信息。
///
/// # Arguments
/// * `skill_name` - Skill 名称
///
/// # Returns
/// * `Ok(SkillDetailInfo)` - Skill 详情
/// * `Err(String)` - 错误信息（如 skill 不存在）
///
/// # Requirements
/// - 5.1: 接受 skill_name 参数
/// - 5.2: 返回完整的 SkillDefinition
/// - 5.3: 包含 workflow steps 信息（如果有）
/// - 5.4: skill 不存在时返回错误
#[tauri::command]
pub async fn get_skill_detail(skill_name: String) -> Result<SkillDetailInfo, String> {
    // 查找 skill（Requirements 5.1, 5.4）
    let skill = find_skill_by_name(&skill_name).map_err(map_find_skill_error)?;

    // 转换为 SkillDetailInfo（Requirements 5.2, 5.3）
    let detail = SkillDetailInfo {
        basic: ExecutableSkillInfo {
            name: skill.skill_name,
            display_name: skill.display_name,
            description: skill.description,
            execution_mode: skill.execution_mode.clone(),
            has_workflow: skill.execution_mode == "workflow",
            provider: skill.provider,
            model: skill.model,
            argument_hint: skill.argument_hint,
        },
        markdown_content: skill.markdown_content,
        workflow_steps: if skill.workflow_steps.is_empty() {
            None
        } else {
            Some(
                skill
                    .workflow_steps
                    .iter()
                    .map(|s| WorkflowStepInfo {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        dependencies: Vec::new(),
                    })
                    .collect(),
            )
        },
        allowed_tools: skill.allowed_tools,
        when_to_use: skill.when_to_use,
    };

    tracing::info!("[get_skill_detail] 返回 Skill 详情: name={}", skill_name);

    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executable_skill_info_serialization() {
        let info = ExecutableSkillInfo {
            name: "test-skill".to_string(),
            display_name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            execution_mode: "prompt".to_string(),
            has_workflow: false,
            provider: None,
            model: None,
            argument_hint: Some("Enter your query".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-skill"));
        assert!(json.contains("Test Skill"));
    }

    #[test]
    fn test_skill_execution_result_serialization() {
        let result = SkillExecutionResult {
            success: true,
            output: Some("Hello, world!".to_string()),
            error: None,
            steps_completed: vec![StepResult {
                step_id: "step-1".to_string(),
                step_name: "Process".to_string(),
                success: true,
                output: Some("Done".to_string()),
                error: None,
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("Hello, world!"));
        assert!(json.contains("step-1"));
    }

    #[test]
    fn test_skill_detail_info_serialization() {
        let detail = SkillDetailInfo {
            basic: ExecutableSkillInfo {
                name: "workflow-skill".to_string(),
                display_name: "Workflow Skill".to_string(),
                description: "A workflow skill".to_string(),
                execution_mode: "workflow".to_string(),
                has_workflow: true,
                provider: Some("claude".to_string()),
                model: Some("claude-sonnet-4-5-20250514".to_string()),
                argument_hint: None,
            },
            markdown_content: "# Workflow Skill\n\nThis is a workflow skill.".to_string(),
            workflow_steps: Some(vec![
                WorkflowStepInfo {
                    id: "step-1".to_string(),
                    name: "Initialize".to_string(),
                    dependencies: vec![],
                },
                WorkflowStepInfo {
                    id: "step-2".to_string(),
                    name: "Process".to_string(),
                    dependencies: vec!["step-1".to_string()],
                },
            ]),
            allowed_tools: Some(vec!["read_file".to_string(), "write_file".to_string()]),
            when_to_use: Some("Use this skill for complex workflows".to_string()),
        };

        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("workflow-skill"));
        assert!(json.contains("workflow_steps"));
        assert!(json.contains("step-1"));
        assert!(json.contains("step-2"));
    }

    #[test]
    fn test_parse_skill_frontmatter_basic() {
        let content = r#"---
name: test-skill
description: A test skill
model: claude-sonnet-4-5-20250514
provider: claude
---

# Test Skill

This is the body content.
"#;
        let (fm, body) = parse_skill_frontmatter(content);
        assert_eq!(fm.name, Some("test-skill".to_string()));
        assert_eq!(fm.description, Some("A test skill".to_string()));
        assert_eq!(fm.model, Some("claude-sonnet-4-5-20250514".to_string()));
        assert_eq!(fm.provider, Some("claude".to_string()));
        assert!(body.contains("# Test Skill"));
        assert!(body.contains("This is the body content."));
    }

    #[test]
    fn test_parse_skill_frontmatter_no_frontmatter() {
        let content = "# Just content\nNo frontmatter here.";
        let (fm, body) = parse_skill_frontmatter(content);
        assert!(fm.name.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_skill_frontmatter_with_quotes() {
        let content = r#"---
name: "quoted-name"
description: 'single quoted'
---
Body
"#;
        let (fm, _) = parse_skill_frontmatter(content);
        assert_eq!(fm.name, Some("quoted-name".to_string()));
        assert_eq!(fm.description, Some("single quoted".to_string()));
    }

    #[test]
    fn test_parse_allowed_tools() {
        assert_eq!(parse_allowed_tools(None), None);
        assert_eq!(parse_allowed_tools(Some("")), None);
        assert_eq!(
            parse_allowed_tools(Some("tool1")),
            Some(vec!["tool1".to_string()])
        );
        assert_eq!(
            parse_allowed_tools(Some("tool1, tool2, tool3")),
            Some(vec![
                "tool1".to_string(),
                "tool2".to_string(),
                "tool3".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_boolean() {
        assert!(!parse_boolean(None, false));
        assert!(parse_boolean(None, true));
        assert!(parse_boolean(Some("true"), false));
        assert!(parse_boolean(Some("TRUE"), false));
        assert!(parse_boolean(Some("1"), false));
        assert!(parse_boolean(Some("yes"), false));
        assert!(!parse_boolean(Some("false"), true));
        assert!(!parse_boolean(Some("no"), true));
    }

    #[test]
    fn test_load_skill_from_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("my-skill");
        std::fs::create_dir(&skill_dir).unwrap();

        let skill_file = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_file,
            r#"---
name: my-skill
description: Test skill description
allowed-tools: tool1, tool2
model: gpt-4
provider: openai
---

# My Skill

Instructions here.
"#,
        )
        .unwrap();

        let skill = load_skill_from_file("my-skill", &skill_file).unwrap();

        assert_eq!(skill.skill_name, "my-skill");
        assert_eq!(skill.display_name, "my-skill");
        assert_eq!(skill.description, "Test skill description");
        assert_eq!(
            skill.allowed_tools,
            Some(vec!["tool1".to_string(), "tool2".to_string()])
        );
        assert_eq!(skill.model, Some("gpt-4".to_string()));
        assert_eq!(skill.provider, Some("openai".to_string()));
        assert!(!skill.disable_model_invocation);
        assert_eq!(skill.execution_mode, "prompt");
    }

    #[test]
    fn test_load_skills_from_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path();

        // 创建 skill 1
        let skill1_dir = skills_dir.join("skill-one");
        std::fs::create_dir(&skill1_dir).unwrap();
        std::fs::write(
            skill1_dir.join("SKILL.md"),
            r#"---
name: skill-one
description: First skill
---
Content 1
"#,
        )
        .unwrap();

        // 创建 skill 2
        let skill2_dir = skills_dir.join("skill-two");
        std::fs::create_dir(&skill2_dir).unwrap();
        std::fs::write(
            skill2_dir.join("SKILL.md"),
            r#"---
name: skill-two
description: Second skill
disable-model-invocation: true
---
Content 2
"#,
        )
        .unwrap();

        let skills = load_skills_from_directory(skills_dir);

        assert_eq!(skills.len(), 2);
        let names: Vec<_> = skills.iter().map(|s| s.skill_name.as_str()).collect();
        assert!(names.contains(&"skill-one"));
        assert!(names.contains(&"skill-two"));

        // 验证 disable_model_invocation 被正确解析
        let skill_two = skills.iter().find(|s| s.skill_name == "skill-two").unwrap();
        assert!(skill_two.disable_model_invocation);
    }

    #[test]
    fn test_load_skills_from_nonexistent_directory() {
        let skills = load_skills_from_directory(std::path::Path::new("/nonexistent/path"));
        assert!(skills.is_empty());
    }
}
