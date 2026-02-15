//! 配置观察者 Trait 定义
//!
//! 定义观察者接口，支持异步和同步两种模式

use super::events::ConfigChangeEvent;
use async_trait::async_trait;
use proxycast_core::config::Config;
use std::sync::Arc;

/// 配置观察者 Trait
///
/// 实现此 Trait 的组件可以订阅配置变更通知
#[async_trait]
pub trait ConfigObserver: Send + Sync {
    /// 观察者名称（用于日志和调试）
    fn name(&self) -> &str;

    /// 处理配置变更事件
    async fn on_config_changed(
        &self,
        event: &ConfigChangeEvent,
        config: &Config,
    ) -> Result<(), String>;

    /// 是否对特定事件类型感兴趣（默认对所有事件感兴趣）
    fn is_interested_in(&self, _event: &ConfigChangeEvent) -> bool {
        true
    }

    /// 观察者优先级（数字越小优先级越高，默认 100）
    fn priority(&self) -> i32 {
        100
    }
}

/// 同步配置观察者 Trait（用于不需要异步的简单观察者）
#[allow(dead_code)]
pub trait SyncConfigObserver: Send + Sync {
    /// 观察者名称
    fn name(&self) -> &str;

    /// 同步处理配置变更
    fn on_config_changed_sync(
        &self,
        event: &ConfigChangeEvent,
        config: &Config,
    ) -> Result<(), String>;

    /// 是否对特定事件类型感兴趣
    fn is_interested_in(&self, _event: &ConfigChangeEvent) -> bool {
        true
    }

    /// 观察者优先级
    fn priority(&self) -> i32 {
        100
    }
}

/// 将同步观察者包装为异步观察者
#[allow(dead_code)]
pub struct SyncObserverWrapper<T: SyncConfigObserver>(pub Arc<T>);

#[async_trait]
impl<T: SyncConfigObserver + 'static> ConfigObserver for SyncObserverWrapper<T> {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn on_config_changed(
        &self,
        event: &ConfigChangeEvent,
        config: &Config,
    ) -> Result<(), String> {
        self.0.on_config_changed_sync(event, config)
    }

    fn is_interested_in(&self, event: &ConfigChangeEvent) -> bool {
        self.0.is_interested_in(event)
    }

    fn priority(&self) -> i32 {
        self.0.priority()
    }
}

/// 函数式观察者（用于简单的回调场景）
#[allow(dead_code)]
pub struct FnObserver<F>
where
    F: Fn(&ConfigChangeEvent, &Config) -> Result<(), String> + Send + Sync,
{
    name: String,
    priority: i32,
    handler: F,
}

impl<F> FnObserver<F>
where
    F: Fn(&ConfigChangeEvent, &Config) -> Result<(), String> + Send + Sync,
{
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            name: name.into(),
            priority: 100,
            handler,
        }
    }

    #[allow(dead_code)]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

#[async_trait]
impl<F> ConfigObserver for FnObserver<F>
where
    F: Fn(&ConfigChangeEvent, &Config) -> Result<(), String> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn on_config_changed(
        &self,
        event: &ConfigChangeEvent,
        config: &Config,
    ) -> Result<(), String> {
        (self.handler)(event, config)
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestObserver {
        name: String,
        priority: i32,
    }

    #[async_trait]
    impl ConfigObserver for TestObserver {
        fn name(&self) -> &str {
            &self.name
        }

        async fn on_config_changed(
            &self,
            _event: &ConfigChangeEvent,
            _config: &Config,
        ) -> Result<(), String> {
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_observer_priority() {
        let observer = TestObserver {
            name: "test".to_string(),
            priority: 50,
        };
        assert_eq!(observer.priority(), 50);
    }

    #[test]
    fn test_fn_observer() {
        let observer = FnObserver::new("fn_test", |_event, _config| Ok(())).with_priority(10);
        assert_eq!(observer.name(), "fn_test");
        assert_eq!(observer.priority(), 10);
    }
}
