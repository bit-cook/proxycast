//! 工作流服务
//!
//! 管理内容创作工作流的状态和生命周期

use super::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// 工作流服务
pub struct WorkflowService {
    /// 活跃的工作流（内存缓存）
    workflows: Arc<RwLock<HashMap<String, WorkflowState>>>,
}

impl WorkflowService {
    /// 创建新的工作流服务
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新工作流
    pub async fn create_workflow(
        &self,
        theme: ThemeType,
        mode: CreationMode,
    ) -> Result<WorkflowState> {
        let workflow_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();

        // 根据主题和模式生成步骤
        let steps = self.generate_steps(&theme, &mode);

        let workflow = WorkflowState {
            id: workflow_id.clone(),
            theme,
            mode,
            steps,
            current_step_index: 0,
            created_at: now,
            updated_at: now,
        };

        // 缓存工作流
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow.clone());

        info!("创建工作流: {}", workflow_id);
        Ok(workflow)
    }

    /// 获取工作流
    pub async fn get_workflow(&self, workflow_id: &str) -> Option<WorkflowState> {
        let workflows = self.workflows.read().await;
        workflows.get(workflow_id).cloned()
    }

    /// 更新工作流
    pub async fn update_workflow(&self, workflow: WorkflowState) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        let workflow_id = workflow.id.clone();
        workflows.insert(workflow_id.clone(), workflow);
        debug!("更新工作流: {}", workflow_id);
        Ok(())
    }

    /// 完成当前步骤
    pub async fn complete_step(
        &self,
        workflow_id: &str,
        result: StepResult,
    ) -> Result<WorkflowState> {
        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("工作流不存在: {workflow_id}"))?;

        let current_index = workflow.current_step_index;
        if current_index >= workflow.steps.len() {
            return Err(anyhow::anyhow!("已完成所有步骤"));
        }

        // 更新当前步骤状态
        workflow.steps[current_index].status = StepStatus::Completed;
        workflow.steps[current_index].result = Some(result);

        // 自动进入下一步
        if workflow.steps[current_index]
            .definition
            .behavior
            .auto_advance
        {
            let next_index = current_index + 1;
            if next_index < workflow.steps.len() {
                workflow.current_step_index = next_index;
                workflow.steps[next_index].status = StepStatus::Active;
            }
        }

        workflow.updated_at = chrono::Utc::now().timestamp_millis();

        info!("完成步骤 {} / {}", current_index + 1, workflow.steps.len());
        Ok(workflow.clone())
    }

    /// 跳过当前步骤
    pub async fn skip_step(&self, workflow_id: &str) -> Result<WorkflowState> {
        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("工作流不存在: {workflow_id}"))?;

        let current_index = workflow.current_step_index;
        if current_index >= workflow.steps.len() {
            return Err(anyhow::anyhow!("已完成所有步骤"));
        }

        // 检查是否可跳过
        if !workflow.steps[current_index].definition.behavior.skippable {
            return Err(anyhow::anyhow!("当前步骤不可跳过"));
        }

        // 更新状态
        workflow.steps[current_index].status = StepStatus::Skipped;

        // 进入下一步
        let next_index = current_index + 1;
        if next_index < workflow.steps.len() {
            workflow.current_step_index = next_index;
            workflow.steps[next_index].status = StepStatus::Active;
        }

        workflow.updated_at = chrono::Utc::now().timestamp_millis();

        info!("跳过步骤 {}", current_index + 1);
        Ok(workflow.clone())
    }

    /// 重做指定步骤
    pub async fn redo_step(&self, workflow_id: &str, step_index: usize) -> Result<WorkflowState> {
        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("工作流不存在: {workflow_id}"))?;

        if step_index >= workflow.steps.len() {
            return Err(anyhow::anyhow!("步骤索引无效"));
        }

        // 检查是否可重做
        if !workflow.steps[step_index].definition.behavior.redoable {
            return Err(anyhow::anyhow!("该步骤不可重做"));
        }

        // 重置该步骤及之后的所有步骤
        for i in step_index..workflow.steps.len() {
            if i == step_index {
                workflow.steps[i].status = StepStatus::Active;
            } else {
                workflow.steps[i].status = StepStatus::Pending;
            }
            workflow.steps[i].result = None;
        }

        workflow.current_step_index = step_index;
        workflow.updated_at = chrono::Utc::now().timestamp_millis();

        info!("重做步骤 {}", step_index + 1);
        Ok(workflow.clone())
    }

    /// 跳转到指定步骤（仅限已完成的步骤）
    pub async fn go_to_step(&self, workflow_id: &str, step_index: usize) -> Result<WorkflowState> {
        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("工作流不存在: {workflow_id}"))?;

        if step_index >= workflow.steps.len() {
            return Err(anyhow::anyhow!("步骤索引无效"));
        }

        // 只能跳转到已完成或已跳过的步骤
        let target_status = &workflow.steps[step_index].status;
        if *target_status != StepStatus::Completed && *target_status != StepStatus::Skipped {
            return Err(anyhow::anyhow!("只能跳转到已完成的步骤"));
        }

        workflow.current_step_index = step_index;
        workflow.updated_at = chrono::Utc::now().timestamp_millis();

        debug!("跳转到步骤 {}", step_index + 1);
        Ok(workflow.clone())
    }

    /// 删除工作流
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        workflows.remove(workflow_id);
        info!("删除工作流: {}", workflow_id);
        Ok(())
    }

    /// 根据主题和模式生成步骤定义
    fn generate_steps(&self, theme: &ThemeType, mode: &CreationMode) -> Vec<WorkflowStep> {
        // 通用对话不需要工作流
        if *theme == ThemeType::General {
            return vec![];
        }

        let base_steps = self.get_base_steps();

        // 根据模式调整步骤行为
        let steps: Vec<WorkflowStep> = base_steps
            .into_iter()
            .enumerate()
            .map(|(i, mut step)| {
                // 根据模式调整可跳过性
                if *mode == CreationMode::Fast
                    && (step.definition.step_type == StepType::Research
                        || step.definition.step_type == StepType::Polish)
                {
                    step.definition.behavior.skippable = true;
                }

                // 第一个步骤设为 Active
                if i == 0 {
                    step.status = StepStatus::Active;
                }

                step
            })
            .collect();

        steps
    }

    /// 获取基础步骤定义
    fn get_base_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep {
                definition: StepDefinition {
                    id: "clarify".to_string(),
                    step_type: StepType::Clarify,
                    title: "明确需求".to_string(),
                    description: Some("确认创作主题、目标读者和风格".to_string()),
                    form: Some(FormConfig {
                        fields: vec![
                            FormField {
                                name: "topic".to_string(),
                                label: "内容主题".to_string(),
                                field_type: FormFieldType::Text,
                                required: true,
                                placeholder: Some("请输入你想创作的主题".to_string()),
                                options: None,
                                default_value: None,
                            },
                            FormField {
                                name: "audience".to_string(),
                                label: "目标读者".to_string(),
                                field_type: FormFieldType::Select,
                                required: false,
                                placeholder: None,
                                options: Some(vec![
                                    FormFieldOption {
                                        label: "普通大众".to_string(),
                                        value: "general".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "专业人士".to_string(),
                                        value: "professional".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "学生群体".to_string(),
                                        value: "student".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "技术开发者".to_string(),
                                        value: "developer".to_string(),
                                    },
                                ]),
                                default_value: None,
                            },
                            FormField {
                                name: "style".to_string(),
                                label: "内容风格".to_string(),
                                field_type: FormFieldType::Radio,
                                required: false,
                                placeholder: None,
                                options: Some(vec![
                                    FormFieldOption {
                                        label: "专业严谨".to_string(),
                                        value: "professional".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "轻松活泼".to_string(),
                                        value: "casual".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "深度分析".to_string(),
                                        value: "analytical".to_string(),
                                    },
                                    FormFieldOption {
                                        label: "故事叙述".to_string(),
                                        value: "narrative".to_string(),
                                    },
                                ]),
                                default_value: None,
                            },
                        ],
                        submit_label: "确认并继续".to_string(),
                        skip_label: None,
                    }),
                    ai_task: None,
                    behavior: StepBehavior {
                        skippable: false,
                        redoable: true,
                        auto_advance: true,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
            WorkflowStep {
                definition: StepDefinition {
                    id: "research".to_string(),
                    step_type: StepType::Research,
                    title: "调研收集".to_string(),
                    description: Some("AI 搜索相关资料，你可以补充真实经历".to_string()),
                    form: None,
                    ai_task: Some(AITaskConfig {
                        task_type: "research".to_string(),
                        prompt: None,
                        streaming: true,
                    }),
                    behavior: StepBehavior {
                        skippable: true,
                        redoable: true,
                        auto_advance: false,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
            WorkflowStep {
                definition: StepDefinition {
                    id: "outline".to_string(),
                    step_type: StepType::Outline,
                    title: "生成大纲".to_string(),
                    description: Some("AI 生成内容大纲，你可以调整顺序".to_string()),
                    form: None,
                    ai_task: Some(AITaskConfig {
                        task_type: "outline".to_string(),
                        prompt: None,
                        streaming: true,
                    }),
                    behavior: StepBehavior {
                        skippable: false,
                        redoable: true,
                        auto_advance: false,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
            WorkflowStep {
                definition: StepDefinition {
                    id: "write".to_string(),
                    step_type: StepType::Write,
                    title: "撰写内容".to_string(),
                    description: Some("根据模式不同，AI 和你协作完成内容".to_string()),
                    form: None,
                    ai_task: Some(AITaskConfig {
                        task_type: "write".to_string(),
                        prompt: None,
                        streaming: true,
                    }),
                    behavior: StepBehavior {
                        skippable: false,
                        redoable: true,
                        auto_advance: false,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
            WorkflowStep {
                definition: StepDefinition {
                    id: "polish".to_string(),
                    step_type: StepType::Polish,
                    title: "润色优化".to_string(),
                    description: Some("AI 检查并建议优化".to_string()),
                    form: None,
                    ai_task: Some(AITaskConfig {
                        task_type: "polish".to_string(),
                        prompt: None,
                        streaming: true,
                    }),
                    behavior: StepBehavior {
                        skippable: true,
                        redoable: true,
                        auto_advance: false,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
            WorkflowStep {
                definition: StepDefinition {
                    id: "adapt".to_string(),
                    step_type: StepType::Adapt,
                    title: "适配发布".to_string(),
                    description: Some("选择目标平台，AI 自动适配格式".to_string()),
                    form: Some(FormConfig {
                        fields: vec![FormField {
                            name: "platform".to_string(),
                            label: "目标平台".to_string(),
                            field_type: FormFieldType::Checkbox,
                            required: true,
                            placeholder: None,
                            options: Some(vec![
                                FormFieldOption {
                                    label: "微信公众号".to_string(),
                                    value: "wechat".to_string(),
                                },
                                FormFieldOption {
                                    label: "小红书".to_string(),
                                    value: "xiaohongshu".to_string(),
                                },
                                FormFieldOption {
                                    label: "知乎".to_string(),
                                    value: "zhihu".to_string(),
                                },
                                FormFieldOption {
                                    label: "通用 Markdown".to_string(),
                                    value: "markdown".to_string(),
                                },
                            ]),
                            default_value: None,
                        }],
                        submit_label: "生成适配版本".to_string(),
                        skip_label: None,
                    }),
                    ai_task: None,
                    behavior: StepBehavior {
                        skippable: true,
                        redoable: true,
                        auto_advance: false,
                    },
                },
                status: StepStatus::Pending,
                result: None,
            },
        ]
    }
}

impl Default for WorkflowService {
    fn default() -> Self {
        Self::new()
    }
}
