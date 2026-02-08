//! 托盘状态模块
//!
//! 定义托盘图标状态和状态快照结构

use serde::{Deserialize, Serialize};

/// 托盘图标状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayIconStatus {
    /// 正常运行（绿色）- 服务器运行且凭证健康
    Running,
    /// 警告状态（黄色）- 有凭证即将过期或余额不足
    Warning,
    /// 错误状态（红色）- 服务器停止或所有凭证无效
    Error,
    /// 停止状态（灰色）- 服务器未启动
    Stopped,
}

impl Default for TrayIconStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

/// 凭证健康状态
#[derive(Debug, Clone, Default)]
pub struct CredentialHealth {
    /// 凭证是否有效
    pub is_valid: bool,
    /// 是否即将过期
    pub is_expiring_soon: bool,
    /// 是否余额不足
    pub is_low_balance: bool,
}

impl CredentialHealth {
    /// 创建健康的凭证状态
    pub fn healthy() -> Self {
        Self {
            is_valid: true,
            is_expiring_soon: false,
            is_low_balance: false,
        }
    }

    /// 创建无效的凭证状态
    pub fn invalid() -> Self {
        Self {
            is_valid: false,
            is_expiring_soon: false,
            is_low_balance: false,
        }
    }

    /// 检查凭证是否有警告
    pub fn has_warning(&self) -> bool {
        self.is_valid && (self.is_expiring_soon || self.is_low_balance)
    }
}

/// 托盘状态快照
#[derive(Debug, Clone, Serialize)]
pub struct TrayStateSnapshot {
    /// 图标状态
    pub icon_status: TrayIconStatus,
    /// 服务器是否运行
    pub server_running: bool,
    /// 服务器地址
    pub server_address: String,
    /// 可用凭证数
    pub available_credentials: usize,
    /// 总凭证数
    pub total_credentials: usize,
    /// 今日请求数
    pub today_requests: u64,
    /// 是否开机自启
    pub auto_start_enabled: bool,
}

impl Default for TrayStateSnapshot {
    fn default() -> Self {
        Self {
            icon_status: TrayIconStatus::Stopped,
            server_running: false,
            server_address: String::new(),
            available_credentials: 0,
            total_credentials: 0,
            today_requests: 0,
            auto_start_enabled: false,
        }
    }
}

/// 根据服务器状态和凭证健康状态计算托盘图标状态
///
/// # 规则
/// - 服务器未运行 -> Stopped
/// - 服务器运行 + 所有凭证无效 -> Error
/// - 服务器运行 + 有凭证警告 -> Warning
/// - 服务器运行 + 所有凭证健康 -> Running
pub fn calculate_icon_status(
    server_running: bool,
    credentials: &[CredentialHealth],
) -> TrayIconStatus {
    if !server_running {
        return TrayIconStatus::Stopped;
    }

    if credentials.is_empty() {
        return TrayIconStatus::Error;
    }

    let all_invalid = credentials.iter().all(|c| !c.is_valid);
    if all_invalid {
        return TrayIconStatus::Error;
    }

    let has_warning = credentials.iter().any(|c| c.has_warning());
    if has_warning {
        return TrayIconStatus::Warning;
    }

    TrayIconStatus::Running
}

/// 将凭证池健康数据转换为托盘所需的健康状态
pub fn get_credential_health_from_pool(
    pool_credentials: &[(String, bool, bool, bool)], // (id, is_valid, is_expiring_soon, is_low_balance)
) -> Vec<CredentialHealth> {
    pool_credentials
        .iter()
        .map(
            |(_, is_valid, is_expiring_soon, is_low_balance)| CredentialHealth {
                is_valid: *is_valid,
                is_expiring_soon: *is_expiring_soon,
                is_low_balance: *is_low_balance,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_credential_health() -> impl Strategy<Value = CredentialHealth> {
        (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
            |(is_valid, is_expiring_soon, is_low_balance)| CredentialHealth {
                is_valid,
                is_expiring_soon,
                is_low_balance,
            },
        )
    }

    proptest! {
        /// **Feature: system-tray, Property 1: 状态到图标映射正确性**
        /// **Validates: Requirements 1.1, 1.2, 1.3**
        #[test]
        fn prop_icon_status_mapping(
            server_running in any::<bool>(),
            credentials in prop::collection::vec(arb_credential_health(), 0..10)
        ) {
            let status = calculate_icon_status(server_running, &credentials);

            if !server_running {
                prop_assert_eq!(status, TrayIconStatus::Stopped);
                return Ok(());
            }

            if credentials.is_empty() {
                prop_assert_eq!(status, TrayIconStatus::Error);
                return Ok(());
            }

            let all_invalid = credentials.iter().all(|c| !c.is_valid);
            if all_invalid {
                prop_assert_eq!(status, TrayIconStatus::Error);
                return Ok(());
            }

            let has_warning = credentials.iter().any(|c| c.has_warning());
            if has_warning {
                prop_assert_eq!(status, TrayIconStatus::Warning);
                return Ok(());
            }

            prop_assert_eq!(status, TrayIconStatus::Running);
        }
    }

    #[test]
    fn test_credential_health_healthy() {
        let health = CredentialHealth::healthy();
        assert!(health.is_valid);
        assert!(!health.is_expiring_soon);
        assert!(!health.is_low_balance);
        assert!(!health.has_warning());
    }

    #[test]
    fn test_credential_health_invalid() {
        let health = CredentialHealth::invalid();
        assert!(!health.is_valid);
        assert!(!health.has_warning());
    }

    #[test]
    fn test_credential_health_warning() {
        let mut health = CredentialHealth::healthy();
        health.is_expiring_soon = true;
        assert!(health.has_warning());

        let mut health2 = CredentialHealth::healthy();
        health2.is_low_balance = true;
        assert!(health2.has_warning());
    }

    #[test]
    fn test_calculate_icon_status_server_stopped() {
        let credentials = vec![CredentialHealth::healthy()];
        let status = calculate_icon_status(false, &credentials);
        assert_eq!(status, TrayIconStatus::Stopped);
    }

    #[test]
    fn test_calculate_icon_status_no_credentials() {
        let credentials = vec![];
        let status = calculate_icon_status(true, &credentials);
        assert_eq!(status, TrayIconStatus::Error);
    }

    #[test]
    fn test_calculate_icon_status_all_invalid() {
        let credentials = vec![CredentialHealth::invalid(), CredentialHealth::invalid()];
        let status = calculate_icon_status(true, &credentials);
        assert_eq!(status, TrayIconStatus::Error);
    }

    #[test]
    fn test_calculate_icon_status_with_warning() {
        let credentials = vec![
            CredentialHealth::healthy(),
            CredentialHealth {
                is_valid: true,
                is_expiring_soon: true,
                is_low_balance: false,
            },
        ];
        let status = calculate_icon_status(true, &credentials);
        assert_eq!(status, TrayIconStatus::Warning);
    }

    #[test]
    fn test_calculate_icon_status_running() {
        let credentials = vec![CredentialHealth::healthy(), CredentialHealth::healthy()];
        let status = calculate_icon_status(true, &credentials);
        assert_eq!(status, TrayIconStatus::Running);
    }

    #[test]
    fn test_get_credential_health_from_pool() {
        let pool_data = vec![
            ("cred1".to_string(), true, false, false),
            ("cred2".to_string(), true, true, false),
            ("cred3".to_string(), false, false, false),
        ];

        let health = get_credential_health_from_pool(&pool_data);

        assert_eq!(health.len(), 3);
        assert!(health[0].is_valid);
        assert!(!health[0].is_expiring_soon);
        assert!(health[1].is_valid);
        assert!(health[1].is_expiring_soon);
        assert!(!health[2].is_valid);
    }

    #[test]
    fn test_get_credential_health_empty() {
        let pool_data: Vec<(String, bool, bool, bool)> = vec![];
        let health = get_credential_health_from_pool(&pool_data);
        assert!(health.is_empty());
    }
}
