use crate::database::dao::providers::ProviderDao;
use crate::database::DbConnection;
use crate::models::{AppType, Provider};
use crate::services::live_sync;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

pub struct SwitchService;

static SWITCH_PROVIDER_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 用于在异步上下文中传递的切换数据
struct SwitchContext {
    target_provider: Provider,
    current_provider: Option<Provider>,
    app_type_enum: AppType,
}

impl SwitchService {
    pub fn get_providers(db: &DbConnection, app_type: &str) -> Result<Vec<Provider>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ProviderDao::get_all(&conn, app_type).map_err(|e| e.to_string())
    }

    pub fn get_current_provider(
        db: &DbConnection,
        app_type: &str,
    ) -> Result<Option<Provider>, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ProviderDao::get_current(&conn, app_type).map_err(|e| e.to_string())
    }

    pub fn add_provider(db: &DbConnection, provider: Provider) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // Check if this is the first provider for this app type
        let existing =
            ProviderDao::get_all(&conn, &provider.app_type).map_err(|e| e.to_string())?;
        let is_first = existing.is_empty();

        ProviderDao::insert(&conn, &provider).map_err(|e| e.to_string())?;

        // If this is the first provider, automatically set it as current and sync
        if is_first {
            ProviderDao::set_current(&conn, &provider.app_type, &provider.id)
                .map_err(|e| e.to_string())?;

            if let Ok(app_type_enum) = provider.app_type.parse::<AppType>() {
                if app_type_enum != AppType::ProxyCast {
                    live_sync::sync_to_live(&app_type_enum, &provider)
                        .map_err(|e| format!("Failed to sync: {e}"))?;
                }
            }
        }

        Ok(())
    }

    pub fn update_provider(db: &DbConnection, provider: Provider) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // Check if this is the current provider
        let current =
            ProviderDao::get_current(&conn, &provider.app_type).map_err(|e| e.to_string())?;
        let is_current = current
            .as_ref()
            .map(|p| p.id == provider.id)
            .unwrap_or(false);

        ProviderDao::update(&conn, &provider).map_err(|e| e.to_string())?;

        // If this is the current provider, sync to live
        if is_current {
            if let Ok(app_type_enum) = provider.app_type.parse::<AppType>() {
                if app_type_enum != AppType::ProxyCast {
                    live_sync::sync_to_live(&app_type_enum, &provider)
                        .map_err(|e| format!("Failed to sync: {e}"))?;
                }
            }
        }

        Ok(())
    }

    pub fn delete_provider(db: &DbConnection, app_type: &str, id: &str) -> Result<(), String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // Check if trying to delete the current provider
        let current = ProviderDao::get_current(&conn, app_type).map_err(|e| e.to_string())?;
        if let Some(ref current_provider) = current {
            if current_provider.id == id {
                return Err("Cannot delete the currently active provider".to_string());
            }
        }

        ProviderDao::delete(&conn, app_type, id).map_err(|e| e.to_string())
    }

    pub fn switch_provider(db: &DbConnection, app_type: &str, id: &str) -> Result<(), String> {
        use tracing::{error, info, warn};

        info!("开始切换 {} 配置到 provider: {}", app_type, id);

        let conn = db.lock().map_err(|e| e.to_string())?;

        // Get target provider
        let target_provider = ProviderDao::get_by_id(&conn, app_type, id)
            .map_err(|e| {
                error!("查找目标 provider 失败: {}", e);
                e.to_string()
            })?
            .ok_or_else(|| {
                error!("目标 provider 不存在: {}", id);
                format!("Provider not found: {id}")
            })?;

        let app_type_enum = app_type.parse::<AppType>().map_err(|e| {
            error!("无效的 app_type: {} - {}", app_type, e);
            e.to_string()
        })?;

        // 获取当前 provider（用于回填和回滚）
        let current_provider = if app_type_enum != AppType::ProxyCast {
            ProviderDao::get_current(&conn, app_type).map_err(|e| {
                error!("获取当前 provider 失败: {}", e);
                e.to_string()
            })?
        } else {
            None
        };

        // 实施事务保护：先尝试同步，再更新数据库
        if app_type_enum != AppType::ProxyCast {
            // Step 1: Backfill - 回填当前配置
            if let Some(ref current) = current_provider {
                if current.id != id {
                    info!("回填当前配置: {}", current.name);
                    match live_sync::read_live_settings(&app_type_enum) {
                        Ok(live_settings) => {
                            let mut updated_provider = current.clone();
                            updated_provider.settings_config = live_settings;
                            if let Err(e) = ProviderDao::update(&conn, &updated_provider) {
                                warn!("回填配置失败，但继续执行: {}", e);
                            } else {
                                info!("回填配置完成");
                            }
                        }
                        Err(e) => {
                            warn!("读取当前配置失败，跳过回填: {}", e);
                        }
                    }
                }
            }

            // Step 2: 尝试同步新配置（在更新数据库前验证）
            info!("验证目标配置可同步性");
            if let Err(sync_error) = live_sync::sync_to_live(&app_type_enum, &target_provider) {
                error!("配置同步失败: {}", sync_error);

                // 尝试恢复原配置（如果有）
                if let Some(ref current) = current_provider {
                    warn!("尝试恢复原配置: {}", current.name);
                    if let Err(restore_error) = live_sync::sync_to_live(&app_type_enum, current) {
                        error!("恢复原配置失败: {}", restore_error);
                        return Err(format!("切换失败且无法恢复原配置: {sync_error}"));
                    }
                }

                return Err(format!("配置同步失败: {sync_error}"));
            }
        }

        // Step 3: 更新数据库（同步成功后）
        info!("更新数据库中的当前 provider");
        if let Err(db_error) = ProviderDao::set_current(&conn, app_type, id) {
            error!("数据库更新失败: {}", db_error);

            // 如果数据库更新失败，尝试恢复原配置文件
            if app_type_enum != AppType::ProxyCast {
                if let Some(ref current) = current_provider {
                    warn!("数据库更新失败，尝试恢复原配置文件");
                    if let Err(restore_error) = live_sync::sync_to_live(&app_type_enum, current) {
                        error!("恢复配置文件失败: {}", restore_error);
                    }
                }
            }

            return Err(db_error.to_string());
        }

        info!("配置切换成功: {} -> {}", app_type, target_provider.name);
        Ok(())
    }

    /// 异步版本的 switch_provider，优化 Windows 性能
    ///
    /// 优化策略：
    /// 1. 减少数据库锁持有时间 - 先获取数据，释放锁，执行 I/O，再获取锁更新
    /// 2. 使用 spawn_blocking 将文件 I/O 移出主线程
    /// 3. 使用全局互斥锁确保切换流程串行化，避免并发写入
    pub async fn switch_provider_async(
        db: &DbConnection,
        app_type: &str,
        id: &str,
    ) -> Result<(), String> {
        use tracing::{error, info, warn};

        info!("开始切换 {} 配置到 provider: {} (异步)", app_type, id);
        let _switch_guard = SWITCH_PROVIDER_LOCK.lock().await;

        // Step 1: 获取数据（短暂持有锁）
        let ctx = {
            let conn = db.lock().map_err(|e| e.to_string())?;

            // Get target provider
            let target_provider = ProviderDao::get_by_id(&conn, app_type, id)
                .map_err(|e| {
                    error!("查找目标 provider 失败: {}", e);
                    e.to_string()
                })?
                .ok_or_else(|| {
                    error!("目标 provider 不存在: {}", id);
                    format!("Provider not found: {id}")
                })?;

            let app_type_enum = app_type.parse::<AppType>().map_err(|e| {
                error!("无效的 app_type: {} - {}", app_type, e);
                e.to_string()
            })?;

            // 获取当前 provider（用于回填和回滚）
            let current_provider = if app_type_enum != AppType::ProxyCast {
                ProviderDao::get_current(&conn, app_type).map_err(|e| {
                    error!("获取当前 provider 失败: {}", e);
                    e.to_string()
                })?
            } else {
                None
            };

            // 锁在这里释放
            SwitchContext {
                target_provider,
                current_provider,
                app_type_enum,
            }
        };

        // Step 2: 执行文件 I/O（在后台线程，不持有锁）
        if ctx.app_type_enum != AppType::ProxyCast {
            let current_for_backfill = ctx.current_provider.clone();
            let app_type_for_sync = ctx.app_type_enum.clone();
            let target_id = id.to_string();

            // 使用 spawn_blocking 将文件 I/O 移到后台线程
            let sync_result = tokio::task::spawn_blocking(move || {
                // Step 2a: Backfill - 回填当前配置
                if let Some(ref current) = current_for_backfill {
                    if current.id != target_id {
                        info!("回填当前配置: {}", current.name);
                        match live_sync::read_live_settings(&app_type_for_sync) {
                            Ok(live_settings) => {
                                // 返回需要更新的 provider 数据
                                Some((current.clone(), live_settings))
                            }
                            Err(e) => {
                                warn!("读取当前配置失败，跳过回填: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .await
            .map_err(|e| format!("后台任务失败: {e}"))?;

            // 如果需要回填，更新数据库（短暂持有锁）
            if let Some((mut current, live_settings)) = sync_result {
                let conn = db.lock().map_err(|e| e.to_string())?;
                current.settings_config = live_settings;
                if let Err(e) = ProviderDao::update(&conn, &current) {
                    warn!("回填配置失败，但继续执行: {}", e);
                } else {
                    info!("回填配置完成");
                }
                // 锁在这里释放
            }

            // Step 2b: 同步新配置（在后台线程）
            let target_for_sync = ctx.target_provider.clone();
            let current_for_restore = ctx.current_provider.clone();
            let app_type_for_sync = ctx.app_type_enum.clone();

            tokio::task::spawn_blocking(move || {
                info!("验证目标配置可同步性");
                if let Err(sync_error) =
                    live_sync::sync_to_live(&app_type_for_sync, &target_for_sync)
                {
                    error!("配置同步失败: {}", sync_error);

                    // 尝试恢复原配置（如果有）
                    if let Some(ref current) = current_for_restore {
                        warn!("尝试恢复原配置: {}", current.name);
                        if let Err(restore_error) =
                            live_sync::sync_to_live(&app_type_for_sync, current)
                        {
                            error!("恢复原配置失败: {}", restore_error);
                            return Err(format!("切换失败且无法恢复原配置: {sync_error}"));
                        }
                    }

                    return Err(format!("配置同步失败: {sync_error}"));
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("后台任务失败: {e}"))??;
        }

        // Step 3: 更新数据库（短暂持有锁）
        {
            let conn = db.lock().map_err(|e| e.to_string())?;
            info!("更新数据库中的当前 provider");
            if let Err(db_error) = ProviderDao::set_current(&conn, app_type, id) {
                error!("数据库更新失败: {}", db_error);

                // 如果数据库更新失败，尝试恢复原配置文件
                if ctx.app_type_enum != AppType::ProxyCast {
                    if let Some(ref current) = ctx.current_provider {
                        warn!("数据库更新失败，尝试恢复原配置文件");
                        let current_clone = current.clone();
                        let app_type_clone = ctx.app_type_enum.clone();
                        // 在后台线程恢复
                        let _ = tokio::task::spawn_blocking(move || {
                            if let Err(restore_error) =
                                live_sync::sync_to_live(&app_type_clone, &current_clone)
                            {
                                error!("恢复配置文件失败: {}", restore_error);
                            }
                        });
                    }
                }

                return Err(db_error.to_string());
            }
            // 锁在这里释放
        }

        info!("配置切换成功: {} -> {}", app_type, ctx.target_provider.name);
        Ok(())
    }

    /// Import current live config as a default provider
    pub fn import_default_config(db: &DbConnection, app_type: &str) -> Result<bool, String> {
        let conn = db.lock().map_err(|e| e.to_string())?;

        // Check if providers already exist
        let existing = ProviderDao::get_all(&conn, app_type).map_err(|e| e.to_string())?;
        if !existing.is_empty() {
            return Ok(false); // Already has providers, skip import
        }

        let app_type_enum = app_type.parse::<AppType>().map_err(|e| e.to_string())?;

        // Skip for ProxyCast
        if app_type_enum == AppType::ProxyCast {
            return Ok(false);
        }

        // Read live settings
        let live_settings = live_sync::read_live_settings(&app_type_enum)
            .map_err(|e| format!("Failed to read live settings: {e}"))?;

        // Create default provider
        let provider = Provider {
            id: "default".to_string(),
            app_type: app_type.to_string(),
            name: "Default (Imported)".to_string(),
            settings_config: live_settings,
            category: Some("custom".to_string()),
            icon: None,
            icon_color: Some("#6366f1".to_string()),
            notes: Some("Imported from existing configuration".to_string()),
            is_current: true,
            sort_index: Some(0),
            created_at: Some(chrono::Utc::now().timestamp()),
        };

        ProviderDao::insert(&conn, &provider).map_err(|e| e.to_string())?;

        Ok(true)
    }

    /// Read current live settings for an app type
    pub fn read_live_settings(app_type: &str) -> Result<serde_json::Value, String> {
        let app_type_enum = app_type.parse::<AppType>().map_err(|e| e.to_string())?;
        live_sync::read_live_settings_for_display(&app_type_enum).map_err(|e| e.to_string())
    }
}
