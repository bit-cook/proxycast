//! 参数注入模块测试

use super::*;
use serde_json::json;

#[cfg(test)]
mod rule_tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let rule = InjectionRule::new("r1", "claude-sonnet-4-5", json!({"temperature": 0.7}));

        assert!(rule.matches("claude-sonnet-4-5"));
        assert!(!rule.matches("claude-sonnet-4-5-20250514"));
        assert!(!rule.matches("claude-opus"));
    }

    #[test]
    fn test_prefix_match() {
        let rule = InjectionRule::new("r1", "claude-*", json!({"temperature": 0.7}));

        assert!(rule.matches("claude-sonnet-4-5"));
        assert!(rule.matches("claude-opus"));
        assert!(!rule.matches("gemini-2.5-flash"));
    }

    #[test]
    fn test_suffix_match() {
        let rule = InjectionRule::new("r1", "*-preview", json!({"temperature": 0.7}));

        assert!(rule.matches("gemini-2.5-pro-preview"));
        assert!(rule.matches("claude-preview"));
        assert!(!rule.matches("gemini-2.5-flash"));
    }

    #[test]
    fn test_contains_match() {
        let rule = InjectionRule::new("r1", "*flash*", json!({"temperature": 0.7}));

        assert!(rule.matches("gemini-2.5-flash"));
        assert!(rule.matches("gemini-flash-preview"));
        assert!(!rule.matches("gemini-2.5-pro"));
    }

    #[test]
    fn test_disabled_rule() {
        let mut rule = InjectionRule::new("r1", "claude-*", json!({"temperature": 0.7}));
        rule.enabled = false;

        assert!(!rule.matches("claude-sonnet-4-5"));
    }

    #[test]
    fn test_rule_ordering() {
        let exact = InjectionRule::new("r1", "claude-sonnet-4-5", json!({})).with_priority(20);
        let wildcard = InjectionRule::new("r2", "claude-*", json!({})).with_priority(10);

        // 精确匹配应优先于通配符
        assert!(exact < wildcard);
    }
}

#[cfg(test)]
mod injector_tests {
    use super::*;

    #[test]
    fn test_inject_merge_mode() {
        let mut injector = Injector::new();
        injector.add_rule(InjectionRule::new(
            "r1",
            "claude-*",
            json!({"temperature": 0.7, "max_tokens": 1000}),
        ));

        let mut payload = json!({
            "model": "claude-sonnet-4-5",
            "messages": []
        });

        let result = injector.inject("claude-sonnet-4-5", &mut payload);

        assert!(result.has_injections());
        assert_eq!(result.applied_rules, vec!["r1"]);
        assert_eq!(payload["temperature"], 0.7);
        assert_eq!(payload["max_tokens"], 1000);
    }

    #[test]
    fn test_inject_merge_no_override() {
        let mut injector = Injector::new();
        injector.add_rule(InjectionRule::new(
            "r1",
            "claude-*",
            json!({"temperature": 0.7}),
        ));

        let mut payload = json!({
            "model": "claude-sonnet-4-5",
            "temperature": 0.5,
            "messages": []
        });

        let result = injector.inject("claude-sonnet-4-5", &mut payload);

        // 已有参数不应被覆盖
        assert!(!result.has_injections());
        assert_eq!(payload["temperature"], 0.5);
    }

    #[test]
    fn test_inject_override_mode() {
        let mut injector = Injector::new();
        injector.add_rule(
            InjectionRule::new("r1", "claude-*", json!({"temperature": 0.7}))
                .with_mode(InjectionMode::Override),
        );

        let mut payload = json!({
            "model": "claude-sonnet-4-5",
            "temperature": 0.5,
            "messages": []
        });

        let result = injector.inject("claude-sonnet-4-5", &mut payload);

        // Override 模式应覆盖已有参数
        assert!(result.has_injections());
        assert_eq!(payload["temperature"], 0.7);
    }

    #[test]
    fn test_inject_no_match() {
        let mut injector = Injector::new();
        injector.add_rule(InjectionRule::new(
            "r1",
            "claude-*",
            json!({"temperature": 0.7}),
        ));

        let mut payload = json!({
            "model": "gemini-2.5-flash",
            "messages": []
        });

        let result = injector.inject("gemini-2.5-flash", &mut payload);

        assert!(!result.has_injections());
        assert!(payload.get("temperature").is_none());
    }

    #[test]
    fn test_inject_multiple_rules() {
        let mut injector = Injector::new();
        injector.add_rule(
            InjectionRule::new("r1", "claude-*", json!({"temperature": 0.7})).with_priority(10),
        );
        injector
            .add_rule(InjectionRule::new("r2", "*", json!({"max_tokens": 1000})).with_priority(20));

        let mut payload = json!({
            "model": "claude-sonnet-4-5",
            "messages": []
        });

        let result = injector.inject("claude-sonnet-4-5", &mut payload);

        assert_eq!(result.applied_rules.len(), 2);
        assert_eq!(payload["temperature"], 0.7);
        assert_eq!(payload["max_tokens"], 1000);
    }

    #[test]
    fn test_inject_priority_order() {
        let mut injector = Injector::new();
        // 低优先级规则先添加
        injector
            .add_rule(InjectionRule::new("r2", "*", json!({"temperature": 0.5})).with_priority(20));
        // 高优先级规则后添加
        injector.add_rule(
            InjectionRule::new("r1", "claude-*", json!({"temperature": 0.7})).with_priority(10),
        );

        let mut payload = json!({
            "model": "claude-sonnet-4-5",
            "messages": []
        });

        let result = injector.inject("claude-sonnet-4-5", &mut payload);

        // 高优先级规则应先应用，后续规则不应覆盖（merge 模式）
        assert_eq!(payload["temperature"], 0.7);
        assert!(result.applied_rules.contains(&"r1".to_string()));
    }

    #[test]
    fn test_remove_rule() {
        let mut injector = Injector::new();
        injector.add_rule(InjectionRule::new(
            "r1",
            "claude-*",
            json!({"temperature": 0.7}),
        ));

        let removed = injector.remove_rule("r1");
        assert!(removed.is_some());
        assert!(injector.rules().is_empty());

        let removed = injector.remove_rule("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_matching_rules() {
        let mut injector = Injector::new();
        injector.add_rule(InjectionRule::new("r1", "claude-*", json!({})));
        injector.add_rule(InjectionRule::new("r2", "gemini-*", json!({})));
        injector.add_rule(InjectionRule::new("r3", "*", json!({})));

        let matches = injector.matching_rules("claude-sonnet-4-5");
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|r| r.id == "r1"));
        assert!(matches.iter().any(|r| r.id == "r3"));
    }
}
