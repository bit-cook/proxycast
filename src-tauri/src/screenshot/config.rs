//! 截图对话配置管理
//!
//! 提供实验室功能配置的加载和检查功能

use crate::config::GlobalConfigManagerState;
use proxycast_core::config::{ExperimentalFeatures, ScreenshotChatConfig};
use tauri::{AppHandle, Manager};
use tracing::debug;

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("无法获取应用状态")]
    StateNotFound,
    #[error("配置加载失败: {0}")]
    LoadFailed(String),
}

/// 加载实验室功能配置
///
/// 从应用状态中获取当前的实验室功能配置
///
/// # 参数
/// - `app`: Tauri 应用句柄
///
/// # 返回
/// 成功返回 ExperimentalFeatures，失败返回错误
pub fn load_experimental_config(app: &AppHandle) -> Result<ExperimentalFeatures, ConfigError> {
    debug!("加载实验室功能配置");

    // 获取全局配置管理器
    let config_manager = app
        .try_state::<GlobalConfigManagerState>()
        .ok_or(ConfigError::StateNotFound)?;

    // 从配置中获取实验室功能配置
    let config = config_manager.config();
    let experimental = config.experimental.clone();

    debug!(
        "实验室功能配置: screenshot_chat.enabled={}, shortcut={}",
        experimental.screenshot_chat.enabled, experimental.screenshot_chat.shortcut
    );

    Ok(experimental)
}

/// 检查截图对话功能是否启用
///
/// # 参数
/// - `config`: 实验室功能配置
///
/// # 返回
/// 如果截图对话功能启用返回 true，否则返回 false
pub fn is_screenshot_chat_enabled(config: &ExperimentalFeatures) -> bool {
    config.screenshot_chat.enabled
}

/// 获取截图对话配置
///
/// # 参数
/// - `app`: Tauri 应用句柄
///
/// # 返回
/// 成功返回 ScreenshotChatConfig，失败返回错误
pub fn get_screenshot_chat_config(app: &AppHandle) -> Result<ScreenshotChatConfig, ConfigError> {
    let experimental = load_experimental_config(app)?;
    Ok(experimental.screenshot_chat)
}

/// 保存实验室功能配置
///
/// 将实验室功能配置保存到应用状态和配置文件
///
/// # 参数
/// - `app`: Tauri 应用句柄
/// - `experimental_config`: 要保存的实验室功能配置
///
/// # 返回
/// 成功返回 Ok(()), 失败返回错误
pub async fn save_experimental_config(
    app: &AppHandle,
    experimental_config: ExperimentalFeatures,
) -> Result<(), ConfigError> {
    debug!("保存实验室功能配置");

    // 获取全局配置管理器
    let config_manager = app
        .try_state::<GlobalConfigManagerState>()
        .ok_or(ConfigError::StateNotFound)?;

    // 获取当前配置并更新实验室功能部分
    let mut config = config_manager.config();
    config.experimental = experimental_config;

    // 保存配置
    config_manager
        .save_config(&config)
        .await
        .map_err(ConfigError::LoadFailed)?;

    debug!("实验室功能配置已保存");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_screenshot_chat_enabled_default() {
        let config = ExperimentalFeatures::default();
        assert!(!is_screenshot_chat_enabled(&config));
    }

    #[test]
    fn test_is_screenshot_chat_enabled_true() {
        let config = ExperimentalFeatures {
            screenshot_chat: ScreenshotChatConfig {
                enabled: true,
                shortcut: "CommandOrControl+Shift+S".to_string(),
            },
            ..Default::default()
        };
        assert!(is_screenshot_chat_enabled(&config));
    }

    #[test]
    fn test_default_shortcut() {
        let config = ScreenshotChatConfig::default();
        assert_eq!(config.shortcut, "CommandOrControl+Alt+Q");
        assert!(!config.enabled);
    }
}
