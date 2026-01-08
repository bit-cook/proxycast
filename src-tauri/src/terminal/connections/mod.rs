//! 连接模块
//!
//! 提供不同类型的终端连接实现：本地 PTY、SSH、WSL。
//!
//! ## 模块结构
//! - `local_pty` - 本地 PTY 连接
//! - `ssh_connection` - SSH 远程连接
//! - `ssh_shell_proc` - SSH 远程 Shell 进程
//! - `wsl_connection` - WSL 连接（仅 Windows）
//! - `connection_router` - 连接类型路由
//! - `connection_config` - 连接配置持久化
//!
//! ## 功能
//! - 本地 PTY 进程管理
//! - SSH 远程连接和认证
//! - SSH 远程 PTY 创建和数据转发
//! - WSL 发行版连接
//! - 连接类型自动路由
//! - 连接配置存储和管理

pub mod connection_config;
pub mod connection_router;
pub mod local_pty;
pub mod ssh_connection;
pub mod ssh_shell_proc;
pub mod wsl_connection;

pub use connection_config::{
    ConnectionConfig, ConnectionConfigManager, ConnectionConfigType, ConnectionListEntry,
    ConnectionSource, ConnectionsFile, SSHHostEntry,
};
pub use connection_router::{ConnectionInfo, ConnectionRouter, ConnectionType};
pub use local_pty::ShellProc;
pub use ssh_connection::{
    build_default_auth_methods, get_default_identity_files, is_local_conn_name,
    is_ssh_agent_available, is_ssh_conn_name, ConnKeywords, ConnStatus, ConnectionState,
    HostKeyVerification, NoOpAuthCallback, SSHAuthCallback, SSHAuthMethod, SSHConfigEntry,
    SSHConfigParser, SSHConn, SSHOpts, DEFAULT_SSH_PORT, MAX_PROXY_JUMP_DEPTH,
};
pub use ssh_shell_proc::SSHShellProc;
pub use wsl_connection::{
    is_wsl_conn_name, WSLConn, WSLDistro, WSLDistroState, WSLOpts, WSLShellProc,
    DEFAULT_WSL_DISTRO, WSL_CONN_PREFIX,
};
