//! 连接管理 Tauri 命令
//!
//! 提供连接配置的 CRUD 操作接口。
//!
//! ## 命令列表
//! - `connection_list` - 获取所有连接（本地 + SSH 配置）
//! - `connection_add` - 添加新连接
//! - `connection_update` - 更新连接
//! - `connection_delete` - 删除连接
//! - `connection_get_config_path` - 获取配置文件路径
//! - `connection_get_raw_config` - 获取原始配置内容
//! - `connection_save_raw_config` - 保存原始配置内容

use crate::terminal::connections::{
    ConnectionConfig, ConnectionConfigManager, ConnectionConfigType, ConnectionListEntry,
};
use serde::{Deserialize, Serialize};

/// 添加连接的请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddConnectionRequest {
    /// 连接名称
    pub name: String,
    /// 连接类型
    #[serde(rename = "type")]
    pub conn_type: ConnectionConfigType,
    /// 用户名（SSH）
    pub user: Option<String>,
    /// 主机名（SSH）
    pub host: Option<String>,
    /// 端口（SSH）
    pub port: Option<u16>,
    /// 身份文件
    pub identity_file: Option<String>,
    /// 跳板机
    pub proxy_jump: Option<String>,
    /// WSL 发行版
    pub wsl_distro: Option<String>,
}

/// 更新连接的请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConnectionRequest {
    /// 连接名称
    pub name: String,
    /// 连接配置
    pub config: ConnectionConfig,
}

/// 通用操作响应
#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl ConnectionResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(msg.into()),
        }
    }
}

/// 获取所有可用连接
///
/// 返回本地连接 + 用户配置 + SSH 配置中的 Host 列表
#[tauri::command]
pub fn connection_list() -> Result<Vec<ConnectionListEntry>, String> {
    let manager = ConnectionConfigManager::new();
    manager.list_all_connections()
}

/// 添加新连接
#[tauri::command]
pub fn connection_add(request: AddConnectionRequest) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    // 加载现有配置
    let mut config = match manager.load() {
        Ok(c) => c,
        Err(e) => return ConnectionResponse::err(e),
    };

    // 检查名称是否已存在
    if config.connections.contains_key(&request.name) {
        return ConnectionResponse::err(format!("连接 '{}' 已存在", request.name));
    }

    // 创建新连接配置
    let conn_config = ConnectionConfig {
        conn_type: request.conn_type,
        user: request.user,
        host: request.host,
        port: request.port,
        identity_file: request.identity_file,
        identity_files: None,
        proxy_jump: request.proxy_jump,
        display_order: None,
        hidden: None,
        wsl_distro: request.wsl_distro,
    };

    // 添加并保存
    config.add(request.name.clone(), conn_config);

    match manager.save(&config) {
        Ok(_) => {
            tracing::info!("[Connection] 添加连接成功: {}", request.name);
            ConnectionResponse::ok()
        }
        Err(e) => ConnectionResponse::err(e),
    }
}

/// 更新连接
#[tauri::command]
pub fn connection_update(request: UpdateConnectionRequest) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    // 加载现有配置
    let mut config = match manager.load() {
        Ok(c) => c,
        Err(e) => return ConnectionResponse::err(e),
    };

    // 检查连接是否存在
    if !config.connections.contains_key(&request.name) {
        return ConnectionResponse::err(format!("连接 '{}' 不存在", request.name));
    }

    // 更新配置
    config
        .connections
        .insert(request.name.clone(), request.config);

    match manager.save(&config) {
        Ok(_) => {
            tracing::info!("[Connection] 更新连接成功: {}", request.name);
            ConnectionResponse::ok()
        }
        Err(e) => ConnectionResponse::err(e),
    }
}

/// 删除连接
#[tauri::command]
pub fn connection_delete(name: String) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    // 加载现有配置
    let mut config = match manager.load() {
        Ok(c) => c,
        Err(e) => return ConnectionResponse::err(e),
    };

    // 检查连接是否存在
    if !config.connections.contains_key(&name) {
        return ConnectionResponse::err(format!("连接 '{}' 不存在", name));
    }

    // 删除并保存
    config.remove(&name);

    match manager.save(&config) {
        Ok(_) => {
            tracing::info!("[Connection] 删除连接成功: {}", name);
            ConnectionResponse::ok()
        }
        Err(e) => ConnectionResponse::err(e),
    }
}

/// 获取连接配置
#[tauri::command]
pub fn connection_get(name: String) -> Result<Option<ConnectionConfig>, String> {
    let manager = ConnectionConfigManager::new();
    let config = manager.load()?;
    Ok(config.get(&name).cloned())
}

/// 获取配置文件路径
#[tauri::command]
pub fn connection_get_config_path() -> String {
    ConnectionConfigManager::default_config_path()
        .to_string_lossy()
        .to_string()
}

/// 获取原始配置文件内容（用于 JSON 编辑器）
#[tauri::command]
pub fn connection_get_raw_config() -> Result<String, String> {
    let manager = ConnectionConfigManager::new();
    manager.load_raw()
}

/// 保存原始配置文件内容（用于 JSON 编辑器）
#[tauri::command]
pub fn connection_save_raw_config(content: String) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    match manager.save_raw(&content) {
        Ok(_) => {
            tracing::info!("[Connection] 原始配置保存成功");
            ConnectionResponse::ok()
        }
        Err(e) => ConnectionResponse::err(e),
    }
}

/// 测试连接（异步）
#[tauri::command]
pub async fn connection_test(name: String) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    // 加载配置
    let config = match manager.load() {
        Ok(c) => c,
        Err(e) => return ConnectionResponse::err(e),
    };

    // 获取连接
    let conn = match config.get(&name) {
        Some(c) => c,
        None => return ConnectionResponse::err(format!("连接 '{}' 不存在", name)),
    };

    // 根据连接类型测试
    match conn.conn_type {
        ConnectionConfigType::Local => {
            // 本地连接总是可用
            ConnectionResponse::ok()
        }
        ConnectionConfigType::Ssh => {
            // SSH 连接测试
            let host = match &conn.host {
                Some(h) => h,
                None => return ConnectionResponse::err("SSH 连接缺少主机名"),
            };

            let port = conn.port.unwrap_or(22);

            // 简单的 TCP 连接测试
            match tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
                Ok(_) => {
                    tracing::info!("[Connection] SSH 连接测试成功: {}:{}", host, port);
                    ConnectionResponse::ok()
                }
                Err(e) => {
                    tracing::warn!("[Connection] SSH 连接测试失败: {} - {}", host, e);
                    ConnectionResponse::err(format!("连接失败: {}", e))
                }
            }
        }
        ConnectionConfigType::Wsl => {
            // WSL 连接测试
            #[cfg(target_os = "windows")]
            {
                // 检查 wsl.exe 是否可用
                match std::process::Command::new("wsl")
                    .arg("--list")
                    .arg("--quiet")
                    .output()
                {
                    Ok(output) => {
                        if output.status.success() {
                            ConnectionResponse::ok()
                        } else {
                            ConnectionResponse::err("WSL 命令执行失败")
                        }
                    }
                    Err(e) => ConnectionResponse::err(format!("WSL 不可用: {}", e)),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                ConnectionResponse::err("WSL 仅在 Windows 上可用")
            }
        }
    }
}

/// 导入 SSH 配置中的 Host 到用户配置
#[tauri::command]
pub fn connection_import_ssh_host(host_name: String) -> ConnectionResponse {
    let manager = ConnectionConfigManager::new();

    // 加载 SSH hosts
    let ssh_hosts = manager.load_ssh_hosts();

    // 查找指定的 host
    let ssh_host = match ssh_hosts.into_iter().find(|h| h.pattern == host_name) {
        Some(h) => h,
        None => return ConnectionResponse::err(format!("SSH Host '{}' 不存在", host_name)),
    };

    // 加载用户配置
    let mut config = match manager.load() {
        Ok(c) => c,
        Err(e) => return ConnectionResponse::err(e),
    };

    // 检查是否已存在
    if config.connections.contains_key(&host_name) {
        return ConnectionResponse::err(format!("连接 '{}' 已存在", host_name));
    }

    // 创建连接配置
    let conn_config = ConnectionConfig {
        conn_type: ConnectionConfigType::Ssh,
        user: ssh_host.user,
        host: ssh_host.hostname.or(Some(ssh_host.pattern.clone())),
        port: ssh_host.port,
        identity_file: ssh_host.identity_file,
        identity_files: None,
        proxy_jump: None,
        display_order: None,
        hidden: None,
        wsl_distro: None,
    };

    // 添加并保存
    config.add(host_name.clone(), conn_config);

    match manager.save(&config) {
        Ok(_) => {
            tracing::info!("[Connection] 导入 SSH Host 成功: {}", host_name);
            ConnectionResponse::ok()
        }
        Err(e) => ConnectionResponse::err(e),
    }
}
