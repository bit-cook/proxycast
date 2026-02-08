//! 托盘菜单元数据模块
//!
//! 提供与 Tauri 无关的菜单 ID、地址解析和菜单元信息工具函数。

/// 菜单项 ID 常量
pub mod menu_ids {
    /// 状态信息
    pub const STATUS_INFO: &str = "status_info";
    /// 凭证信息
    pub const CREDENTIAL_INFO: &str = "credential_info";
    /// 请求信息
    pub const REQUEST_INFO: &str = "request_info";
    /// 分隔符 1
    pub const SEPARATOR_1: &str = "sep_1";
    /// 启动服务器
    pub const START_SERVER: &str = "start_server";
    /// 停止服务器
    pub const STOP_SERVER: &str = "stop_server";
    /// 刷新所有 Token
    pub const REFRESH_TOKENS: &str = "refresh_tokens";
    /// 健康检查
    pub const HEALTH_CHECK: &str = "health_check";
    /// 分隔符 2
    pub const SEPARATOR_2: &str = "sep_2";
    /// 打开主窗口
    pub const OPEN_WINDOW: &str = "open_window";
    /// 复制 API 地址
    pub const COPY_API_ADDRESS: &str = "copy_api_address";
    /// 打开日志目录
    pub const OPEN_LOG_DIR: &str = "open_log_dir";
    /// 分隔符 3
    pub const SEPARATOR_3: &str = "sep_3";
    /// 开机自启
    pub const AUTO_START: &str = "auto_start";
    /// 分隔符 4
    pub const SEPARATOR_4: &str = "sep_4";
    /// 退出
    pub const QUIT: &str = "quit";

    /// 获取所有必需的菜单项 ID 列表
    pub fn all_required_ids() -> Vec<&'static str> {
        vec![
            STATUS_INFO,
            CREDENTIAL_INFO,
            REQUEST_INFO,
            START_SERVER,
            STOP_SERVER,
            REFRESH_TOKENS,
            HEALTH_CHECK,
            OPEN_WINDOW,
            COPY_API_ADDRESS,
            OPEN_LOG_DIR,
            AUTO_START,
            QUIT,
        ]
    }
}

/// 解析服务器地址字符串为 host 和 port
///
/// 支持格式：
/// - "host:port" -> (host, port)
/// - "host" -> (host, 8080)
/// - "" -> ("127.0.0.1", 8080)
pub fn parse_server_address(address: &str) -> (String, u16) {
    if address.is_empty() {
        return ("127.0.0.1".to_string(), 8080);
    }

    if let Some((host, port_str)) = address.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return (host.to_string(), port);
        }
    }

    (address.to_string(), 8080)
}

/// 获取菜单中包含的所有菜单项 ID
pub fn get_menu_item_ids() -> Vec<&'static str> {
    menu_ids::all_required_ids()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_all_required_ids_not_empty() {
        let ids = menu_ids::all_required_ids();
        assert!(!ids.is_empty(), "必需的菜单项 ID 列表不应为空");
    }

    #[test]
    fn test_all_required_ids_unique() {
        let ids = menu_ids::all_required_ids();
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len(), "菜单项 ID 应该唯一");
    }

    #[test]
    fn test_menu_ids_completeness() {
        let ids = menu_ids::all_required_ids();

        assert!(ids.contains(&menu_ids::STATUS_INFO), "应包含 STATUS_INFO");
        assert!(
            ids.contains(&menu_ids::CREDENTIAL_INFO),
            "应包含 CREDENTIAL_INFO"
        );
        assert!(ids.contains(&menu_ids::REQUEST_INFO), "应包含 REQUEST_INFO");
        assert!(ids.contains(&menu_ids::START_SERVER), "应包含 START_SERVER");
        assert!(ids.contains(&menu_ids::STOP_SERVER), "应包含 STOP_SERVER");
        assert!(
            ids.contains(&menu_ids::REFRESH_TOKENS),
            "应包含 REFRESH_TOKENS"
        );
        assert!(ids.contains(&menu_ids::HEALTH_CHECK), "应包含 HEALTH_CHECK");
        assert!(ids.contains(&menu_ids::OPEN_WINDOW), "应包含 OPEN_WINDOW");
        assert!(
            ids.contains(&menu_ids::COPY_API_ADDRESS),
            "应包含 COPY_API_ADDRESS"
        );
        assert!(ids.contains(&menu_ids::OPEN_LOG_DIR), "应包含 OPEN_LOG_DIR");
        assert!(ids.contains(&menu_ids::AUTO_START), "应包含 AUTO_START");
        assert!(ids.contains(&menu_ids::QUIT), "应包含 QUIT");
    }

    #[test]
    fn test_parse_server_address_with_port() {
        let (host, port) = parse_server_address("127.0.0.1:8080");
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_server_address_without_port() {
        let (host, port) = parse_server_address("localhost");
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_server_address_empty() {
        let (host, port) = parse_server_address("");
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_server_address_ipv6() {
        let (host, port) = parse_server_address("[::1]:9000");
        assert_eq!(host, "[::1]");
        assert_eq!(port, 9000);
    }

    #[test]
    fn test_get_menu_item_ids() {
        let ids = get_menu_item_ids();
        assert_eq!(ids.len(), 12, "应有 12 个必需的菜单项");
    }

    proptest! {
        #[test]
        fn prop_menu_ids_completeness(
            _server_running in any::<bool>(),
            _available in 0usize..100,
            _total in 0usize..100,
            _requests in 0u64..1000000,
            _auto_start in any::<bool>()
        ) {
            let ids = menu_ids::all_required_ids();

            let required = vec![
                menu_ids::STATUS_INFO,
                menu_ids::CREDENTIAL_INFO,
                menu_ids::REQUEST_INFO,
                menu_ids::START_SERVER,
                menu_ids::STOP_SERVER,
                menu_ids::REFRESH_TOKENS,
                menu_ids::HEALTH_CHECK,
                menu_ids::OPEN_WINDOW,
                menu_ids::COPY_API_ADDRESS,
                menu_ids::OPEN_LOG_DIR,
                menu_ids::AUTO_START,
                menu_ids::QUIT,
            ];

            for id in required {
                prop_assert!(ids.contains(&id), "菜单项列表应包含 {}", id);
            }

            let mut sorted_ids = ids.clone();
            sorted_ids.sort();
            sorted_ids.dedup();
            prop_assert_eq!(ids.len(), sorted_ids.len(), "菜单项 ID 应该唯一");
        }
    }
}
