//! 任务模板定义
//!
//! 定义可复用的任务模板

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 任务模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    /// 模板 ID
    pub id: Uuid,

    /// 模板名称
    pub name: String,

    /// 模板描述
    pub description: Option<String>,

    /// 模型名称
    pub model: String,

    /// 系统提示词
    pub system_prompt: Option<String>,

    /// 用户消息模板 (支持变量替换,例如 "{{variable_name}}")
    pub user_message_template: String,

    /// 温度参数
    pub temperature: Option<f32>,

    /// 最大 tokens
    pub max_tokens: Option<u32>,

    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TaskTemplate {
    /// 创建新的任务模板
    pub fn new(name: String, model: String, user_message_template: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            model,
            system_prompt: None,
            user_message_template,
            temperature: None,
            max_tokens: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, system_prompt: String) -> Self {
        self.system_prompt = Some(system_prompt);
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置最大 tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 渲染用户消息 (替换变量)
    pub fn render_user_message(&self, variables: &HashMap<String, String>) -> String {
        let mut message = self.user_message_template.clone();

        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            message = message.replace(&placeholder, value);
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let template = TaskTemplate::new(
            "测试模板".to_string(),
            "gpt-4".to_string(),
            "请处理: {{content}}".to_string(),
        )
        .with_description("这是一个测试模板".to_string())
        .with_temperature(0.7)
        .with_max_tokens(1000);

        assert_eq!(template.name, "测试模板");
        assert_eq!(template.model, "gpt-4");
        assert_eq!(template.temperature, Some(0.7));
        assert_eq!(template.max_tokens, Some(1000));
    }

    #[test]
    fn test_render_user_message() {
        let template = TaskTemplate::new(
            "测试模板".to_string(),
            "gpt-4".to_string(),
            "请处理内容: {{content}}, 来自: {{source}}".to_string(),
        );

        let mut variables = HashMap::new();
        variables.insert("content".to_string(), "测试内容".to_string());
        variables.insert("source".to_string(), "测试来源".to_string());

        let rendered = template.render_user_message(&variables);
        assert_eq!(rendered, "请处理内容: 测试内容, 来自: 测试来源");
    }
}
