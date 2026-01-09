//! 网络相关命令
//!
//! 提供获取本地网络接口信息的功能

use serde::Serialize;
use std::net::{IpAddr, UdpSocket};

/// 网络接口信息
#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo {
    /// 本地回环地址
    pub localhost: String,
    /// 内网 IP 地址（局域网）
    pub lan_ip: Option<String>,
    /// 所有可用的网络接口 IP 地址
    pub all_ips: Vec<String>,
}

/// 获取本地网络信息
///
/// 返回 localhost 和内网 IP 地址，用于客户端连接
#[tauri::command]
pub fn get_network_info() -> Result<NetworkInfo, String> {
    let lan_ip = get_local_ip();
    let all_ips = get_all_local_ips();

    Ok(NetworkInfo {
        localhost: "127.0.0.1".to_string(),
        lan_ip,
        all_ips,
    })
}

/// 获取本机内网 IP 地址
///
/// 通过创建 UDP socket 连接外部地址来获取本机的内网 IP
fn get_local_ip() -> Option<String> {
    // 创建一个 UDP socket 并连接到外部地址（不会真正发送数据）
    // 这样可以获取到本机用于出站连接的 IP 地址
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

/// 获取所有本地网络接口的 IP 地址
///
/// 返回所有非回环的 IPv4 地址，过滤掉 VPN 和虚拟网卡
fn get_all_local_ips() -> Vec<String> {
    use std::net::Ipv4Addr;

    let mut ips = Vec::new();

    // 使用 if-addrs crate 获取所有网络接口
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for iface in interfaces {
            // 只处理 IPv4 地址
            if let IpAddr::V4(ipv4) = iface.ip() {
                // 过滤掉回环地址
                if ipv4.is_loopback() {
                    continue;
                }

                // 过滤掉链路本地地址 (169.254.x.x)
                if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
                    continue;
                }

                // 过滤掉常见的 VPN 地址段
                // 198.18.0.0/15 (用于基准测试)
                if ipv4.octets()[0] == 198 && (ipv4.octets()[1] == 18 || ipv4.octets()[1] == 19) {
                    continue;
                }

                // 只保留私有网络地址
                // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                let is_private = ipv4.octets()[0] == 10
                    || (ipv4.octets()[0] == 172
                        && (ipv4.octets()[1] >= 16 && ipv4.octets()[1] <= 31))
                    || (ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168);

                if is_private {
                    ips.push(ipv4.to_string());
                }
            }
        }
    }

    ips
}
