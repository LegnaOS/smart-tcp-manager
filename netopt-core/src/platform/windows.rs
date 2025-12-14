//! Windows平台TCP管理实现
//!
//! 使用Windows注册表API和IP Helper API

use crate::{
    Result, NetOptError, TcpConnection, TcpState, ProcessTcpStats, SystemTcpStats,
    TcpSystemConfig,
};
use crate::tcp_config::TcpConfigManager;
use crate::monitor::{TcpMonitor, calculate_health_score};
use crate::optimizer::ConnectionOptimizer;
use crate::policy::AppPolicy;
use std::collections::HashMap;

#[cfg(target_os = "windows")]
use windows::{
    Win32::System::Registry::*,
    Win32::Foundation::*,
    core::PCWSTR,
};

const TCP_PARAMS_PATH: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters";

/// 检查是否有管理员权限
#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use std::process::Command;
    Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated() -> bool { false }

/// Windows TCP配置管理器
pub struct WindowsTcpConfigManager;

impl WindowsTcpConfigManager {
    pub fn new() -> Self {
        Self
    }
    
    #[cfg(target_os = "windows")]
    fn read_dword(&self, name: &str) -> Option<u32> {
        unsafe {
            let mut key = HKEY::default();
            let path: Vec<u16> = TCP_PARAMS_PATH.encode_utf16().chain(std::iter::once(0)).collect();
            
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(path.as_ptr()), 0, KEY_READ, &mut key).is_ok() {
                let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut data: u32 = 0;
                let mut size = std::mem::size_of::<u32>() as u32;
                
                if RegQueryValueExW(
                    key,
                    PCWSTR(name_wide.as_ptr()),
                    None,
                    None,
                    Some(&mut data as *mut u32 as *mut u8),
                    Some(&mut size),
                ).is_ok() {
                    let _ = RegCloseKey(key);
                    return Some(data);
                }
                let _ = RegCloseKey(key);
            }
            None
        }
    }
    
    #[cfg(target_os = "windows")]
    fn write_dword(&self, name: &str, value: u32) -> Result<()> {
        unsafe {
            let mut key = HKEY::default();
            let path: Vec<u16> = TCP_PARAMS_PATH.encode_utf16().chain(std::iter::once(0)).collect();
            
            let result = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(path.as_ptr()),
                0,
                KEY_WRITE,
                &mut key,
            );
            
            if result.is_err() {
                return Err(NetOptError::PermissionDenied);
            }
            
            let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let result = RegSetValueExW(
                key,
                PCWSTR(name_wide.as_ptr()),
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(
                    &value as *const u32 as *const u8,
                    std::mem::size_of::<u32>(),
                )),
            );
            
            let _ = RegCloseKey(key);
            
            if result.is_err() {
                return Err(NetOptError::SystemError("写入注册表失败".into()));
            }
            
            Ok(())
        }
    }
}

impl TcpConfigManager for WindowsTcpConfigManager {
    fn get_current_config(&self) -> Result<TcpSystemConfig> {
        #[cfg(target_os = "windows")]
        {
            Ok(TcpSystemConfig {
                max_user_port: self.read_dword("MaxUserPort"),
                time_wait_delay: self.read_dword("TcpTimedWaitDelay"),
                dynamic_port_start: None, // 需要使用 netsh 获取
                max_syn_retransmissions: self.read_dword("TcpMaxConnectRetransmissions"),
                keep_alive_time: self.read_dword("KeepAliveTime"),
                keep_alive_interval: self.read_dword("KeepAliveInterval"),
            })
        }
        #[cfg(not(target_os = "windows"))]
        Err(NetOptError::UnsupportedPlatform("Not Windows".into()))
    }
    
    fn apply_config(&self, config: &TcpSystemConfig) -> Result<()> {
        config.validate()?;
        
        #[cfg(target_os = "windows")]
        {
            if let Some(v) = config.max_user_port {
                self.write_dword("MaxUserPort", v)?;
            }
            if let Some(v) = config.time_wait_delay {
                self.write_dword("TcpTimedWaitDelay", v)?;
            }
            if let Some(v) = config.max_syn_retransmissions {
                self.write_dword("TcpMaxConnectRetransmissions", v)?;
            }
            if let Some(v) = config.keep_alive_time {
                self.write_dword("KeepAliveTime", v)?;
            }
            if let Some(v) = config.keep_alive_interval {
                self.write_dword("KeepAliveInterval", v)?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        Err(NetOptError::UnsupportedPlatform("Not Windows".into()))
    }
    
    fn get_default_config(&self) -> TcpSystemConfig {
        TcpSystemConfig {
            max_user_port: Some(5000),
            time_wait_delay: Some(240),
            dynamic_port_start: Some(1025),
            max_syn_retransmissions: Some(2),
            keep_alive_time: Some(7200000),
            keep_alive_interval: Some(1000),
        }
    }
    
    fn has_admin_privileges(&self) -> bool {
        is_elevated()
    }
    
    fn requires_reboot(&self) -> bool {
        true // Windows修改TCP参数通常需要重启
    }
}

/// Windows TCP监控器
/// 使用 IP Helper API (GetExtendedTcpTable)
pub struct WindowsTcpMonitor;

impl WindowsTcpMonitor {
    pub fn new() -> Self {
        Self
    }

    /// 使用 netstat 命令获取连接（备用方案）
    fn parse_netstat(&self) -> Result<Vec<TcpConnection>> {
        use std::process::Command;

        let output = Command::new("netstat")
            .args(["-ano", "-p", "tcp"])
            .output()
            .map_err(|e| NetOptError::SystemError(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut connections = Vec::new();

        for line in stdout.lines().skip(4) {
            if let Some(conn) = self.parse_netstat_line(line) {
                connections.push(conn);
            }
        }

        // 获取进程名
        for conn in &mut connections {
            if conn.pid > 0 {
                conn.process_name = self.get_process_name(conn.pid);
            }
        }

        Ok(connections)
    }

    fn parse_netstat_line(&self, line: &str) -> Option<TcpConnection> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return None;
        }

        let (local_addr, local_port) = Self::parse_addr_port(parts[1])?;
        let (remote_addr, remote_port) = Self::parse_addr_port(parts[2])?;
        let state = Self::parse_state(parts[3]);
        let pid = parts[4].parse().ok()?;

        Some(TcpConnection {
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            pid,
            process_name: String::new(),
        })
    }

    fn parse_addr_port(s: &str) -> Option<(String, u16)> {
        let idx = s.rfind(':')?;
        let addr = s[..idx].to_string();
        let port = s[idx + 1..].parse().ok()?;
        Some((addr, port))
    }

    fn parse_state(s: &str) -> TcpState {
        match s {
            "LISTENING" => TcpState::Listen,
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
        use std::process::Command;

        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.split(',').next().map(|n| n.trim_matches('"').to_string())
            })
            .unwrap_or_default()
    }
}

impl TcpMonitor for WindowsTcpMonitor {
    fn get_all_connections(&self) -> Result<Vec<TcpConnection>> {
        self.parse_netstat()
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

        // 获取端口范围
        let config_mgr = WindowsTcpConfigManager::new();
        let config = config_mgr.get_current_config().unwrap_or_default();
        let port_start = config.dynamic_port_start.unwrap_or(1025) as usize;
        let port_end = config.max_user_port.unwrap_or(5000) as usize;
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

/// Windows 连接优化器
pub struct WindowsConnectionOptimizer;

impl WindowsConnectionOptimizer {
    pub fn new() -> Self {
        Self
    }
}

impl ConnectionOptimizer for WindowsConnectionOptimizer {
    fn close_connection(&self, _conn: &TcpConnection) -> Result<()> {
        // 可以使用 SetTcpEntry 来关闭连接
        #[cfg(target_os = "windows")]
        {
            // TODO: 实现使用 SetTcpEntry API
            Err(NetOptError::UnsupportedPlatform(
                "连接关闭功能开发中".into()
            ))
        }
        #[cfg(not(target_os = "windows"))]
        Err(NetOptError::UnsupportedPlatform("Not Windows".into()))
    }

    fn close_connections_by_state(&self, _pid: u32, _state: TcpState) -> Result<usize> {
        // TODO: 批量关闭实现
        Ok(0)
    }

    fn optimize_process(&self, pid: u32, policy: &AppPolicy) -> Result<crate::optimizer::OptimizationAction> {
        Ok(crate::optimizer::OptimizationAction {
            pid,
            process_name: policy.process_name.clone(),
            action_type: crate::optimizer::ActionType::None,
            reason: "Windows连接优化功能开发中".into(),
            connections_affected: 0,
            success: true,
            error_message: None,
        })
    }

    fn supports_connection_control(&self) -> bool {
        true // Windows 支持通过 SetTcpEntry 控制连接
    }
}

