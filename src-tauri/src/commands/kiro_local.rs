//! Kiro 凭证本地切换命令
//!
//! 将 Kiro 凭证切换到本地 IDE，同时切换设备指纹。

use crate::commands::provider_pool_cmd::ProviderPoolServiceState;
use crate::database::DbConnection;
use crate::models::kiro_fingerprint::{KiroFingerprintStore, SwitchToLocalResult};
use crate::models::provider_pool_model::CredentialData;
use proxycast_services::machine_id_service::MachineIdService;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tauri::State;

/// Kiro auth token 文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KiroAuthToken {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    client_id_hash: String,
    auth_method: String,
    provider: String,
    region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
}

/// 客户端注册文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientRegistration {
    client_id: String,
    client_secret: String,
    expires_at: String,
    scopes: Vec<String>,
}

/// 获取 AWS SSO cache 目录
fn get_aws_sso_cache_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法获取用户主目录".to_string())?;
    let cache_dir = home.join(".aws").join("sso").join("cache");

    // 确保目录存在
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).map_err(|e| format!("创建 AWS SSO cache 目录失败: {e}"))?;
    }

    Ok(cache_dir)
}

/// 计算 clientIdHash（备用方案，使用 SHA256 的前 40 位模拟 SHA1 格式）
fn calculate_client_id_hash() -> String {
    let start_url = "https://view.awsapps.com/start";
    let json_str = format!("{{\"startUrl\":\"{start_url}\"}}");

    let mut hasher = Sha256::new();
    hasher.update(json_str.as_bytes());
    let result = hasher.finalize();

    // SHA1 是 40 位十六进制，取 SHA256 的前 20 字节（40 位十六进制）
    format!("{result:x}")[..40].to_string()
}

/// 切换 Kiro 凭证到本地
///
/// 1. 从凭证池读取指定凭证
/// 2. 获取/生成绑定的 Machine ID
/// 3. 切换系统机器码
/// 4. 写入凭证到 ~/.aws/sso/cache/kiro-auth-token.json
#[tauri::command]
pub async fn switch_kiro_to_local(
    uuid: String,
    db: State<'_, DbConnection>,
    pool_service: State<'_, ProviderPoolServiceState>,
) -> Result<SwitchToLocalResult, String> {
    tracing::info!("[KIRO_LOCAL] 开始切换凭证到本地: {}", uuid);

    // 1. 获取凭证信息
    let credential = pool_service
        .0
        .get_by_uuid(&db, &uuid)
        .map_err(|e| format!("获取凭证失败: {e}"))?
        .ok_or_else(|| format!("找不到凭证: {uuid}"))?;

    // 检查是否为 Kiro 凭证
    let creds_file_path = match &credential.credential {
        CredentialData::KiroOAuth { creds_file_path } => creds_file_path.clone(),
        _ => return Err("只支持 Kiro OAuth 凭证".to_string()),
    };

    // 2. 读取凭证文件
    let creds_content =
        fs::read_to_string(&creds_file_path).map_err(|e| format!("读取凭证文件失败: {e}"))?;
    let creds: serde_json::Value =
        serde_json::from_str(&creds_content).map_err(|e| format!("解析凭证文件失败: {e}"))?;

    // 3. 获取/生成绑定的 Machine ID
    let mut fingerprint_store =
        KiroFingerprintStore::load().map_err(|e| format!("加载指纹存储失败: {e}"))?;

    let profile_arn = creds.get("profileArn").and_then(|v| v.as_str());
    let client_id = creds.get("clientId").and_then(|v| v.as_str());

    let binding = fingerprint_store
        .get_or_create_binding(&uuid, profile_arn, client_id)
        .map_err(|e| format!("获取指纹绑定失败: {e}"))?;

    let machine_id = binding.machine_id.clone();
    tracing::info!("[KIRO_LOCAL] 使用 Machine ID: {}", &machine_id[..8]);

    // 4. 切换系统机器码
    let machine_service =
        MachineIdService::new().map_err(|e| format!("初始化机器码服务失败: {e}"))?;

    let machine_result = machine_service
        .set_machine_id(&machine_id)
        .await
        .map_err(|e| format!("切换机器码失败: {e}"))?;

    if !machine_result.success {
        if machine_result.requires_admin {
            return Ok(SwitchToLocalResult::requires_admin(format!(
                "需要管理员权限切换机器码: {}",
                machine_result.message
            )));
        }
        return Ok(SwitchToLocalResult::error(format!(
            "切换机器码失败: {}",
            machine_result.message
        )));
    }

    // 5. 准备 Kiro auth token 数据
    let access_token = creds
        .get("accessToken")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "凭证文件缺少 accessToken".to_string())?;

    let refresh_token = creds
        .get("refreshToken")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "凭证文件缺少 refreshToken".to_string())?;

    let expires_at = creds
        .get("expiresAt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let auth_method = creds
        .get("authMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("social");

    let provider = creds
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("BuilderId");

    let region = creds
        .get("region")
        .and_then(|v| v.as_str())
        .unwrap_or("us-east-1");

    let client_id_hash = creds
        .get("clientIdHash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(calculate_client_id_hash);

    // 6. 写入 kiro-auth-token.json
    let cache_dir = get_aws_sso_cache_dir()?;
    let auth_token_path = cache_dir.join("kiro-auth-token.json");

    let auth_token = KiroAuthToken {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        expires_at: expires_at.to_string(),
        client_id_hash: client_id_hash.clone(),
        auth_method: auth_method.to_string(),
        provider: provider.to_string(),
        region: region.to_string(),
        client_id: creds
            .get("clientId")
            .and_then(|v| v.as_str())
            .map(String::from),
        client_secret: creds
            .get("clientSecret")
            .and_then(|v| v.as_str())
            .map(String::from),
    };

    let auth_token_json = serde_json::to_string_pretty(&auth_token)
        .map_err(|e| format!("序列化 auth token 失败: {e}"))?;

    fs::write(&auth_token_path, &auth_token_json)
        .map_err(|e| format!("写入 kiro-auth-token.json 失败: {e}"))?;

    tracing::info!("[KIRO_LOCAL] 已写入 kiro-auth-token.json");

    // 7. 如果是 IdC 认证，写入客户端注册文件
    if auth_method.to_lowercase() == "idc" {
        if let (Some(client_id), Some(client_secret)) = (
            creds.get("clientId").and_then(|v| v.as_str()),
            creds.get("clientSecret").and_then(|v| v.as_str()),
        ) {
            let registration = ClientRegistration {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
                expires_at: expires_at.to_string(),
                scopes: vec![
                    "codewhisperer:completions".to_string(),
                    "codewhisperer:analysis".to_string(),
                    "codewhisperer:conversations".to_string(),
                ],
            };

            let registration_path = cache_dir.join(format!("{client_id_hash}.json"));
            let registration_json = serde_json::to_string_pretty(&registration)
                .map_err(|e| format!("序列化客户端注册信息失败: {e}"))?;

            fs::write(&registration_path, &registration_json)
                .map_err(|e| format!("写入客户端注册文件失败: {e}"))?;

            tracing::info!(
                "[KIRO_LOCAL] 已写入客户端注册文件: {}.json",
                &client_id_hash[..8]
            );
        }
    }

    // 8. 更新最后切换时间
    fingerprint_store
        .update_last_switched(&uuid)
        .map_err(|e| format!("更新切换时间失败: {e}"))?;

    let credential_name = credential
        .name
        .clone()
        .unwrap_or_else(|| uuid[..8].to_string());

    Ok(SwitchToLocalResult::success(
        format!(
            "已切换到凭证 \"{}\"，机器码: {}...\n请重启 Kiro IDE 使配置生效",
            credential_name,
            &machine_id[..8]
        ),
        machine_id,
    ))
}

/// 获取凭证的指纹信息
#[tauri::command]
pub async fn get_kiro_fingerprint_info(
    uuid: String,
    db: State<'_, DbConnection>,
    pool_service: State<'_, ProviderPoolServiceState>,
) -> Result<KiroFingerprintInfo, String> {
    // 获取凭证信息
    let credential = pool_service
        .0
        .get_by_uuid(&db, &uuid)
        .map_err(|e| format!("获取凭证失败: {e}"))?
        .ok_or_else(|| format!("找不到凭证: {uuid}"))?;

    // 检查是否为 Kiro 凭证
    let creds_file_path = match &credential.credential {
        CredentialData::KiroOAuth { creds_file_path } => creds_file_path.clone(),
        _ => return Err("只支持 Kiro OAuth 凭证".to_string()),
    };

    // 读取凭证文件
    let creds_content =
        fs::read_to_string(&creds_file_path).map_err(|e| format!("读取凭证文件失败: {e}"))?;
    let creds: serde_json::Value =
        serde_json::from_str(&creds_content).map_err(|e| format!("解析凭证文件失败: {e}"))?;

    // 获取指纹绑定
    let mut fingerprint_store =
        KiroFingerprintStore::load().map_err(|e| format!("加载指纹存储失败: {e}"))?;

    let profile_arn = creds.get("profileArn").and_then(|v| v.as_str());
    let client_id = creds.get("clientId").and_then(|v| v.as_str());

    let binding = fingerprint_store
        .get_or_create_binding(&uuid, profile_arn, client_id)
        .map_err(|e| format!("获取指纹绑定失败: {e}"))?;

    let auth_method = creds
        .get("authMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("social");

    let source = if profile_arn.is_some() {
        "profileArn"
    } else if client_id.is_some() {
        "clientId"
    } else {
        "uuid"
    };

    Ok(KiroFingerprintInfo {
        machine_id: binding.machine_id.clone(),
        machine_id_short: binding.machine_id[..8].to_string(),
        source: source.to_string(),
        auth_method: auth_method.to_string(),
        created_at: binding.created_at.to_rfc3339(),
        last_switched_at: binding.last_switched_at.map(|t| t.to_rfc3339()),
    })
}

/// 获取当前本地使用的 Kiro 凭证 UUID
///
/// 读取 ~/.aws/sso/cache/kiro-auth-token.json，与凭证池中的凭证比较
#[tauri::command]
pub async fn get_local_kiro_credential_uuid(
    db: State<'_, DbConnection>,
    pool_service: State<'_, ProviderPoolServiceState>,
) -> Result<Option<String>, String> {
    // 读取本地 kiro-auth-token.json
    let cache_dir = get_aws_sso_cache_dir()?;
    let auth_token_path = cache_dir.join("kiro-auth-token.json");

    if !auth_token_path.exists() {
        return Ok(None);
    }

    let local_content =
        fs::read_to_string(&auth_token_path).map_err(|e| format!("读取本地凭证文件失败: {e}"))?;
    let local_creds: serde_json::Value =
        serde_json::from_str(&local_content).map_err(|e| format!("解析本地凭证文件失败: {e}"))?;

    let local_access_token = local_creds.get("accessToken").and_then(|v| v.as_str());
    let local_refresh_token = local_creds.get("refreshToken").and_then(|v| v.as_str());

    if local_access_token.is_none() && local_refresh_token.is_none() {
        return Ok(None);
    }

    // 获取所有 Kiro 凭证
    let overview = pool_service.0.get_overview(&db)?;
    let kiro_pool = overview.iter().find(|p| p.provider_type == "kiro");

    if let Some(pool) = kiro_pool {
        for cred_display in &pool.credentials {
            // 读取凭证文件并比较
            if let Ok(Some(cred)) = pool_service.0.get_by_uuid(&db, &cred_display.uuid) {
                if let CredentialData::KiroOAuth { creds_file_path } = &cred.credential {
                    if let Ok(content) = fs::read_to_string(creds_file_path) {
                        if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content) {
                            let access_token = creds.get("accessToken").and_then(|v| v.as_str());
                            let refresh_token = creds.get("refreshToken").and_then(|v| v.as_str());

                            // 比较 token
                            let matches = match (local_access_token, access_token) {
                                (Some(l), Some(r)) if l == r => true,
                                _ => match (local_refresh_token, refresh_token) {
                                    (Some(l), Some(r)) if l == r => true,
                                    _ => false,
                                },
                            };

                            if matches {
                                return Ok(Some(cred_display.uuid.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Kiro 指纹信息（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiroFingerprintInfo {
    /// 完整的 Machine ID
    pub machine_id: String,
    /// 简短的 Machine ID（前8位）
    pub machine_id_short: String,
    /// 指纹来源（profileArn/clientId/uuid）
    pub source: String,
    /// 认证方式
    pub auth_method: String,
    /// 创建时间
    pub created_at: String,
    /// 最后切换时间
    pub last_switched_at: Option<String>,
}
