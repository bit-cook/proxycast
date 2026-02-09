//! WebSocket RPC 处理器
//!
//! 实现 RPC 请求处理逻辑

pub mod rpc_handler;

pub use rpc_handler::{parse_rpc_request, serialize_rpc_response, RpcHandler, RpcHandlerState};
