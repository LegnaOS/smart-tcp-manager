//! macOS平台TCP管理实现
//!
//! 使用sysctl系统调用

use crate::{
    Result, NetOptError, TcpConnection, TcpState, ProcessTcpStats, SystemTcpStats,
    TcpSystemConfig,
};
use crate::tcp_config::TcpConfigManager;
use crate::monitor::{TcpMonitor, calculate_health_score};
use crate::optimizer::ConnectionOptimizer;
use crate::policy::AppPolicy;
use std::collections::HashMap;
use std::process::Command;

/// 检查是否有root权限
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// macOS TCP配置管理器
pub struct MacOsTcpConfigManager;

impl MacOsTcpConfigManager {
    pub fn new() -> Self {
        Self
    }
    
    fn sysctl_get(&self, name: &str) -> Option<u32> {
        Command::new("sysctl")
            .arg("-n")
            .arg(name)
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse()
                    .ok()
            })
    }
    
    fn sysctl_set(&self, name: &str, value: u32) -> Result<()> {
        let output = Command::new("sysctl")
            .arg("-w")
            .arg(format!("{}={}", name, value))
            .output()
            .map_err(|e| NetOptError::SystemError(e.to_string()))?;
        
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            if err.contains("Permission denied") || err.contains("Operation not permitted") {
                return Err(NetOptError::PermissionDenied);
            }
            return Err(NetOptError::SystemError(err.to_string()));
        }
        
        Ok(())
    }
}

impl TcpConfigManager for MacOsTcpConfigManager {
    fn get_current_config(&self) -> Result<TcpSystemConfig> {
        Ok(TcpSystemConfig {
            max_user_port: self.sysctl_get("net.inet.ip.portrange.last"),
            time_wait_delay: self.sysctl_get("net.inet.tcp.msl").map(|v| v * 2), // MSL * 2 = TIME_WAIT
            dynamic_port_start: self.sysctl_get("net.inet.ip.portrange.first"),
            max_syn_retransmissions: self.sysctl_get("net.inet.tcp.keepinit")
                .map(|v| v / 1000), // 转换为秒
            keep_alive_time: self.sysctl_get("net.inet.tcp.keepidle"),
            keep_alive_interval: self.sysctl_get("net.inet.tcp.keepintvl"),
        })
    }
    
    fn apply_config(&self, config: &TcpSystemConfig) -> Result<()> {
        config.validate()?;
        
        if let Some(v) = config.max_user_port {
            self.sysctl_set("net.inet.ip.portrange.last", v)?;
        }
        if let Some(v) = config.time_wait_delay {
            // macOS 使用 MSL (Maximum Segment Lifetime)，TIME_WAIT = 2 * MSL
            self.sysctl_set("net.inet.tcp.msl", v / 2)?;
        }
        if let Some(v) = config.dynamic_port_start {
            self.sysctl_set("net.inet.ip.portrange.first", v)?;
        }
        if let Some(v) = config.keep_alive_time {
            self.sysctl_set("net.inet.tcp.keepidle", v)?;
        }
        if let Some(v) = config.keep_alive_interval {
            self.sysctl_set("net.inet.tcp.keepintvl", v)?;
        }
        
        Ok(())
    }
    
    fn get_default_config(&self) -> TcpSystemConfig {
        TcpSystemConfig {
            max_user_port: Some(65535),
            time_wait_delay: Some(30), // macOS 默认 MSL=15s，TIME_WAIT=30s
            dynamic_port_start: Some(49152),
            max_syn_retransmissions: Some(3),
            keep_alive_time: Some(7200),
            keep_alive_interval: Some(75),
        }
    }
    
    fn has_admin_privileges(&self) -> bool {
        is_root()
    }
    
    fn requires_reboot(&self) -> bool {
        false // macOS sysctl 修改立即生效
    }
}

/// macOS TCP监控器
pub struct MacOsTcpMonitor;

impl MacOsTcpMonitor {
    pub fn new() -> Self {
        Self
    }
    
    /// 使用 netstat 获取连接信息
    fn parse_netstat(&self) -> Result<Vec<TcpConnection>> {
        let output = Command::new("netstat")
            .args(["-anv", "-p", "tcp"])
            .output()
            .map_err(|e| NetOptError::SystemError(e.to_string()))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut connections = Vec::new();
        
        for line in stdout.lines().skip(2) {
            if let Some(conn) = self.parse_netstat_line(line) {
                connections.push(conn);
            }
        }
        
        Ok(connections)
    }
    
    fn parse_netstat_line(&self, line: &str) -> Option<TcpConnection> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            return None;
        }
        
        // 解析本地地址和端口
        let (local_addr, local_port) = Self::parse_addr_port(parts[3])?;
        let (remote_addr, remote_port) = Self::parse_addr_port(parts[4])?;
        let state = Self::parse_state(parts[5]);
        let pid = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);
        
        Some(TcpConnection {
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            pid,
            process_name: String::new(), // 需要额外查询
        })
    }
    
    fn parse_addr_port(s: &str) -> Option<(String, u16)> {
        let idx = s.rfind('.')?;
        let addr = s[..idx].to_string();
        let port = s[idx + 1..].parse().ok()?;
        Some((addr, port))
    }
    
    fn parse_state(s: &str) -> TcpState {
        match s {
            "LISTEN" => TcpState::Listen,
            "ESTABLISHED" => TcpState::Established,
            "TIME_WAIT" => TcpState::TimeWait,
            "CLOSE_WAIT" => TcpState::CloseWait,
            "FIN_WAIT_1" => TcpState::FinWait1,
            "FIN_WAIT_2" => TcpState::FinWait2,
            "SYN_SENT" => TcpState::SynSent,
            "SYN_RECEIVED" => TcpState::SynReceived,
            "CLOSING" => TcpState::Closing,
            "LAST_ACK" => TcpState::LastAck,
            "CLOSED" => TcpState::Closed,
            _ => TcpState::Unknown,
        }
    }

    fn get_process_name(&self, pid: u32) -> String {
        Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }
}

impl TcpMonitor for MacOsTcpMonitor {
    fn get_all_connections(&self) -> Result<Vec<TcpConnection>> {
        let mut connections = self.parse_netstat()?;
        for conn in &mut connections {
            if conn.pid > 0 {
                conn.process_name = self.get_process_name(conn.pid);
            }
        }
        Ok(connections)
    }

    fn get_process_connections(&self, pid: u32) -> Result<Vec<TcpConnection>> {
        let all = self.get_all_connections()?;
        Ok(all.into_iter().filter(|c| c.pid == pid).collect())
    }

    fn get_process_stats(&self, pid: u32) -> Result<ProcessTcpStats> {
        let connections = self.get_process_connections(pid)?;
        let process_name = if connections.is_empty() {
            self.get_process_name(pid)
        } else {
            connections[0].process_name.clone()
        };

        let mut stats = ProcessTcpStats {
            pid,
            process_name,
            total_connections: connections.len(),
            ..Default::default()
        };

        for conn in &connections {
            match conn.state {
                TcpState::Established => stats.established += 1,
                TcpState::TimeWait => stats.time_wait += 1,
                TcpState::CloseWait => stats.close_wait += 1,
                TcpState::Listen => stats.listen += 1,
                _ => stats.other += 1,
            }
        }

        stats.health_score = calculate_health_score(&stats);
        Ok(stats)
    }

    fn get_system_stats(&self) -> Result<SystemTcpStats> {
        let connections = self.get_all_connections()?;
        let mut by_state = HashMap::new();
        let mut by_process_map: HashMap<u32, ProcessTcpStats> = HashMap::new();

        for conn in &connections {
            *by_state.entry(conn.state).or_insert(0) += 1;

            let entry = by_process_map.entry(conn.pid).or_insert_with(|| ProcessTcpStats {
                pid: conn.pid,
                process_name: conn.process_name.clone(),
                ..Default::default()
            });

            entry.total_connections += 1;
            match conn.state {
                TcpState::Established => entry.established += 1,
                TcpState::TimeWait => entry.time_wait += 1,
                TcpState::CloseWait => entry.close_wait += 1,
                TcpState::Listen => entry.listen += 1,
                _ => entry.other += 1,
            }
        }

        let mut by_process: Vec<_> = by_process_map.into_values().collect();
        for p in &mut by_process {
            p.health_score = calculate_health_score(p);
        }
        by_process.sort_by(|a, b| b.total_connections.cmp(&a.total_connections));

        // 获取端口范围计算可用端口
        let config_mgr = MacOsTcpConfigManager::new();
        let config = config_mgr.get_current_config().unwrap_or_default();
        let port_start = config.dynamic_port_start.unwrap_or(49152) as usize;
        let port_end = config.max_user_port.unwrap_or(65535) as usize;
        let total_ports = port_end - port_start + 1;
        let used_ports = connections.len();

        Ok(SystemTcpStats {
            total_connections: connections.len(),
            by_state,
            by_process,
            available_ports: total_ports.saturating_sub(used_ports),
            port_usage_percent: (used_ports as f32 / total_ports as f32) * 100.0,
        })
    }

    fn get_top_processes(&self, limit: usize) -> Result<Vec<ProcessTcpStats>> {
        let stats = self.get_system_stats()?;
        Ok(stats.by_process.into_iter().take(limit).collect())
    }

    fn get_problematic_processes(&self, threshold: usize) -> Result<Vec<ProcessTcpStats>> {
        let stats = self.get_system_stats()?;
        Ok(stats.by_process.into_iter()
            .filter(|p| p.time_wait > threshold || p.close_wait > threshold / 4)
            .collect())
    }
}

/// macOS 连接优化器
pub struct MacOsConnectionOptimizer;

impl MacOsConnectionOptimizer {
    pub fn new() -> Self {
        Self
    }
}

impl ConnectionOptimizer for MacOsConnectionOptimizer {
    fn close_connection(&self, _conn: &TcpConnection) -> Result<()> {
        // macOS 不支持直接关闭其他进程的连接
        Err(NetOptError::UnsupportedPlatform(
            "macOS不支持直接关闭TCP连接，需要通过信号通知进程".into()
        ))
    }

    fn close_connections_by_state(&self, _pid: u32, _state: TcpState) -> Result<usize> {
        Err(NetOptError::UnsupportedPlatform(
            "macOS不支持直接关闭TCP连接".into()
        ))
    }

    fn optimize_process(&self, pid: u32, policy: &AppPolicy) -> Result<crate::optimizer::OptimizationAction> {
        // 对于 macOS，只能发送信号建议进程自己清理
        Ok(crate::optimizer::OptimizationAction {
            pid,
            process_name: policy.process_name.clone(),
            action_type: crate::optimizer::ActionType::GracefulShutdown,
            reason: "macOS仅支持发送信号建议进程清理连接".into(),
            connections_affected: 0,
            success: true,
            error_message: None,
        })
    }

    fn supports_connection_control(&self) -> bool {
        false // macOS 不支持直接控制其他进程的连接
    }
}

