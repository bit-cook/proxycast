//! 参数注入模块
//!
//! 提供请求参数注入功能，支持：
//! - 模型通配符匹配规则
//! - merge 和 override 两种注入模式
//! - 规则优先级排序

mod types;

pub use types::{InjectionConfig, InjectionMode, InjectionResult, InjectionRule, Injector};

#[cfg(test)]
mod tests;
