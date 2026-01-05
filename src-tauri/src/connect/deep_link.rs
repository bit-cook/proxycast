//! Deep Link URL 解析模块
//!
//! 负责解析 `proxycast://connect` 协议的 URL，提取中转商配置参数。
//!
//! ## 功能
//!
//! - 解析 Deep Link URL 并提取参数
//! - 验证必填参数（relay, key）
//! - 返回结构化的 ConnectPayload 或错误
//!
//! ## 使用示例
//!
//! ```rust
//! use proxycast_lib::connect::deep_link::{parse_deep_link, ConnectPayload, DeepLinkError};
//!
//! let url = "proxycast://connect?relay=example&key=sk-xxx&name=MyKey";
//! match parse_deep_link(url) {
//!     Ok(payload) => println!("Relay: {}, Key: {}", payload.relay, payload.key),
//!     Err(e) => eprintln!("Error: {:?}", e),
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Deep Link 解析结果
///
/// 包含从 `proxycast://connect` URL 中提取的所有参数。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectPayload {
    /// 中转商 ID（必填）
    pub relay: String,
    /// API Key（必填）
    pub key: String,
    /// Key 名称（可选）
    pub name: Option<String>,
    /// 推广码（可选）
    pub ref_code: Option<String>,
}

/// Deep Link 解析错误
///
/// 表示解析 Deep Link URL 时可能发生的各种错误。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeepLinkError {
    /// URL 格式无效
    InvalidUrl(String),
    /// 缺少必填的 relay 参数
    MissingRelay,
    /// 缺少必填的 key 参数
    MissingKey,
}

impl std::fmt::Display for DeepLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeepLinkError::InvalidUrl(msg) => write!(f, "无效的 URL: {}", msg),
            DeepLinkError::MissingRelay => write!(f, "缺少必填参数: relay"),
            DeepLinkError::MissingKey => write!(f, "缺少必填参数: key"),
        }
    }
}

impl std::error::Error for DeepLinkError {}

/// 解析 Deep Link URL
///
/// 解析 `proxycast://connect` 格式的 URL，提取 relay、key、name 和 ref 参数。
///
/// # 参数
///
/// * `url` - Deep Link URL 字符串
///
/// # 返回值
///
/// * `Ok(ConnectPayload)` - 解析成功，返回包含所有参数的结构体
/// * `Err(DeepLinkError)` - 解析失败，返回具体错误类型
///
/// # 示例
///
/// ```rust
/// use proxycast_lib::connect::deep_link::parse_deep_link;
///
/// // 完整 URL
/// let result = parse_deep_link("proxycast://connect?relay=example&key=sk-xxx&name=MyKey&ref=abc");
/// assert!(result.is_ok());
///
/// // 缺少 relay 参数
/// let result = parse_deep_link("proxycast://connect?key=sk-xxx");
/// assert!(result.is_err());
/// ```
pub fn parse_deep_link(url: &str) -> Result<ConnectPayload, DeepLinkError> {
    // 解析 URL
    let parsed = Url::parse(url).map_err(|e| DeepLinkError::InvalidUrl(e.to_string()))?;

    // 验证协议
    if parsed.scheme() != "proxycast" {
        return Err(DeepLinkError::InvalidUrl(format!(
            "无效的协议: {}，期望 proxycast",
            parsed.scheme()
        )));
    }

    // 验证路径（host 在自定义协议中作为路径的一部分）
    if parsed.host_str() != Some("connect") {
        return Err(DeepLinkError::InvalidUrl(format!(
            "无效的路径: {:?}，期望 connect",
            parsed.host_str()
        )));
    }

    // 提取查询参数
    let params: HashMap<String, String> = parsed.query_pairs().into_owned().collect();

    // 验证并提取必填参数
    let relay = params
        .get("relay")
        .filter(|s| !s.is_empty())
        .ok_or(DeepLinkError::MissingRelay)?
        .clone();

    let key = params
        .get("key")
        .filter(|s| !s.is_empty())
        .ok_or(DeepLinkError::MissingKey)?
        .clone();

    // 提取可选参数
    let name = params.get("name").filter(|s| !s.is_empty()).cloned();
    let ref_code = params.get("ref").filter(|s| !s.is_empty()).cloned();

    Ok(ConnectPayload {
        relay,
        key,
        name,
        ref_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_url_with_all_params() {
        let url = "proxycast://connect?relay=example&key=sk-xxx&name=MyKey&ref=abc123";
        let result = parse_deep_link(url).unwrap();

        assert_eq!(result.relay, "example");
        assert_eq!(result.key, "sk-xxx");
        assert_eq!(result.name, Some("MyKey".to_string()));
        assert_eq!(result.ref_code, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_valid_url_with_required_params_only() {
        let url = "proxycast://connect?relay=test-relay&key=sk-12345";
        let result = parse_deep_link(url).unwrap();

        assert_eq!(result.relay, "test-relay");
        assert_eq!(result.key, "sk-12345");
        assert_eq!(result.name, None);
        assert_eq!(result.ref_code, None);
    }

    #[test]
    fn test_parse_missing_relay() {
        let url = "proxycast://connect?key=sk-xxx";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::MissingRelay)));
    }

    #[test]
    fn test_parse_missing_key() {
        let url = "proxycast://connect?relay=example";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::MissingKey)));
    }

    #[test]
    fn test_parse_empty_relay() {
        let url = "proxycast://connect?relay=&key=sk-xxx";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::MissingRelay)));
    }

    #[test]
    fn test_parse_empty_key() {
        let url = "proxycast://connect?relay=example&key=";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::MissingKey)));
    }

    #[test]
    fn test_parse_invalid_protocol() {
        let url = "http://connect?relay=example&key=sk-xxx";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::InvalidUrl(_))));
    }

    #[test]
    fn test_parse_invalid_path() {
        let url = "proxycast://other?relay=example&key=sk-xxx";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::InvalidUrl(_))));
    }

    #[test]
    fn test_parse_malformed_url() {
        let url = "not a valid url";
        let result = parse_deep_link(url);

        assert!(matches!(result, Err(DeepLinkError::InvalidUrl(_))));
    }

    #[test]
    fn test_parse_url_encoded_params() {
        let url = "proxycast://connect?relay=test%20relay&key=sk-xxx&name=My%20Key";
        let result = parse_deep_link(url).unwrap();

        assert_eq!(result.relay, "test relay");
        assert_eq!(result.key, "sk-xxx");
        assert_eq!(result.name, Some("My Key".to_string()));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// 生成有效的 relay ID（字母数字和连字符）
    fn arb_relay_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,30}[a-z0-9]".prop_filter("非空", |s| !s.is_empty())
    }

    /// 生成有效的 API Key
    fn arb_api_key() -> impl Strategy<Value = String> {
        "sk-[a-zA-Z0-9]{8,64}".prop_filter("非空", |s| !s.is_empty())
    }

    /// 生成可选的 Key 名称
    fn arb_key_name() -> impl Strategy<Value = Option<String>> {
        prop_oneof![Just(None), "[a-zA-Z0-9 _-]{1,50}".prop_map(Some),]
    }

    /// 生成可选的推广码
    fn arb_ref_code() -> impl Strategy<Value = Option<String>> {
        prop_oneof![Just(None), "[a-zA-Z0-9]{4,20}".prop_map(Some),]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: proxycast-connect, Property 1: Deep Link URL Parsing Completeness
        /// Validates: Requirements 1.1
        ///
        /// *For any* valid Deep Link URL containing relay and key parameters,
        /// parsing the URL SHALL extract all parameters correctly and return
        /// a ConnectPayload with matching values.
        #[test]
        fn prop_deep_link_parsing_completeness(
            relay in arb_relay_id(),
            key in arb_api_key(),
            name in arb_key_name(),
            ref_code in arb_ref_code(),
        ) {
            // 构建 URL
            let mut url = format!("proxycast://connect?relay={}&key={}", relay, key);
            if let Some(ref n) = name {
                url.push_str(&format!("&name={}", urlencoding::encode(n)));
            }
            if let Some(ref r) = ref_code {
                url.push_str(&format!("&ref={}", r));
            }

            // 解析 URL
            let result = parse_deep_link(&url);

            // 验证解析成功
            prop_assert!(result.is_ok(), "解析失败: {:?}", result);

            let payload = result.unwrap();

            // 验证所有参数正确提取
            prop_assert_eq!(&payload.relay, &relay, "relay 不匹配");
            prop_assert_eq!(&payload.key, &key, "key 不匹配");
            prop_assert_eq!(&payload.name, &name, "name 不匹配");
            prop_assert_eq!(&payload.ref_code, &ref_code, "ref_code 不匹配");
        }

        /// Feature: proxycast-connect, Property 2: Deep Link Invalid Parameter Handling
        /// Validates: Requirements 1.2, 1.3, 7.1
        ///
        /// *For any* Deep Link URL that is malformed, missing the relay parameter,
        /// or missing the key parameter, the parser SHALL return an appropriate error
        /// and not produce a valid ConnectPayload.
        #[test]
        fn prop_deep_link_invalid_parameter_handling(
            relay in arb_relay_id(),
            key in arb_api_key(),
            error_type in 0..4u8,
        ) {
            let url = match error_type {
                // 缺少 relay 参数
                0 => format!("proxycast://connect?key={}", key),
                // 缺少 key 参数
                1 => format!("proxycast://connect?relay={}", relay),
                // 空 relay 参数
                2 => format!("proxycast://connect?relay=&key={}", key),
                // 空 key 参数
                _ => format!("proxycast://connect?relay={}&key=", relay),
            };

            let result = parse_deep_link(&url);

            // 验证解析失败
            prop_assert!(result.is_err(), "应该返回错误，但解析成功了: {:?}", result);

            // 验证错误类型正确
            match error_type {
                0 | 2 => prop_assert!(
                    matches!(result, Err(DeepLinkError::MissingRelay)),
                    "期望 MissingRelay 错误，实际: {:?}", result
                ),
                1 | 3 => prop_assert!(
                    matches!(result, Err(DeepLinkError::MissingKey)),
                    "期望 MissingKey 错误，实际: {:?}", result
                ),
                _ => unreachable!(),
            }
        }

        /// 测试无效协议
        #[test]
        fn prop_invalid_protocol(
            protocol in "(http|https|ftp|file)",
            relay in arb_relay_id(),
            key in arb_api_key(),
        ) {
            let url = format!("{}://connect?relay={}&key={}", protocol, relay, key);
            let result = parse_deep_link(&url);

            prop_assert!(
                matches!(result, Err(DeepLinkError::InvalidUrl(_))),
                "期望 InvalidUrl 错误，实际: {:?}", result
            );
        }

        /// 测试无效路径
        #[test]
        fn prop_invalid_path(
            path in "(other|invalid|test|api)",
            relay in arb_relay_id(),
            key in arb_api_key(),
        ) {
            let url = format!("proxycast://{}?relay={}&key={}", path, relay, key);
            let result = parse_deep_link(&url);

            prop_assert!(
                matches!(result, Err(DeepLinkError::InvalidUrl(_))),
                "期望 InvalidUrl 错误，实际: {:?}", result
            );
        }
    }
}
