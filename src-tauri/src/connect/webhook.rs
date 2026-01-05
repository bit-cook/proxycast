//! Webhook 回调模块
//!
//! 实现统计回调功能，让中转商追踪推广效果。
//!
//! ## 功能
//!
//! - 在用户确认/取消配置后发送回调
//! - 支持重试机制
//! - 隐私保护（仅发送脱敏数据）
//!
//! ## 回调事件
//!
//! - `success` - 配置成功
//! - `cancelled` - 用户取消
//! - `error` - 配置失败
//!
//! ## 安全说明
//!
//! 由于 ProxyCast 是开源软件，不使用签名验证。
//! 中转商应通过检查 `key_prefix` 是否为自己下发的 Key 来验证请求。
//!
//! _Requirements: 5.3_

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Webhook 错误类型
#[derive(Debug, Error)]
pub enum WebhookError {
    /// 网络请求失败
    #[error("网络请求失败: {0}")]
    NetworkError(String),

    /// 回调地址无效
    #[error("回调地址无效: {0}")]
    InvalidUrl(String),

    /// 重试次数耗尽
    #[error("重试次数耗尽")]
    RetryExhausted,
}

/// 回调状态
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CallbackStatus {
    /// 配置成功
    Success,
    /// 用户取消
    Cancelled,
    /// 配置失败
    Error,
}

impl std::fmt::Display for CallbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallbackStatus::Success => write!(f, "success"),
            CallbackStatus::Cancelled => write!(f, "cancelled"),
            CallbackStatus::Error => write!(f, "error"),
        }
    }
}

/// 客户端信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    /// ProxyCast 版本
    pub version: String,
    /// 平台（macos, windows, linux）
    pub platform: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: get_platform(),
        }
    }
}

/// 获取当前平台
fn get_platform() -> String {
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown".to_string();
}

/// 回调 Payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallbackPayload {
    /// 事件类型
    pub event: String,
    /// 状态
    pub status: CallbackStatus,
    /// 中转商 ID
    pub relay_id: String,
    /// 推广码（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_code: Option<String>,
    /// Key 前缀（脱敏，仅前 7 位）
    pub key_prefix: String,
    /// 事件时间
    pub timestamp: DateTime<Utc>,
    /// 客户端信息
    pub client: ClientInfo,
    /// 错误码（仅 status=error 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 错误信息（仅 status=error 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl CallbackPayload {
    /// 创建成功回调
    pub fn success(relay_id: &str, key: &str, ref_code: Option<String>) -> Self {
        Self {
            event: "connect".to_string(),
            status: CallbackStatus::Success,
            relay_id: relay_id.to_string(),
            ref_code,
            key_prefix: mask_key(key),
            timestamp: Utc::now(),
            client: ClientInfo::default(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建取消回调
    pub fn cancelled(relay_id: &str, key: &str, ref_code: Option<String>) -> Self {
        Self {
            event: "connect".to_string(),
            status: CallbackStatus::Cancelled,
            relay_id: relay_id.to_string(),
            ref_code,
            key_prefix: mask_key(key),
            timestamp: Utc::now(),
            client: ClientInfo::default(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建错误回调
    pub fn error(
        relay_id: &str,
        key: &str,
        ref_code: Option<String>,
        error_code: &str,
        error_message: &str,
    ) -> Self {
        Self {
            event: "connect".to_string(),
            status: CallbackStatus::Error,
            relay_id: relay_id.to_string(),
            ref_code,
            key_prefix: mask_key(key),
            timestamp: Utc::now(),
            client: ClientInfo::default(),
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message.to_string()),
        }
    }
}

/// 脱敏 API Key（仅保留前 7 位）
fn mask_key(key: &str) -> String {
    if key.len() <= 7 {
        key.to_string()
    } else {
        key[..7].to_string()
    }
}

/// Webhook 发送器
pub struct WebhookSender {
    /// HTTP 客户端
    client: reqwest::Client,
    /// 最大重试次数
    max_retries: u32,
    /// 重试间隔（秒）
    retry_intervals: Vec<u64>,
}

impl Default for WebhookSender {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookSender {
    /// 创建新的 WebhookSender
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            max_retries: 4,
            retry_intervals: vec![0, 60, 300, 1800], // 立即, 1分钟, 5分钟, 30分钟
        }
    }

    /// 发送回调（带重试）
    pub async fn send(
        &self,
        callback_url: &str,
        payload: &CallbackPayload,
    ) -> Result<(), WebhookError> {
        // 验证 URL
        if !callback_url.starts_with("https://") {
            return Err(WebhookError::InvalidUrl(
                "回调地址必须使用 HTTPS".to_string(),
            ));
        }

        let payload_json = serde_json::to_string(payload)
            .map_err(|e| WebhookError::NetworkError(e.to_string()))?;

        // 重试循环
        for attempt in 0..self.max_retries {
            // 等待重试间隔
            if attempt > 0 {
                let interval = self.retry_intervals.get(attempt as usize).unwrap_or(&1800);
                tracing::info!("[Webhook] 第 {} 次重试，等待 {} 秒", attempt, interval);
                tokio::time::sleep(Duration::from_secs(*interval)).await;
            }

            match self.send_once(callback_url, &payload_json).await {
                Ok(_) => {
                    tracing::info!(
                        "[Webhook] 回调发送成功: relay={}, status={}",
                        payload.relay_id,
                        payload.status
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "[Webhook] 回调发送失败 (尝试 {}/{}): {}",
                        attempt + 1,
                        self.max_retries,
                        e
                    );
                    if attempt == self.max_retries - 1 {
                        return Err(WebhookError::RetryExhausted);
                    }
                }
            }
        }

        Err(WebhookError::RetryExhausted)
    }

    /// 发送单次请求
    async fn send_once(&self, url: &str, payload: &str) -> Result<(), WebhookError> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                format!("ProxyCast/{}", env!("CARGO_PKG_VERSION")),
            )
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| WebhookError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(WebhookError::NetworkError(format!(
                "HTTP 状态码: {}",
                response.status()
            )))
        }
    }

    /// 异步发送回调（不阻塞，在后台执行）
    pub fn send_async(&self, callback_url: String, payload: CallbackPayload) {
        let client = self.client.clone();
        let max_retries = self.max_retries;
        let retry_intervals = self.retry_intervals.clone();

        tokio::spawn(async move {
            let sender = WebhookSender {
                client,
                max_retries,
                retry_intervals,
            };

            if let Err(e) = sender.send(&callback_url, &payload).await {
                tracing::error!(
                    "[Webhook] 回调最终失败: relay={}, error={}",
                    payload.relay_id,
                    e
                );
            }
        });
    }
}

/// 全局 WebhookSender 实例
static WEBHOOK_SENDER: std::sync::OnceLock<WebhookSender> = std::sync::OnceLock::new();

/// 获取全局 WebhookSender
pub fn get_webhook_sender() -> &'static WebhookSender {
    WEBHOOK_SENDER.get_or_init(WebhookSender::new)
}

/// 发送成功回调
pub fn send_success_callback(
    callback_url: &str,
    relay_id: &str,
    key: &str,
    ref_code: Option<String>,
) {
    let payload = CallbackPayload::success(relay_id, key, ref_code);
    get_webhook_sender().send_async(callback_url.to_string(), payload);
}

/// 发送取消回调
pub fn send_cancelled_callback(
    callback_url: &str,
    relay_id: &str,
    key: &str,
    ref_code: Option<String>,
) {
    let payload = CallbackPayload::cancelled(relay_id, key, ref_code);
    get_webhook_sender().send_async(callback_url.to_string(), payload);
}

/// 发送错误回调
pub fn send_error_callback(
    callback_url: &str,
    relay_id: &str,
    key: &str,
    ref_code: Option<String>,
    error_code: &str,
    error_message: &str,
) {
    let payload = CallbackPayload::error(relay_id, key, ref_code, error_code, error_message);
    get_webhook_sender().send_async(callback_url.to_string(), payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key_short() {
        assert_eq!(mask_key("sk-abc"), "sk-abc");
    }

    #[test]
    fn test_mask_key_long() {
        assert_eq!(mask_key("sk-1234567890abcdef"), "sk-1234");
    }

    #[test]
    fn test_callback_payload_success() {
        let payload =
            CallbackPayload::success("test-relay", "sk-test123456", Some("promo".to_string()));

        assert_eq!(payload.event, "connect");
        assert_eq!(payload.status, CallbackStatus::Success);
        assert_eq!(payload.relay_id, "test-relay");
        assert_eq!(payload.key_prefix, "sk-test");
        assert_eq!(payload.ref_code, Some("promo".to_string()));
        assert!(payload.error_code.is_none());
    }

    #[test]
    fn test_callback_payload_cancelled() {
        let payload = CallbackPayload::cancelled("test-relay", "sk-test123456", None);

        assert_eq!(payload.status, CallbackStatus::Cancelled);
        assert!(payload.ref_code.is_none());
    }

    #[test]
    fn test_callback_payload_error() {
        let payload = CallbackPayload::error(
            "test-relay",
            "sk-test123456",
            None,
            "INVALID_KEY",
            "Key 验证失败",
        );

        assert_eq!(payload.status, CallbackStatus::Error);
        assert_eq!(payload.error_code, Some("INVALID_KEY".to_string()));
        assert_eq!(payload.error_message, Some("Key 验证失败".to_string()));
    }

    #[test]
    fn test_callback_payload_serialization() {
        let payload = CallbackPayload::success("test", "sk-123456789", None);
        let json = serde_json::to_string(&payload).unwrap();

        assert!(json.contains(r#""event":"connect""#));
        assert!(json.contains(r#""status":"success""#));
        assert!(json.contains(r#""relay_id":"test""#));
        // ref_code 为 None 时不应该出现在 JSON 中
        assert!(!json.contains("ref_code"));
    }
}
