//! Aster 状态支持模块
//!
//! 提供可复用的会话配置构建、项目上下文 Prompt 构建、
//! ProxyCast Skills 加载与 Agent 身份配置。

use aster::agents::{AgentIdentity, SessionConfig};
use aster::skills::{global_registry, load_skills_from_directory, SkillSource};
use proxycast_core::database::DbConnection;
use proxycast_services::project_context_builder::ProjectContextBuilder;

/// 重新加载 ProxyCast Skills
pub fn reload_proxycast_skills() {
    load_proxycast_skills();
}

/// 创建 ProxyCast 专属的 Agent 身份配置
pub fn create_proxycast_identity() -> AgentIdentity {
    AgentIdentity::new("ProxyCast 助手")
        .with_language("Chinese")
        .with_description(
            "ProxyCast 是一个 AI 代理服务应用，帮助用户管理和使用各种 AI 模型的凭证。",
        )
        .with_custom_prompt(PROXYCAST_IDENTITY_PROMPT.to_string())
}

/// 加载 ProxyCast Skills 到 aster-rust 的 global_registry
fn load_proxycast_skills() {
    let home = match dirs::home_dir() {
        Some(home_dir) => home_dir,
        None => {
            tracing::warn!("[AsterAgent] 无法获取 home 目录，跳过 Skills 加载");
            return;
        }
    };

    let skills_dir = home.join(".proxycast").join("skills");
    if !skills_dir.exists() {
        tracing::info!(
            "[AsterAgent] ProxyCast Skills 目录不存在: {:?}，跳过加载",
            skills_dir
        );
        return;
    }

    let skills = load_skills_from_directory(&skills_dir, SkillSource::User);
    let skill_count = skills.len();

    if skill_count == 0 {
        tracing::info!("[AsterAgent] ProxyCast Skills 目录为空，无 Skills 可加载");
        return;
    }

    let registry = global_registry();
    if let Ok(mut registry_guard) = registry.write() {
        for skill in skills {
            let skill_name = skill.skill_name.clone();
            registry_guard.register(skill);
            tracing::debug!("[AsterAgent] 已注册 Skill: {}", skill_name);
        }
        tracing::info!(
            "[AsterAgent] 成功加载 {} 个 ProxyCast Skills 到 global_registry",
            skill_count
        );
    } else {
        tracing::error!("[AsterAgent] 无法获取 global_registry 写锁，Skills 加载失败");
    }
}

/// 构建带项目上下文的 System Prompt
pub fn build_project_system_prompt(db: &DbConnection, project_id: &str) -> Result<String, String> {
    let conn = db.lock().map_err(|e| format!("获取数据库连接失败: {e}"))?;
    let context = ProjectContextBuilder::build_context(&conn, project_id)
        .map_err(|e| format!("构建项目上下文失败: {e}"))?;
    Ok(ProjectContextBuilder::build_system_prompt(&context))
}

/// 创建带项目上下文的会话配置
pub fn create_session_config_with_project(
    db: &DbConnection,
    session_id: &str,
    project_id: Option<&str>,
) -> SessionConfig {
    let system_prompt = project_id.and_then(|pid| build_project_system_prompt(db, pid).ok());

    SessionConfigBuilder::new(session_id)
        .system_prompt(system_prompt.unwrap_or_default())
        .build()
}

/// 会话配置构建器
pub struct SessionConfigBuilder {
    id: String,
    max_turns: Option<u32>,
    system_prompt: Option<String>,
}

impl SessionConfigBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            max_turns: None,
            system_prompt: None,
        }
    }

    pub fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn build(self) -> SessionConfig {
        SessionConfig {
            id: self.id,
            schedule_id: None,
            max_turns: self.max_turns,
            retry_config: None,
            system_prompt: self.system_prompt,
        }
    }
}

/// 消息构建辅助函数
pub mod message_helpers {
    use aster::conversation::message::Message;

    /// 创建用户文本消息
    pub fn user_text(text: impl Into<String>) -> Message {
        Message::user().with_text(text)
    }

    /// 创建助手文本消息
    pub fn assistant_text(text: impl Into<String>) -> Message {
        Message::assistant().with_text(text)
    }
}

/// ProxyCast 专属的 Agent 身份提示词
const PROXYCAST_IDENTITY_PROMPT: &str = r#"你是 ProxyCast 助手，一个专业、友好的 AI 技术伙伴。

## 关于 ProxyCast

ProxyCast 是一个 AI 代理服务应用，帮助用户：
- 管理多个 AI 模型提供商的凭证（OpenAI、Claude、Gemini、Kiro 等）
- 通过统一的 API 接口访问不同的 AI 模型
- 实现凭证池的负载均衡和健康检查

## 语言规范

1. **始终使用中文回复**：除非用户明确要求使用其他语言
2. **代码注释使用中文**：生成代码时，注释应使用中文
3. **技术术语保持原文**：API、JSON、HTTP、Token 等专业术语保持英文

## 交互风格

- 简洁专业，直接给出解决方案
- 友好但不啰嗦，像经验丰富的技术伙伴
- 遇到问题时，先分析原因再提供方案
"#;
