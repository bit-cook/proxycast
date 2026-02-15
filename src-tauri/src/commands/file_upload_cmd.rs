//! 文件上传命令
//!
//! 提供用户头像上传功能

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// 上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    /// 文件 URL
    pub url: String,
    /// 文件大小（字节）
    pub size: u64,
}

/// 上传用户头像
#[tauri::command]
pub async fn upload_avatar(file_path: String, app: AppHandle) -> Result<UploadResult, String> {
    tracing::info!("[文件上传] 上传用户头像: {}", file_path);

    let source_path = PathBuf::from(&file_path);

    // 验证文件是否存在
    if !source_path.exists() {
        return Err(format!("文件不存在: {file_path}"));
    }

    // 验证文件大小（限制 5MB）
    let file_size = std::fs::metadata(&source_path)
        .map_err(|e| format!("无法读取文件元数据: {e}"))?
        .len();

    const MAX_SIZE: u64 = 5 * 1024 * 1024; // 5MB
    if file_size > MAX_SIZE {
        return Err(format!(
            "文件过大: {file_size} bytes (最大 {MAX_SIZE} bytes)"
        ));
    }

    // 验证文件类型（通过扩展名）
    let extension = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if !["jpg", "jpeg", "png", "gif", "webp"].contains(&extension.to_lowercase().as_str()) {
        return Err(format!("不支持的文件类型: {extension}"));
    }

    // 获取资源目录
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法获取资源目录: {e}"))?;

    let avatars_dir = resource_dir.join("resources/avatars");

    // 创建目录（如果不存在）
    std::fs::create_dir_all(&avatars_dir).map_err(|e| format!("无法创建头像目录: {e}"))?;

    // 生成唯一文件名
    let file_name = format!(
        "avatar_{}.{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S"),
        extension
    );

    let dest_path = avatars_dir.join(&file_name);

    // 复制文件
    std::fs::copy(&source_path, &dest_path).map_err(|e| format!("无法复制文件: {e}"))?;

    tracing::info!("[文件上传] 头像已保存: {:?}", dest_path);

    // 返回相对路径作为 URL
    let url = format!("resources/avatars/{file_name}");

    Ok(UploadResult {
        url,
        size: file_size,
    })
}

/// 删除用户头像
#[tauri::command]
pub async fn delete_avatar(url: String, app: AppHandle) -> Result<(), String> {
    tracing::info!("[文件上传] 删除用户头像: {}", url);

    // 从 URL 中提取文件路径
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法获取资源目录: {e}"))?;

    let file_path = resource_dir.join(&url);

    // 删除文件
    if file_path.exists() {
        std::fs::remove_file(&file_path).map_err(|e| format!("无法删除文件: {e}"))?;
        tracing::info!("[文件上传] 头像已删除: {:?}", file_path);
    }

    Ok(())
}
