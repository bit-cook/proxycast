//! 状态初始化模块
//!
//! 包含应用状态的初始化逻辑。

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::api_key_provider_cmd::ApiKeyProviderServiceState;
use crate::commands::context_memory::ContextMemoryServiceState;
use crate::commands::flow_monitor_cmd::{
    BatchOperationsState, BookmarkManagerState, EnhancedStatsServiceState, FlowInterceptorState,
    FlowMonitorState, FlowQueryServiceState, FlowReplayerState, QuickFilterManagerState,
    SessionManagerState,
};
use crate::commands::machine_id_cmd::MachineIdState;
use crate::commands::orchestrator_cmd::OrchestratorState;
use crate::commands::plugin_cmd::PluginManagerState;
use crate::commands::plugin_install_cmd::PluginInstallerState;
use crate::commands::provider_pool_cmd::{CredentialSyncServiceState, ProviderPoolServiceState};
use crate::commands::resilience_cmd::ResilienceConfigState;
use crate::commands::skill_cmd::SkillServiceState;
use crate::commands::tool_hooks::ToolHooksServiceState;
use crate::config::{Config, ConfigManager, GlobalConfigManager, GlobalConfigManagerState};
use crate::database;
use crate::flow_monitor::{
    BatchOperations, BookmarkManager, EnhancedStatsService, FlowFileStore, FlowInterceptor,
    FlowMonitor, FlowMonitorConfig, FlowQueryService, FlowReplayer, InterceptConfig,
    QuickFilterManager, RotationConfig, SessionManager,
};
use crate::plugin;
use crate::services::api_key_provider_service::ApiKeyProviderService;
use crate::services::context_memory_service::{ContextMemoryConfig, ContextMemoryService};
use crate::services::provider_pool_service::ProviderPoolService;
use crate::services::skill_service::SkillService;
use crate::services::token_cache_service::TokenCacheService;
use crate::services::tool_hooks_service::ToolHooksService;
use crate::telemetry;

use super::types::{AppState, LogState, TokenCacheServiceState};
use crate::logger;
use crate::server;

/// 初始化核心应用状态
pub fn init_core_state(config: Config) -> (AppState, LogState) {
    let state: AppState = Arc::new(RwLock::new(server::ServerState::new(config.clone())));
    let logs: LogState = Arc::new(RwLock::new(logger::LogStore::with_config(&config.logging)));
    (state, logs)
}

/// 初始化全局配置管理器
pub fn init_global_config_manager(config: &Config) -> GlobalConfigManagerState {
    let config_path = ConfigManager::default_config_path();
    let manager = GlobalConfigManager::new(config.clone(), config_path);
    GlobalConfigManagerState::new(manager)
}

/// 初始化服务状态
pub struct ServiceStates {
    pub skill_service: SkillServiceState,
    pub provider_pool_service: ProviderPoolServiceState,
    pub api_key_provider_service: ApiKeyProviderServiceState,
    pub credential_sync_service: CredentialSyncServiceState,
    pub token_cache_service: TokenCacheServiceState,
    pub machine_id_service: MachineIdState,
    pub resilience_config: ResilienceConfigState,
    pub plugin_manager: PluginManagerState,
    pub plugin_installer: PluginInstallerState,
    pub orchestrator: OrchestratorState,
    pub context_memory_service: ContextMemoryServiceState,
    pub tool_hooks_service: ToolHooksServiceState,
}

/// 初始化所有服务状态
pub fn init_service_states() -> ServiceStates {
    // Initialize SkillService
    let skill_service = SkillService::new().expect("Failed to initialize SkillService");
    let skill_service_state = SkillServiceState(Arc::new(skill_service));

    // Initialize ProviderPoolService
    let provider_pool_service = ProviderPoolService::new();
    let provider_pool_service_state = ProviderPoolServiceState(Arc::new(provider_pool_service));

    // Initialize ApiKeyProviderService
    let api_key_provider_service = ApiKeyProviderService::new();
    let api_key_provider_service_state =
        ApiKeyProviderServiceState(Arc::new(api_key_provider_service));

    // Initialize CredentialSyncService (optional)
    let credential_sync_service_state = CredentialSyncServiceState(None);

    // Initialize TokenCacheService
    let token_cache_service = TokenCacheService::new();
    let token_cache_service_state = TokenCacheServiceState(Arc::new(token_cache_service));

    // Initialize MachineIdService
    let machine_id_service = crate::services::machine_id_service::MachineIdService::new()
        .expect("Failed to initialize MachineIdService");
    let machine_id_service_state: MachineIdState = Arc::new(RwLock::new(machine_id_service));

    // Initialize ResilienceConfigState
    let resilience_config_state = ResilienceConfigState::default();

    // Initialize PluginManager
    let plugin_manager = plugin::PluginManager::with_defaults();
    let plugin_manager_state = PluginManagerState(Arc::new(RwLock::new(plugin_manager)));

    // Initialize PluginInstaller
    let plugin_installer_state = init_plugin_installer();

    // Initialize Orchestrator State
    let orchestrator_state = OrchestratorState::new();

    // Initialize ContextMemoryService
    let context_memory_config = ContextMemoryConfig::default();
    let context_memory_service = ContextMemoryService::new(context_memory_config)
        .expect("Failed to initialize ContextMemoryService");
    let context_memory_service_state = ContextMemoryServiceState(Arc::new(context_memory_service));

    // Initialize ToolHooksService
    let tool_hooks_service = ToolHooksService::new(context_memory_service_state.0.clone());
    let tool_hooks_service_state = ToolHooksServiceState(Arc::new(tool_hooks_service));

    ServiceStates {
        skill_service: skill_service_state,
        provider_pool_service: provider_pool_service_state,
        api_key_provider_service: api_key_provider_service_state,
        credential_sync_service: credential_sync_service_state,
        token_cache_service: token_cache_service_state,
        machine_id_service: machine_id_service_state,
        resilience_config: resilience_config_state,
        plugin_manager: plugin_manager_state,
        plugin_installer: plugin_installer_state,
        orchestrator: orchestrator_state,
        context_memory_service: context_memory_service_state,
        tool_hooks_service: tool_hooks_service_state,
    }
}

/// 初始化插件安装器
fn init_plugin_installer() -> PluginInstallerState {
    let db_path = database::get_db_path().expect("Failed to get database path for PluginInstaller");
    let plugins_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("proxycast")
        .join("plugins");
    let temp_dir = std::env::temp_dir().join("proxycast_plugin_install");

    // 创建目录（如果不存在）
    if let Err(e) = std::fs::create_dir_all(&plugins_dir) {
        tracing::warn!("无法创建插件目录: {}", e);
    }
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        tracing::warn!("无法创建插件临时目录: {}", e);
    }

    match plugin::installer::PluginInstaller::from_paths(plugins_dir, temp_dir, &db_path) {
        Ok(installer) => {
            tracing::info!("[启动] 插件安装器初始化成功");
            PluginInstallerState(Arc::new(RwLock::new(installer)))
        }
        Err(e) => {
            tracing::error!("[启动] 插件安装器初始化失败: {}", e);
            // 创建一个默认的安装器（使用临时目录）
            let fallback_plugins_dir = std::env::temp_dir().join("proxycast_plugins_fallback");
            let fallback_temp_dir = std::env::temp_dir().join("proxycast_plugin_install_fallback");
            let _ = std::fs::create_dir_all(&fallback_plugins_dir);
            let _ = std::fs::create_dir_all(&fallback_temp_dir);
            let installer = plugin::installer::PluginInstaller::from_paths(
                fallback_plugins_dir,
                fallback_temp_dir,
                &db_path,
            )
            .expect("Failed to create fallback PluginInstaller");
            PluginInstallerState(Arc::new(RwLock::new(installer)))
        }
    }
}

/// 遥测状态
pub struct TelemetryStates {
    pub stats: Arc<parking_lot::RwLock<telemetry::StatsAggregator>>,
    pub tokens: Arc<parking_lot::RwLock<telemetry::TokenTracker>>,
    pub logger: Arc<telemetry::RequestLogger>,
    pub telemetry_state: crate::commands::telemetry_cmd::TelemetryState,
}

/// 初始化遥测状态
pub fn init_telemetry_states(config: &Config) -> TelemetryStates {
    let shared_stats = Arc::new(parking_lot::RwLock::new(
        telemetry::StatsAggregator::with_defaults(),
    ));
    let shared_tokens = Arc::new(parking_lot::RwLock::new(
        telemetry::TokenTracker::with_defaults(),
    ));
    let log_rotation = telemetry::LogRotationConfig {
        max_memory_logs: 10000,
        retention_days: config.logging.retention_days,
        max_file_size: 10 * 1024 * 1024,
        enable_file_logging: config.logging.enabled,
    };
    let shared_logger = Arc::new(
        telemetry::RequestLogger::new(log_rotation).expect("Failed to create RequestLogger"),
    );

    let telemetry_state = crate::commands::telemetry_cmd::TelemetryState::with_shared(
        shared_stats.clone(),
        shared_tokens.clone(),
        Some(shared_logger.clone()),
    )
    .expect("Failed to create TelemetryState");

    TelemetryStates {
        stats: shared_stats,
        tokens: shared_tokens,
        logger: shared_logger,
        telemetry_state,
    }
}

/// Flow Monitor 状态
pub struct FlowMonitorStates {
    pub flow_monitor: Arc<FlowMonitor>,
    pub flow_monitor_state: FlowMonitorState,
    pub flow_interceptor: Arc<FlowInterceptor>,
    pub flow_interceptor_state: FlowInterceptorState,
    pub flow_replayer_state: FlowReplayerState,
    pub flow_query_service_state: FlowQueryServiceState,
    pub session_manager_state: SessionManagerState,
    pub quick_filter_manager_state: QuickFilterManagerState,
    pub bookmark_manager_state: BookmarkManagerState,
    pub enhanced_stats_service_state: EnhancedStatsServiceState,
    pub batch_operations_state: BatchOperationsState,
}

/// 初始化 Flow Monitor 状态
///
/// 如果 flow-monitor 插件已安装，则启用监控功能；否则禁用。
pub fn init_flow_monitor_states(
    provider_pool_service: Arc<ProviderPoolService>,
    db: database::DbConnection,
    plugin_installer_state: &PluginInstallerState,
) -> FlowMonitorStates {
    // 检查 flow-monitor 插件是否已安装
    let is_plugin_installed = {
        let installer = plugin_installer_state.0.blocking_read();
        installer.is_installed("flow-monitor").unwrap_or(false)
    };

    // 根据插件安装状态设置 enabled
    let mut flow_monitor_config = FlowMonitorConfig::default();
    flow_monitor_config.enabled = is_plugin_installed;

    if is_plugin_installed {
        tracing::info!("[启动] flow-monitor 插件已安装，启用 Flow 监控");
    } else {
        tracing::info!("[启动] flow-monitor 插件未安装，禁用 Flow 监控");
    }

    let flow_file_store = init_flow_file_store();

    let flow_monitor = Arc::new(FlowMonitor::new(
        flow_monitor_config,
        flow_file_store.clone(),
    ));
    let flow_monitor_state = FlowMonitorState(flow_monitor.clone());

    // 初始化 Flow 拦截器
    let flow_interceptor = Arc::new(FlowInterceptor::new(InterceptConfig::default()));
    let flow_interceptor_state = FlowInterceptorState(flow_interceptor.clone());

    // 初始化 Flow 重放器
    let flow_replayer = Arc::new(FlowReplayer::new(
        flow_monitor.clone(),
        provider_pool_service,
        db,
    ));
    let flow_replayer_state = FlowReplayerState(flow_replayer);

    // 初始化会话管理器
    let db_path = database::get_db_path().expect("Failed to get database path");
    let session_manager =
        Arc::new(SessionManager::new(db_path.clone()).expect("Failed to create SessionManager"));
    let session_manager_state = SessionManagerState(session_manager.clone());

    // 初始化快速过滤器管理器
    let quick_filter_manager = Arc::new(
        QuickFilterManager::new(db_path.clone()).expect("Failed to create QuickFilterManager"),
    );
    let quick_filter_manager_state = QuickFilterManagerState(quick_filter_manager);

    // 初始化书签管理器
    let bookmark_manager =
        Arc::new(BookmarkManager::new(db_path).expect("Failed to create BookmarkManager"));
    let bookmark_manager_state = BookmarkManagerState(bookmark_manager);

    // 初始化增强统计服务
    let enhanced_stats_service = Arc::new(EnhancedStatsService::new(flow_monitor.memory_store()));
    let enhanced_stats_service_state = EnhancedStatsServiceState(enhanced_stats_service);

    // 初始化批量操作服务
    let batch_operations = Arc::new(BatchOperations::new(
        flow_monitor.clone(),
        Some(session_manager_state.0.clone()),
    ));
    let batch_operations_state = BatchOperationsState(batch_operations);

    // FlowQueryService
    let flow_query_service_state = if let Some(file_store) = flow_file_store {
        let query_service = FlowQueryService::new(flow_monitor.memory_store(), file_store);
        FlowQueryServiceState(Arc::new(query_service))
    } else {
        let temp_dir = std::env::temp_dir().join("proxycast_flows");
        let _ = std::fs::create_dir_all(&temp_dir);
        let rotation_config = RotationConfig::default();
        let temp_store = FlowFileStore::new(temp_dir, rotation_config)
            .expect("Failed to create temp FlowFileStore");
        let query_service =
            FlowQueryService::new(flow_monitor.memory_store(), Arc::new(temp_store));
        FlowQueryServiceState(Arc::new(query_service))
    };

    FlowMonitorStates {
        flow_monitor,
        flow_monitor_state,
        flow_interceptor,
        flow_interceptor_state,
        flow_replayer_state,
        flow_query_service_state,
        session_manager_state,
        quick_filter_manager_state,
        bookmark_manager_state,
        enhanced_stats_service_state,
        batch_operations_state,
    }
}

/// 初始化 Flow 文件存储
fn init_flow_file_store() -> Option<Arc<FlowFileStore>> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("proxycast")
        .join("flows");

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        tracing::warn!("无法创建 Flow 存储目录: {}", e);
    }

    let rotation_config = RotationConfig::default();
    match FlowFileStore::new(data_dir, rotation_config) {
        Ok(store) => Some(Arc::new(store)),
        Err(e) => {
            tracing::warn!("无法初始化 Flow 文件存储: {}", e);
            None
        }
    }
}
