//! 模型管理相关命令

use crate::database::dao::provider_pool::ProviderPoolDao;
use crate::database::DbConnection;
use proxycast_services::model_service::ModelService;
use std::collections::HashMap;
use tauri::State;

/// 获取凭证支持的模型列表（从数据库缓存）
#[tauri::command]
pub fn get_credential_models(
    db: State<'_, DbConnection>,
    credential_uuid: String,
) -> Result<Vec<String>, String> {
    tracing::info!(
        "[GET_CREDENTIAL_MODELS] 获取凭证模型列表: {}",
        credential_uuid
    );

    let model_service = ModelService::new();
    model_service.get_credential_models(&db, &credential_uuid)
}

/// 刷新凭证的模型列表（从 Provider API 重新获取）
#[tauri::command]
pub async fn refresh_credential_models(
    db: State<'_, DbConnection>,
    credential_uuid: String,
) -> Result<Vec<String>, String> {
    tracing::info!("[REFRESH_CREDENTIAL_MODELS] ========== 开始刷新凭证模型列表 ==========");
    tracing::info!(
        "[REFRESH_CREDENTIAL_MODELS] credential_uuid: {}",
        credential_uuid
    );

    let model_service = ModelService::new();

    // 从数据库获取凭证信息
    let credential = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ProviderPoolDao::get_by_uuid(&conn, &credential_uuid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("凭证不存在: {credential_uuid}"))?
    };

    tracing::info!(
        "[REFRESH_CREDENTIAL_MODELS] 凭证信息: provider_type={}, name={:?}",
        credential.provider_type,
        credential.name
    );

    // 从 Provider API 获取模型列表
    tracing::info!("[REFRESH_CREDENTIAL_MODELS] 开始从 Provider API 获取模型列表...");
    let models = model_service
        .fetch_models_for_credential(&credential)
        .await?;

    tracing::info!(
        "[REFRESH_CREDENTIAL_MODELS] 成功获取 {} 个模型: {:?}",
        models.len(),
        models
    );

    // 更新到数据库
    tracing::info!("[REFRESH_CREDENTIAL_MODELS] 更新模型列表到数据库...");
    model_service.update_credential_models(&db, &credential_uuid, models.clone())?;

    tracing::info!("[REFRESH_CREDENTIAL_MODELS] ========== 刷新完成 ==========");

    Ok(models)
}

/// 获取所有凭证的模型列表（按 Provider 类型分组）
#[tauri::command]
pub fn get_all_models_by_provider(
    db: State<'_, DbConnection>,
) -> Result<HashMap<String, Vec<String>>, String> {
    tracing::info!("[GET_ALL_MODELS_BY_PROVIDER] 获取所有 Provider 的模型列表");

    let model_service = ModelService::new();
    model_service.get_all_models_by_provider(&db)
}

/// 获取所有可用的模型列表（合并所有健康凭证的模型）
#[tauri::command]
pub fn get_all_available_models(db: State<'_, DbConnection>) -> Result<Vec<String>, String> {
    tracing::info!("[GET_ALL_AVAILABLE_MODELS] 获取所有可用模型");

    let model_service = ModelService::new();
    model_service.get_all_available_models(&db)
}

/// 批量刷新所有凭证的模型列表
#[tauri::command]
pub async fn refresh_all_credential_models(
    db: State<'_, DbConnection>,
) -> Result<HashMap<String, Result<Vec<String>, String>>, String> {
    tracing::info!("[REFRESH_ALL_CREDENTIAL_MODELS] 批量刷新所有凭证的模型列表");

    let model_service = ModelService::new();

    // 获取所有凭证
    let credentials = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        ProviderPoolDao::get_all(&conn).map_err(|e| e.to_string())?
    };

    let mut results = HashMap::new();

    for credential in credentials {
        if credential.is_disabled {
            tracing::debug!("[REFRESH_ALL] 跳过已禁用的凭证: {}", credential.uuid);
            continue;
        }

        tracing::info!(
            "[REFRESH_ALL] 刷新凭证: {} ({})",
            credential.uuid,
            credential.provider_type
        );

        // 尝试获取模型列表
        let result = match model_service.fetch_models_for_credential(&credential).await {
            Ok(models) => {
                // 更新到数据库
                if let Err(e) =
                    model_service.update_credential_models(&db, &credential.uuid, models.clone())
                {
                    tracing::error!("[REFRESH_ALL] 更新数据库失败: {}", e);
                    Err(format!("更新数据库失败: {e}"))
                } else {
                    tracing::info!("[REFRESH_ALL] 成功刷新 {} 个模型", models.len());
                    Ok(models)
                }
            }
            Err(e) => {
                tracing::warn!("[REFRESH_ALL] 获取模型列表失败: {}", e);
                Err(e)
            }
        };

        results.insert(credential.uuid.clone(), result);
    }

    Ok(results)
}

/// 获取 Provider 的默认模型列表
#[tauri::command]
pub fn get_default_models_for_provider(provider_type: String) -> Result<Vec<String>, String> {
    let pt: crate::models::provider_pool_model::PoolProviderType =
        provider_type.parse().map_err(|e: String| e)?;

    let model_service = ModelService::new();
    Ok(model_service.get_default_models_for_provider(&pt))
}
