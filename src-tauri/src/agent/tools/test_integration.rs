//! aster-rust 工具系统集成测试
//!
//! 验证 ProxyCast 与 aster-rust 工具系统的集成是否正常工作

use super::*;
use std::path::PathBuf;

#[tokio::test]
async fn test_create_default_registry() {
    let base_dir = PathBuf::from("/tmp");
    let registry = create_default_registry(&base_dir);

    // 验证注册表已创建
    let definitions = registry.get_definitions();
    assert!(
        !definitions.is_empty(),
        "工具注册表应该包含 aster-rust 工具"
    );

    tracing::info!("✅ 工具注册表创建成功，包含 {} 个工具", definitions.len());
}

#[tokio::test]
async fn test_create_minimal_registry() {
    let base_dir = PathBuf::from("/tmp");
    let registry = create_minimal_registry(&base_dir);

    // 验证注册表已创建
    let definitions = registry.get_definitions();
    assert!(!definitions.is_empty(), "最小工具注册表应该包含基础工具");

    tracing::info!(
        "✅ 最小工具注册表创建成功，包含 {} 个工具",
        definitions.len()
    );
}

#[tokio::test]
async fn test_tool_registry_basic_operations() {
    let base_dir = PathBuf::from("/tmp");
    let registry = create_default_registry(&base_dir);

    // 测试基本操作
    let definitions = registry.get_definitions();
    let tool_count = definitions.len();
    assert!(tool_count > 0, "应该有工具被注册");

    // 测试是否为空
    assert!(!definitions.is_empty(), "注册表不应该为空");

    tracing::info!("✅ 工具注册表基本操作测试通过，共 {} 个工具", tool_count);

    // 验证工具定义的基本结构
    for def in definitions.iter().take(3) {
        assert!(!def.name.is_empty(), "工具名称不应该为空");
        assert!(!def.description.is_empty(), "工具描述不应该为空");
        tracing::debug!("工具: {} - {}", def.name, def.description);
    }
}
