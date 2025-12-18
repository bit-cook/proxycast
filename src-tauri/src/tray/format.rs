//! 菜单文本格式化模块
//!
//! 提供托盘菜单文本的格式化函数

/// 格式化服务器状态文本
///
/// # 示例输出
/// - "🟢 API 服务器: 运行中 (127.0.0.1:8080)"
/// - "⚪ API 服务器: 已停止"
pub fn format_server_status(running: bool, host: &str, port: u16) -> String {
    if running {
        format!("🟢 API 服务器: 运行中 ({host}:{port})")
    } else {
        "⚪ API 服务器: 已停止".to_string()
    }
}

/// 格式化凭证状态文本
///
/// # 示例输出
/// - "🔑 可用凭证: 3/5"
pub fn format_credential_status(available: usize, total: usize) -> String {
    format!("🔑 可用凭证: {available}/{total}")
}

/// 格式化请求统计文本
///
/// # 示例输出
/// - "📊 今日请求: 128 次"
pub fn format_request_count(count: u64) -> String {
    format!("📊 今日请求: {count} 次")
}

/// 格式化 API 地址
///
/// # 示例输出
/// - "http://127.0.0.1:8080"
pub fn format_api_address(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// **Feature: system-tray, Property 2: 菜单内容格式化正确性**
        /// **Validates: Requirements 2.2, 2.3, 2.4**
        #[test]
        fn prop_menu_content_formatting(
            host in "[a-z0-9.]{1,50}",
            port in 1024u16..65535,
            available in 0usize..100,
            total in 0usize..100,
            requests in 0u64..1000000
        ) {
            // 测试服务器状态格式化 - 运行中
            let running_status = format_server_status(true, &host, port);
            prop_assert!(running_status.contains(&host), "运行状态应包含 host");
            prop_assert!(running_status.contains(&port.to_string()), "运行状态应包含 port");
            prop_assert!(running_status.contains("运行中"), "运行状态应包含'运行中'");

            // 测试服务器状态格式化 - 已停止
            let stopped_status = format_server_status(false, &host, port);
            prop_assert!(stopped_status.contains("已停止"), "停止状态应包含'已停止'");

            // 测试凭证状态格式化
            let cred_status = format_credential_status(available, total);
            prop_assert!(cred_status.contains(&available.to_string()), "凭证状态应包含可用数");
            prop_assert!(cred_status.contains(&total.to_string()), "凭证状态应包含总数");

            // 测试请求统计格式化
            let req_status = format_request_count(requests);
            prop_assert!(req_status.contains(&requests.to_string()), "请求统计应包含请求次数");
        }

        /// **Feature: system-tray, Property 4: API 地址格式化正确性**
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_api_address_formatting(
            host in "[a-z0-9.]{1,50}",
            port in 1024u16..65535
        ) {
            let address = format_api_address(&host, port);
            let expected = format!("http://{host}:{port}");
            prop_assert_eq!(address, expected, "API 地址格式应为 http://{{host}}:{{port}}");
        }
    }

    #[test]
    fn test_format_server_status_running() {
        let status = format_server_status(true, "127.0.0.1", 8080);
        assert_eq!(status, "🟢 API 服务器: 运行中 (127.0.0.1:8080)");
    }

    #[test]
    fn test_format_server_status_stopped() {
        let status = format_server_status(false, "127.0.0.1", 8080);
        assert_eq!(status, "⚪ API 服务器: 已停止");
    }

    #[test]
    fn test_format_credential_status() {
        let status = format_credential_status(3, 5);
        assert_eq!(status, "🔑 可用凭证: 3/5");
    }

    #[test]
    fn test_format_request_count() {
        let status = format_request_count(128);
        assert_eq!(status, "📊 今日请求: 128 次");
    }

    #[test]
    fn test_format_api_address() {
        let address = format_api_address("127.0.0.1", 8080);
        assert_eq!(address, "http://127.0.0.1:8080");
    }
}
