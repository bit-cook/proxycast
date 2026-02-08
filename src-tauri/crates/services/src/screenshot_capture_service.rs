//! 屏幕截图服务
//!
//! 提供跨平台的屏幕截图功能，支持交互式区域选择

use std::path::PathBuf;
use tracing::{debug, error, info};

#[cfg(target_os = "macos")]
use tracing::warn;

/// 截图错误类型
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("用户取消截图")]
    Cancelled,
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    #[error("系统错误: {0}")]
    SystemError(String),
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("临时文件创建失败: {0}")]
    TempFileError(String),
}

/// 截图结果
#[derive(Debug)]
pub enum CaptureResult {
    /// 截图成功，包含图片路径
    Success(PathBuf),
    /// 用户取消截图
    Cancelled,
}

/// 启动交互式截图
///
/// 调用系统截图工具进行交互式区域选择
///
/// # 返回
/// 成功返回图片路径，用户取消返回 Cancelled 错误，其他情况返回相应错误
///
/// # 平台支持
/// - macOS: 使用 `screencapture -i -x` 命令
/// - Windows: 使用 Windows API 或系统截图工具
/// - Linux: 使用 `gnome-screenshot` 或 `scrot`
pub async fn start_capture() -> Result<PathBuf, CaptureError> {
    info!("启动交互式截图");

    // 生成临时文件路径
    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("proxycast_screenshot_{timestamp}.png");
    let temp_path = temp_dir.join(&filename);

    debug!("截图临时文件路径: {:?}", temp_path);

    // 根据平台调用不同的截图命令
    #[cfg(target_os = "macos")]
    {
        capture_macos(&temp_path).await?;
    }

    #[cfg(target_os = "windows")]
    {
        capture_windows(&temp_path).await?;
    }

    #[cfg(target_os = "linux")]
    {
        capture_linux(&temp_path).await?;
    }

    // 检查文件是否存在（用户可能取消了截图）
    if !temp_path.exists() {
        info!("截图文件不存在，用户可能取消了截图");
        return Err(CaptureError::Cancelled);
    }

    // 检查文件大小（空文件表示取消）
    let metadata = std::fs::metadata(&temp_path)?;
    if metadata.len() == 0 {
        info!("截图文件为空，用户取消了截图");
        std::fs::remove_file(&temp_path)?;
        return Err(CaptureError::Cancelled);
    }

    info!("截图成功: {:?}", temp_path);
    Ok(temp_path)
}

/// macOS 截图实现
#[cfg(target_os = "macos")]
async fn capture_macos(output_path: &PathBuf) -> Result<(), CaptureError> {
    use std::process::Command;

    debug!("使用 macOS screencapture 命令");

    // 先检查屏幕录制权限
    // 通过尝试执行一个快速的全屏截图到 /dev/null 来检测权限
    let permission_check = Command::new("screencapture")
        .args(["-x", "-c"]) // -c 截图到剪贴板，快速检测权限
        .output();

    if let Ok(output) = permission_check {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("cannot") || stderr.contains("permission") {
                warn!("屏幕录制权限不足: {}", stderr);
                return Err(CaptureError::PermissionDenied(
                    "请在系统设置 → 隐私与安全性 → 录屏与系统录音 中授权 ProxyCast".to_string(),
                ));
            }
        }
    }

    // 使用 screencapture 命令
    // -i: 交互式选择区域
    // -x: 不播放截图声音
    let output = Command::new("screencapture")
        .args(["-i", "-x", output_path.to_str().unwrap()])
        .output()
        .map_err(|e| CaptureError::SystemError(format!("执行 screencapture 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 用户按 ESC 取消时，screencapture 返回非零状态但不创建文件
        if !output_path.exists() {
            return Err(CaptureError::Cancelled);
        }
        error!("screencapture 命令失败: {}", stderr);
        return Err(CaptureError::SystemError(format!(
            "screencapture 失败: {stderr}"
        )));
    }

    Ok(())
}

/// Windows 截图实现
#[cfg(target_os = "windows")]
async fn capture_windows(output_path: &PathBuf) -> Result<(), CaptureError> {
    use std::process::Command;

    debug!("使用 Windows 截图工具");

    // 使用 PowerShell 调用截图功能
    // 注意：这是一个简化实现，实际可能需要使用 Windows API
    let script = format!(
        r#"
        Add-Type -AssemblyName System.Windows.Forms
        $screen = [System.Windows.Forms.Screen]::PrimaryScreen
        $bitmap = New-Object System.Drawing.Bitmap($screen.Bounds.Width, $screen.Bounds.Height)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.CopyFromScreen($screen.Bounds.Location, [System.Drawing.Point]::Empty, $screen.Bounds.Size)
        $bitmap.Save('{}')
        "#,
        output_path.to_str().unwrap().replace("\\", "\\\\")
    );

    let output = Command::new("powershell")
        .args(["-Command", &script])
        .output()
        .map_err(|e| CaptureError::SystemError(format!("执行 PowerShell 失败: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("PowerShell 截图失败: {}", stderr);
        return Err(CaptureError::SystemError(format!("截图失败: {}", stderr)));
    }

    Ok(())
}

/// Linux 截图实现
#[cfg(target_os = "linux")]
async fn capture_linux(output_path: &PathBuf) -> Result<(), CaptureError> {
    use std::process::Command;

    debug!("使用 Linux 截图工具");

    // 尝试使用 gnome-screenshot
    let gnome_result = Command::new("gnome-screenshot")
        .args(["-a", "-f", output_path.to_str().unwrap()])
        .output();

    if let Ok(output) = gnome_result {
        if output.status.success() {
            return Ok(());
        }
    }

    // 回退到 scrot
    let scrot_result = Command::new("scrot")
        .args(["-s", output_path.to_str().unwrap()])
        .output()
        .map_err(|e| {
            CaptureError::SystemError(format!(
                "无法找到截图工具 (gnome-screenshot 或 scrot): {}",
                e
            ))
        })?;

    if !scrot_result.status.success() {
        let stderr = String::from_utf8_lossy(&scrot_result.stderr);
        if !output_path.exists() {
            return Err(CaptureError::Cancelled);
        }
        return Err(CaptureError::SystemError(format!("scrot 失败: {}", stderr)));
    }

    Ok(())
}

/// 清理临时截图文件
///
/// # 参数
/// - `path`: 要删除的文件路径
pub fn cleanup_temp_file(path: &PathBuf) {
    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            error!("删除临时截图文件失败: {}", e);
        } else {
            debug!("已删除临时截图文件: {:?}", path);
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_temp_path_generation() {
        let temp_dir = std::env::temp_dir();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let filename = format!("proxycast_screenshot_{timestamp}.png");
        let temp_path = temp_dir.join(&filename);

        assert!(temp_path
            .to_str()
            .unwrap()
            .contains("proxycast_screenshot_"));
        assert!(temp_path.to_str().unwrap().ends_with(".png"));
    }
}
