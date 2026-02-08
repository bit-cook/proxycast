//! 截图图片处理服务
//!
//! 提供截图文件读取与 Base64 编码能力。

use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;
use tokio::fs;

/// 读取图片文件并转换为 Base64
pub async fn read_image_as_base64(path: &str) -> Result<String, String> {
    tracing::debug!("读取图片为 Base64: {}", path);

    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err(format!("文件不存在: {}", file_path.display()));
    }

    let bytes = fs::read(file_path)
        .await
        .map_err(|e| format!("读取文件失败: {e}"))?;

    if bytes.is_empty() {
        return Err("文件为空".to_string());
    }

    let base64 = STANDARD.encode(&bytes);

    tracing::debug!("图片读取成功，大小: {} 字节", bytes.len());
    Ok(base64)
}
