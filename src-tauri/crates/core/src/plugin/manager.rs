//! 插件管理器
//!
//! 负责插件的生命周期管理、钩子执行和配置管理

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;

use super::loader::PluginLoader;
use super::task::{
    PluginQueueStats, PluginTaskPolicy, PluginTaskRecord, PluginTaskState, PluginTaskTracker,
};
use super::types::{
    HookResult, PluginConfig, PluginContext, PluginError, PluginInfo, PluginInstance, PluginStatus,
};
use crate::DynEmitter;

/// 插件管理器配置
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    /// 默认超时时间 (毫秒)
    pub default_timeout_ms: u64,
    /// 默认重试次数
    pub default_max_retries: u32,
    /// 默认重试退避基数 (毫秒)
    pub default_retry_backoff_ms: u64,
    /// 插件级并发上限
    pub default_max_concurrency_per_plugin: usize,
    /// 插件级队列长度上限
    pub default_queue_limit_per_plugin: usize,
    /// 任务记录保留数量
    pub task_retention_limit: usize,
    /// 是否启用插件系统
    pub enabled: bool,
    /// 最大并发插件数
    pub max_plugins: usize,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 5000,
            default_max_retries: 2,
            default_retry_backoff_ms: 300,
            default_max_concurrency_per_plugin: 4,
            default_queue_limit_per_plugin: 100,
            task_retention_limit: 2000,
            enabled: true,
            max_plugins: 50,
        }
    }
}

/// 插件管理器
pub struct PluginManager {
    /// 插件加载器
    loader: PluginLoader,
    /// 已加载的插件
    plugins: DashMap<String, Arc<RwLock<PluginInstance>>>,
    /// 插件配置
    configs: DashMap<String, PluginConfig>,
    /// 管理器配置
    config: PluginManagerConfig,
    /// 插件任务治理与跟踪
    task_tracker: PluginTaskTracker,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new(plugins_dir: PathBuf, config: PluginManagerConfig) -> Self {
        Self {
            loader: PluginLoader::new(plugins_dir),
            plugins: DashMap::new(),
            configs: DashMap::new(),
            task_tracker: PluginTaskTracker::new(config.task_retention_limit),
            config,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(
            PluginLoader::default_plugins_dir(),
            PluginManagerConfig::default(),
        )
    }

    /// 加载所有插件
    pub async fn load_all(&self) -> Result<Vec<String>, PluginError> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let configs: HashMap<String, PluginConfig> = self
            .configs
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();

        let loaded = self.loader.load_all(&configs).await?;
        let mut names = Vec::new();

        for (path, plugin) in loaded {
            let name = plugin.name().to_string();
            let config = configs.get(&name).cloned().unwrap_or_default();

            let mut instance = PluginInstance::new(plugin.clone(), path, config.clone());

            // 初始化插件
            if let Err(e) = Arc::get_mut(&mut instance.plugin)
                .ok_or_else(|| PluginError::InitError("无法获取插件可变引用".to_string()))?
                .init(&config)
                .await
            {
                tracing::warn!("插件 {} 初始化失败: {}", name, e);
                instance.state.status = PluginStatus::Error;
                instance.state.last_error = Some(e.to_string());
            } else {
                instance.state.status = if config.enabled {
                    PluginStatus::Enabled
                } else {
                    PluginStatus::Disabled
                };
            }

            self.plugins
                .insert(name.clone(), Arc::new(RwLock::new(instance)));
            names.push(name);
        }

        Ok(names)
    }

    /// 加载单个插件
    pub async fn load(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        if self.plugins.len() >= self.config.max_plugins {
            return Err(PluginError::LoadError(format!(
                "已达到最大插件数限制: {}",
                self.config.max_plugins
            )));
        }

        let config = PluginConfig::default();
        let plugin = self.loader.load(plugin_dir, &config).await?;
        let name = plugin.name().to_string();

        // 检查是否已加载
        if self.plugins.contains_key(&name) {
            return Err(PluginError::LoadError(format!("插件 {name} 已加载")));
        }

        let mut instance =
            PluginInstance::new(plugin.clone(), plugin_dir.to_path_buf(), config.clone());

        // 初始化插件
        if let Err(e) = Arc::get_mut(&mut instance.plugin)
            .ok_or_else(|| PluginError::InitError("无法获取插件可变引用".to_string()))?
            .init(&config)
            .await
        {
            instance.state.status = PluginStatus::Error;
            instance.state.last_error = Some(e.to_string());
        } else {
            instance.state.status = PluginStatus::Enabled;
        }

        self.plugins
            .insert(name.clone(), Arc::new(RwLock::new(instance)));
        Ok(name)
    }

    /// 卸载插件
    pub async fn unload(&self, name: &str) -> Result<(), PluginError> {
        let instance = self
            .plugins
            .remove(name)
            .map(|(_, v)| v)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // 关闭插件
        let mut inst = instance.write().await;
        if let Some(plugin) = Arc::get_mut(&mut inst.plugin) {
            plugin.shutdown().await?;
        }

        Ok(())
    }

    /// 启用插件
    pub async fn enable(&self, name: &str) -> Result<(), PluginError> {
        let instance = self
            .plugins
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let mut inst = instance.write().await;
        inst.config.enabled = true;
        inst.state.status = PluginStatus::Enabled;

        // 更新配置
        self.configs.insert(name.to_string(), inst.config.clone());

        Ok(())
    }

    /// 禁用插件
    pub async fn disable(&self, name: &str) -> Result<(), PluginError> {
        let instance = self
            .plugins
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let mut inst = instance.write().await;
        inst.config.enabled = false;
        inst.state.status = PluginStatus::Disabled;

        // 更新配置
        self.configs.insert(name.to_string(), inst.config.clone());

        Ok(())
    }

    /// 更新插件配置
    pub async fn update_config(&self, name: &str, config: PluginConfig) -> Result<(), PluginError> {
        let instance = self
            .plugins
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let mut inst = instance.write().await;
        inst.config = config.clone();

        // 更新状态
        if config.enabled && inst.state.status != PluginStatus::Error {
            inst.state.status = PluginStatus::Enabled;
        } else if !config.enabled {
            inst.state.status = PluginStatus::Disabled;
        }

        // 更新配置存储
        self.configs.insert(name.to_string(), config);

        Ok(())
    }

    /// 获取插件配置
    pub fn get_config(&self, name: &str) -> Option<PluginConfig> {
        self.configs.get(name).map(|r| r.value().clone())
    }

    /// 获取插件信息
    pub async fn get_info(&self, name: &str) -> Option<PluginInfo> {
        let instance = self.plugins.get(name)?;
        let inst = instance.read().await;
        Some(inst.info())
    }

    /// 获取所有插件信息
    pub async fn list(&self) -> Vec<PluginInfo> {
        let mut infos = Vec::new();
        for entry in self.plugins.iter() {
            let inst = entry.value().read().await;
            infos.push(inst.info());
        }
        infos
    }

    /// 设置插件任务事件发射器
    pub async fn set_task_emitter(&self, emitter: DynEmitter) {
        self.task_tracker.set_emitter(emitter).await;
    }

    /// 列出插件任务
    pub fn list_tasks(
        &self,
        plugin_id: Option<&str>,
        state: Option<PluginTaskState>,
        limit: usize,
    ) -> Vec<PluginTaskRecord> {
        self.task_tracker.list_tasks(plugin_id, state, limit)
    }

    /// 获取插件任务详情
    pub fn get_task(&self, task_id: &str) -> Option<PluginTaskRecord> {
        self.task_tracker.get_task(task_id)
    }

    /// 取消插件任务
    pub fn cancel_task(&self, task_id: &str) -> bool {
        self.task_tracker.cancel_task(task_id)
    }

    /// 获取插件队列统计
    pub fn get_queue_stats(&self, plugin_id: Option<&str>) -> Vec<PluginQueueStats> {
        self.task_tracker.queue_stats(plugin_id)
    }

    fn build_policy(&self, timeout_ms: u64) -> PluginTaskPolicy {
        PluginTaskPolicy {
            timeout_ms,
            max_retries: self.config.default_max_retries,
            retry_backoff_ms: self.config.default_retry_backoff_ms,
            max_concurrency_per_plugin: self.config.default_max_concurrency_per_plugin,
            queue_limit_per_plugin: self.config.default_queue_limit_per_plugin,
        }
    }

    /// 执行请求前钩子 (带隔离)
    pub async fn run_on_request(
        &self,
        ctx: &mut PluginContext,
        request: &mut serde_json::Value,
    ) -> Vec<HookResult> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut results = Vec::new();

        for entry in self.plugins.iter() {
            let instance = entry.value().read().await;
            if !instance.is_enabled() {
                continue;
            }

            let timeout_ms = instance.config.timeout_ms;
            let policy = self.build_policy(timeout_ms);
            let plugin = instance.plugin.clone();
            let plugin_name = plugin.name().to_string();
            let base_ctx = ctx.clone();
            let base_request = request.clone();

            let result = match self
                .task_tracker
                .execute(&plugin_name, "on_request", policy, move |_attempt| {
                    let plugin = plugin.clone();
                    let mut attempt_ctx = base_ctx.clone();
                    let mut attempt_request = base_request.clone();
                    async move {
                        let hook_result = plugin
                            .on_request(&mut attempt_ctx, &mut attempt_request)
                            .await?;
                        Ok((hook_result, attempt_ctx, attempt_request))
                    }
                })
                .await
            {
                Ok((hook_result, next_ctx, next_request)) => {
                    *ctx = next_ctx;
                    *request = next_request;
                    hook_result
                }
                Err(failure) => {
                    tracing::warn!(
                        "插件 {} on_request 执行失败: {} (state={:?}, attempts={})",
                        plugin_name,
                        failure.message,
                        failure.state,
                        failure.attempts
                    );
                    HookResult::failure(failure.message, timeout_ms)
                }
            };

            // 更新状态
            drop(instance);
            if let Some(inst) = self.plugins.get(&plugin_name) {
                let mut inst = inst.write().await;
                inst.state
                    .record_execution(result.success, result.error.clone());
            }

            results.push(result);
        }

        results
    }

    /// 执行响应后钩子 (带隔离)
    pub async fn run_on_response(
        &self,
        ctx: &mut PluginContext,
        response: &mut serde_json::Value,
    ) -> Vec<HookResult> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut results = Vec::new();

        for entry in self.plugins.iter() {
            let instance = entry.value().read().await;
            if !instance.is_enabled() {
                continue;
            }

            let timeout_ms = instance.config.timeout_ms;
            let policy = self.build_policy(timeout_ms);
            let plugin = instance.plugin.clone();
            let plugin_name = plugin.name().to_string();
            let base_ctx = ctx.clone();
            let base_response = response.clone();

            let result = match self
                .task_tracker
                .execute(&plugin_name, "on_response", policy, move |_attempt| {
                    let plugin = plugin.clone();
                    let mut attempt_ctx = base_ctx.clone();
                    let mut attempt_response = base_response.clone();
                    async move {
                        let hook_result = plugin
                            .on_response(&mut attempt_ctx, &mut attempt_response)
                            .await?;
                        Ok((hook_result, attempt_ctx, attempt_response))
                    }
                })
                .await
            {
                Ok((hook_result, next_ctx, next_response)) => {
                    *ctx = next_ctx;
                    *response = next_response;
                    hook_result
                }
                Err(failure) => {
                    tracing::warn!(
                        "插件 {} on_response 执行失败: {} (state={:?}, attempts={})",
                        plugin_name,
                        failure.message,
                        failure.state,
                        failure.attempts
                    );
                    HookResult::failure(failure.message, timeout_ms)
                }
            };

            // 更新状态
            drop(instance);
            if let Some(inst) = self.plugins.get(&plugin_name) {
                let mut inst = inst.write().await;
                inst.state
                    .record_execution(result.success, result.error.clone());
            }

            results.push(result);
        }

        results
    }

    /// 执行错误钩子 (带隔离)
    pub async fn run_on_error(&self, ctx: &mut PluginContext, error: &str) -> Vec<HookResult> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut results = Vec::new();

        for entry in self.plugins.iter() {
            let instance = entry.value().read().await;
            if !instance.is_enabled() {
                continue;
            }

            let timeout_ms = instance.config.timeout_ms;
            let policy = self.build_policy(timeout_ms);
            let plugin = instance.plugin.clone();
            let plugin_name = plugin.name().to_string();
            let base_ctx = ctx.clone();
            let error_text = error.to_string();

            let result = match self
                .task_tracker
                .execute(&plugin_name, "on_error", policy, move |_attempt| {
                    let plugin = plugin.clone();
                    let mut attempt_ctx = base_ctx.clone();
                    let error_text = error_text.clone();
                    async move {
                        let hook_result = plugin.on_error(&mut attempt_ctx, &error_text).await?;
                        Ok((hook_result, attempt_ctx))
                    }
                })
                .await
            {
                Ok((hook_result, next_ctx)) => {
                    *ctx = next_ctx;
                    hook_result
                }
                Err(failure) => {
                    tracing::warn!(
                        "插件 {} on_error 执行失败: {} (state={:?}, attempts={})",
                        plugin_name,
                        failure.message,
                        failure.state,
                        failure.attempts
                    );
                    HookResult::failure(failure.message, timeout_ms)
                }
            };

            // 更新状态
            drop(instance);
            if let Some(inst) = self.plugins.get(&plugin_name) {
                let mut inst = inst.write().await;
                inst.state
                    .record_execution(result.success, result.error.clone());
            }

            results.push(result);
        }

        results
    }

    /// 获取已加载插件数量
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// 检查插件是否已加载
    pub fn is_loaded(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// 获取插件目录
    pub fn plugins_dir(&self) -> &Path {
        self.loader.plugins_dir()
    }

    /// 设置插件配置 (批量)
    pub fn set_configs(&self, configs: HashMap<String, PluginConfig>) {
        for (name, config) in configs {
            self.configs.insert(name, config);
        }
    }

    /// 获取所有插件配置
    pub fn get_all_configs(&self) -> HashMap<String, PluginConfig> {
        self.configs
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    // ========================================================================
    // 插件 UI 相关方法
    // ========================================================================

    /// 获取插件的 Surface 定义
    pub async fn get_plugin_surfaces(
        &self,
        plugin_id: &str,
    ) -> Result<Vec<super::SurfaceDefinition>, PluginError> {
        // 目前返回空列表，后续可以扩展为从插件获取 UI 定义
        // 插件需要实现 PluginUI trait
        let _instance = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let policy = self.build_policy(self.config.default_timeout_ms);
        self.task_tracker
            .execute(plugin_id, "get_plugin_surfaces", policy, |_attempt| async {
                Ok::<_, PluginError>(Vec::new())
            })
            .await
            .map_err(|failure| PluginError::ExecutionError {
                plugin_name: plugin_id.to_string(),
                message: failure.message,
            })
    }

    /// 处理插件 UI 操作
    pub async fn handle_plugin_action(
        &mut self,
        plugin_id: &str,
        action: super::UserAction,
    ) -> Result<Vec<super::UIMessage>, PluginError> {
        let _instance = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let action_name = action.name.clone();
        let surface_id = action.surface_id.clone();
        let policy = self.build_policy(self.config.default_timeout_ms);

        self.task_tracker
            .execute(plugin_id, "handle_plugin_action", policy, move |_attempt| {
                let action_name = action_name.clone();
                let surface_id = surface_id.clone();
                async move {
                    tracing::debug!(
                        "收到插件 {} 的 UI 操作: {} (surface: {})",
                        plugin_id,
                        action_name,
                        surface_id
                    );
                    Ok::<_, PluginError>(Vec::new())
                }
            })
            .await
            .map_err(|failure| PluginError::ExecutionError {
                plugin_name: plugin_id.to_string(),
                message: failure.message,
            })
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}
