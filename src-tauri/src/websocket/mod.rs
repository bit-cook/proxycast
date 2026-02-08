//! WebSocket 支持模块（重导出层）
//!
//! 实际实现位于 `proxycast-websocket` crate。

// 重新导出 proxycast-websocket 的所有公共模块
pub use proxycast_websocket::handler;
pub use proxycast_websocket::lifecycle;
pub use proxycast_websocket::processor;
pub use proxycast_websocket::stream;

// 重新导出常用类型
pub use proxycast_websocket::{
    MessageProcessor, WsApiRequest, WsApiResponse, WsConfig, WsConnection, WsConnectionManager,
    WsEndpoint, WsError, WsMessage, WsStats, WsStatsSnapshot, WsStreamChunk, WsStreamEnd,
};

// 保持 types 子模块兼容
pub use proxycast_core::websocket::types;
pub use proxycast_core::websocket::{KiroTokenInfo, WsKiroEvent};
