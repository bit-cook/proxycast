//! 路由模块属性测试
//!
//! 使用 proptest 进行属性测试

use crate::router::{ModelMapper, Router, RoutingRule};
use crate::ProviderType;
use proptest::prelude::*;

/// 生成随机的模型名称
fn arb_model_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // 常见模型名格式
        "[a-z]+-[a-z0-9-]+".prop_map(|s| s),
        // 带版本号的模型名
        "[a-z]+-[0-9]+\\.[0-9]+(-[a-z]+)?".prop_map(|s| s),
    ]
}

/// 生成随机的别名
fn arb_alias() -> impl Strategy<Value = String> {
    "[a-z]+-[a-z0-9]+".prop_map(|s| s)
}

/// 生成随机的 ProviderType
fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
    prop_oneof![
        Just(ProviderType::Kiro),
        Just(ProviderType::Gemini),
        Just(ProviderType::Qwen),
        Just(ProviderType::OpenAI),
        Just(ProviderType::Claude),
    ]
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 6: 模型别名往返**
    /// *对于任意* 别名映射 (alias -> actual)，resolve(alias) 应返回 actual
    /// **Validates: Requirements 2.1 (验收标准 2)**
    #[test]
    fn prop_model_alias_roundtrip(
        alias in arb_alias(),
        actual in arb_model_name()
    ) {
        // 确保别名和实际模型名不同
        prop_assume!(alias != actual);

        let mut mapper = ModelMapper::new();
        mapper.add_alias(&alias, &actual);

        // 验证：resolve(alias) 应返回 actual
        let resolved = mapper.resolve(&alias);
        prop_assert_eq!(
            resolved.clone(),
            actual.clone(),
            "resolve('{}') 应返回 '{}'，但实际返回 '{}'",
            alias,
            actual,
            resolved
        );
    }

    /// **Feature: enhancement-roadmap, Property 6: 模型别名往返（非别名）**
    /// *对于任意* 非别名模型名，resolve() 应返回原模型名
    /// **Validates: Requirements 2.1 (验收标准 2)**
    #[test]
    fn prop_non_alias_passthrough(
        model in arb_model_name(),
        alias in arb_alias(),
        actual in arb_model_name()
    ) {
        // 确保 model 不是已注册的别名
        prop_assume!(model != alias);

        let mut mapper = ModelMapper::new();
        mapper.add_alias(&alias, &actual);

        // 验证：非别名模型应返回原值
        let resolved = mapper.resolve(&model);
        prop_assert_eq!(
            resolved.clone(),
            model.clone(),
            "非别名模型 '{}' 应返回原值，但实际返回 '{}'",
            model,
            resolved
        );
    }

    /// **Feature: enhancement-roadmap, Property 6: 模型别名往返（多别名）**
    /// *对于任意* 多个别名映射，每个 resolve(alias) 应返回对应的 actual
    /// **Validates: Requirements 2.1 (验收标准 2)**
    #[test]
    fn prop_multiple_aliases_roundtrip(
        aliases in prop::collection::vec((arb_alias(), arb_model_name()), 1..=10)
    ) {
        // 确保别名唯一
        let mut seen_aliases = std::collections::HashSet::new();
        let unique_aliases: Vec<_> = aliases
            .into_iter()
            .filter(|(alias, actual)| {
                alias != actual && seen_aliases.insert(alias.clone())
            })
            .collect();

        prop_assume!(!unique_aliases.is_empty());

        let mut mapper = ModelMapper::new();
        for (alias, actual) in &unique_aliases {
            mapper.add_alias(alias, actual);
        }

        // 验证：每个别名都应正确解析
        for (alias, actual) in &unique_aliases {
            let resolved = mapper.resolve(alias);
            prop_assert_eq!(
                &resolved,
                actual,
                "resolve('{}') 应返回 '{}'，但实际返回 '{}'",
                alias,
                actual,
                resolved
            );
        }
    }
}

proptest! {
    /// 路由规则精确匹配测试
    #[test]
    fn prop_exact_rule_matches_only_exact(
        model in arb_model_name(),
        provider in arb_provider_type()
    ) {
        let rule = RoutingRule::new(&model, provider, 10);

        // 精确匹配应该匹配
        prop_assert!(
            rule.matches(&model),
            "精确规则 '{}' 应匹配模型 '{}'",
            rule.pattern,
            model
        );

        // 添加后缀不应匹配
        let modified = format!("{}-extra", model);
        prop_assert!(
            !rule.matches(&modified),
            "精确规则 '{}' 不应匹配 '{}'",
            rule.pattern,
            modified
        );
    }

    /// 路由规则优先级测试
    #[test]
    fn prop_exact_rules_have_higher_priority(
        model in arb_model_name(),
        provider in arb_provider_type(),
        exact_priority in 10i32..100i32,
        wildcard_priority in 1i32..10i32
    ) {
        let exact_rule = RoutingRule::new(&model, provider, exact_priority);
        let wildcard_rule = RoutingRule::new(&format!("{}*", &model[..model.len().min(3)]), provider, wildcard_priority);

        // 精确匹配规则应优先于通配符规则，即使通配符优先级数字更小
        prop_assert!(
            exact_rule < wildcard_rule,
            "精确规则应优先于通配符规则"
        );
    }
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 7: 路由规则优先级**
    /// *对于任意* 路由规则集合，精确匹配规则应优先于通配符规则
    /// **Validates: Requirements 2.2 (验收标准 3)**
    #[test]
    fn prop_routing_rule_priority_exact_over_wildcard(
        model in arb_model_name(),
        exact_provider in arb_provider_type(),
        wildcard_provider in arb_provider_type(),
        exact_priority in 50i32..100i32,
        wildcard_priority in 1i32..50i32
    ) {
        // 确保两个 Provider 不同，以便区分匹配结果
        prop_assume!(exact_provider != wildcard_provider);
        // 确保模型名至少有 3 个字符用于生成通配符前缀
        prop_assume!(model.len() >= 3);

        let mut router = Router::new(ProviderType::Qwen);

        // 添加通配符规则（优先级数字更小，但应该被精确匹配覆盖）
        let prefix = &model[..model.len().min(3)];
        let wildcard_pattern = format!("{}*", prefix);
        router.add_rule(RoutingRule::new(&wildcard_pattern, wildcard_provider, wildcard_priority));

        // 添加精确匹配规则（优先级数字更大）
        router.add_rule(RoutingRule::new(&model, exact_provider, exact_priority));

        // 路由应该选择精确匹配规则，即使通配符规则优先级数字更小
        let result = router.route(&model);

        prop_assert_eq!(
            result.provider,
            exact_provider,
            "模型 '{}' 应路由到精确匹配的 Provider {:?}，但实际路由到 {:?}。\n\
             精确规则: pattern='{}', priority={}\n\
             通配符规则: pattern='{}', priority={}",
            model,
            exact_provider,
            result.provider,
            model,
            exact_priority,
            wildcard_pattern,
            wildcard_priority
        );

        // 验证匹配的规则是精确匹配
        prop_assert!(
            result.matched_rule.as_ref().map(|r| r.is_exact()).unwrap_or(false),
            "匹配的规则应该是精确匹配规则"
        );
    }

    /// **Feature: enhancement-roadmap, Property 7: 路由规则优先级（同类型按优先级）**
    /// *对于任意* 同类型的路由规则，优先级数字更小的规则应优先
    /// **Validates: Requirements 2.2 (验收标准 3)**
    #[test]
    fn prop_routing_rule_priority_same_type(
        model in arb_model_name(),
        provider1 in arb_provider_type(),
        provider2 in arb_provider_type(),
        priority1 in 1i32..50i32,
        priority2 in 51i32..100i32
    ) {
        // 确保两个 Provider 不同
        prop_assume!(provider1 != provider2);
        // 确保模型名至少有 3 个字符
        prop_assume!(model.len() >= 3);

        let mut router = Router::new(ProviderType::Qwen);

        // 两个通配符规则，使用相同的前缀
        let prefix = &model[..model.len().min(3)];
        let pattern1 = format!("{}*", prefix);
        let pattern2 = format!("{}*-extra", prefix); // 不同的模式但可能都匹配

        // 添加优先级更高的规则（数字更小）
        router.add_rule(RoutingRule::new(&pattern1, provider1, priority1));
        // 添加优先级更低的规则（数字更大）
        router.add_rule(RoutingRule::new(&pattern1, provider2, priority2));

        // 路由应该选择优先级数字更小的规则
        let result = router.route(&model);

        prop_assert_eq!(
            result.provider,
            provider1,
            "模型 '{}' 应路由到优先级更高的 Provider {:?}，但实际路由到 {:?}",
            model,
            provider1,
            result.provider
        );
    }

    /// **Feature: enhancement-roadmap, Property 7: 路由规则优先级（无匹配使用默认）**
    /// *对于任意* 不匹配任何规则的模型，应使用默认 Provider
    /// **Validates: Requirements 2.2 (验收标准 3)**
    #[test]
    fn prop_routing_default_provider_when_no_match(
        model in arb_model_name(),
        default_provider in arb_provider_type(),
        rule_provider in arb_provider_type()
    ) {
        // 确保模型名不以 "zzz-" 开头（我们的规则使用这个前缀）
        prop_assume!(!model.starts_with("zzz-"));

        let mut router = Router::new(default_provider);

        // 添加一个不会匹配的规则
        router.add_rule(RoutingRule::new("zzz-*", rule_provider, 10));

        // 路由应该使用默认 Provider
        let result = router.route(&model);

        prop_assert_eq!(
            result.provider,
            default_provider,
            "模型 '{}' 应路由到默认 Provider {:?}，但实际路由到 {:?}",
            model,
            default_provider,
            result.provider
        );

        prop_assert!(
            result.is_default,
            "结果应标记为使用默认 Provider"
        );
    }
}

proptest! {
    /// **Feature: enhancement-roadmap, Property 8: 排除列表生效**
    /// *对于任意* 被排除的模型，路由结果不应指向排除该模型的 Provider
    /// **Validates: Requirements 2.3 (验收标准 1, 2)**
    #[test]
    fn prop_exclusion_list_prevents_routing(
        model in arb_model_name(),
        excluded_provider in arb_provider_type(),
        default_provider in arb_provider_type()
    ) {
        // 确保排除的 Provider 和默认 Provider 不同
        prop_assume!(excluded_provider != default_provider);

        let mut router = Router::new(default_provider);

        // 添加一个路由规则，将模型路由到 excluded_provider
        let prefix = &model[..model.len().min(3).max(1)];
        let pattern = format!("{}*", prefix);
        router.add_rule(RoutingRule::new(&pattern, excluded_provider, 10));

        // 添加精确排除模式
        router.add_exclusion(excluded_provider, &model);

        // 路由结果不应指向被排除的 Provider
        let result = router.route(&model);

        prop_assert_ne!(
            result.provider,
            excluded_provider,
            "模型 '{}' 被排除在 Provider {:?} 之外，但路由结果仍指向该 Provider",
            model,
            excluded_provider
        );
    }

    /// **Feature: enhancement-roadmap, Property 8: 排除列表生效（通配符模式）**
    /// *对于任意* 匹配排除通配符模式的模型，路由结果不应指向排除该模型的 Provider
    /// **Validates: Requirements 2.3 (验收标准 1, 2)**
    #[test]
    fn prop_exclusion_wildcard_pattern_prevents_routing(
        model in arb_model_name(),
        excluded_provider in arb_provider_type(),
        default_provider in arb_provider_type()
    ) {
        // 确保排除的 Provider 和默认 Provider 不同
        prop_assume!(excluded_provider != default_provider);
        // 确保模型名至少有 3 个字符
        prop_assume!(model.len() >= 3);

        let mut router = Router::new(default_provider);

        // 添加一个路由规则，将模型路由到 excluded_provider
        let prefix = &model[..3];
        let pattern = format!("{}*", prefix);
        router.add_rule(RoutingRule::new(&pattern, excluded_provider, 10));

        // 添加通配符排除模式（使用相同的前缀）
        router.add_exclusion(excluded_provider, &pattern);

        // 路由结果不应指向被排除的 Provider
        let result = router.route(&model);

        prop_assert_ne!(
            result.provider,
            excluded_provider,
            "模型 '{}' 匹配排除模式 '{}' 在 Provider {:?}，但路由结果仍指向该 Provider",
            model,
            pattern,
            excluded_provider
        );
    }

    /// **Feature: enhancement-roadmap, Property 8: 排除列表生效（非排除模型正常路由）**
    /// *对于任意* 未被排除的模型，应正常路由到匹配的 Provider
    /// **Validates: Requirements 2.3 (验收标准 1, 2)**
    #[test]
    fn prop_non_excluded_model_routes_normally(
        model in arb_model_name(),
        target_provider in arb_provider_type(),
        default_provider in arb_provider_type()
    ) {
        // 确保目标 Provider 和默认 Provider 不同
        prop_assume!(target_provider != default_provider);
        // 确保模型名不以 "zzz-" 开头（我们的排除模式使用这个前缀）
        prop_assume!(!model.starts_with("zzz-"));

        let mut router = Router::new(default_provider);

        // 添加一个路由规则
        let prefix = &model[..model.len().min(3).max(1)];
        let pattern = format!("{}*", prefix);
        router.add_rule(RoutingRule::new(&pattern, target_provider, 10));

        // 添加一个不会匹配的排除模式
        router.add_exclusion(target_provider, "zzz-*");

        // 未被排除的模型应正常路由
        let result = router.route(&model);

        prop_assert_eq!(
            result.provider,
            target_provider,
            "未被排除的模型 '{}' 应路由到 Provider {:?}，但实际路由到 {:?}",
            model,
            target_provider,
            result.provider
        );
    }

    /// **Feature: enhancement-roadmap, Property 8: 排除列表生效（is_excluded 一致性）**
    /// *对于任意* 模型和 Provider，is_excluded() 的结果应与路由行为一致
    /// **Validates: Requirements 2.3 (验收标准 1, 2)**
    #[test]
    fn prop_is_excluded_consistency(
        model in arb_model_name(),
        provider in arb_provider_type(),
        default_provider in arb_provider_type()
    ) {
        // 确保 Provider 和默认 Provider 不同
        prop_assume!(provider != default_provider);

        let mut router = Router::new(default_provider);

        // 添加路由规则
        let prefix = &model[..model.len().min(3).max(1)];
        let pattern = format!("{}*", prefix);
        router.add_rule(RoutingRule::new(&pattern, provider, 10));

        // 添加精确排除
        router.add_exclusion(provider, &model);

        // is_excluded 应返回 true
        prop_assert!(
            router.is_excluded(provider, &model),
            "is_excluded({:?}, '{}') 应返回 true",
            provider,
            model
        );

        // 路由结果不应指向被排除的 Provider
        let result = router.route(&model);
        prop_assert_ne!(
            result.provider,
            provider,
            "is_excluded 返回 true，但路由仍指向被排除的 Provider"
        );
    }
}
