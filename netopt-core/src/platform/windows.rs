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
pub struct WindowsTcpMonitor {
    /// 进程名缓存 (PID -> 进程名)
    process_cache: std::sync::Mutex<std::collections::HashMap<u32, String>>,
}

impl WindowsTcpMonitor {
    pub fn new() -> Self {
        Self {
            process_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
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

        // 收集需要查询的唯一 PID
        let unique_pids: std::collections::HashSet<u32> = connections.iter()
            .filter(|c| c.pid > 0)
            .map(|c| c.pid)
            .collect();

        // 批量获取进程名（使用缓存）
        let process_names = self.get_process_names_batch(&unique_pids);

        // 应用进程名
        for conn in &mut connections {
            if conn.pid > 0 {
                if let Some(name) = process_names.get(&conn.pid) {
                    conn.process_name = name.clone();
                }
            }
        }

        Ok(connections)
    }

    /// 批量获取进程名，使用缓存减少 tasklist 调用
    fn get_process_names_batch(&self, pids: &std::collections::HashSet<u32>) -> std::collections::HashMap<u32, String> {
        use std::process::Command;

        let mut result = std::collections::HashMap::new();
        let mut cache = self.process_cache.lock().unwrap();

        // 先从缓存获取
        let mut missing_pids: Vec<u32> = Vec::new();
        for &pid in pids {
            if let Some(name) = cache.get(&pid) {
                result.insert(pid, name.clone());
            } else {
                missing_pids.push(pid);
            }
        }

        // 如果有缺失的 PID，一次性调用 tasklist 获取所有进程
        if !missing_pids.is_empty() {
            if let Ok(output) = Command::new("tasklist").args(["/FO", "CSV", "/NH"]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // 格式: "进程名","PID",...
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim_matches('"').to_string();
                        if let Ok(pid) = parts[1].trim_matches('"').parse::<u32>() {
                            cache.insert(pid, name.clone());
                            if pids.contains(&pid) {
                                result.insert(pid, name);
                            }
                        }
                    }
                }
            }
        }

        result
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
///
/// 使用 SetTcpEntry API 来关闭 TCP 连接
pub struct WindowsConnectionOptimizer {
    monitor: WindowsTcpMonitor,
}

impl WindowsConnectionOptimizer {
    pub fn new() -> Self {
        Self {
            monitor: WindowsTcpMonitor::new(),
        }
    }

    /// 将 IP 地址转换为 Windows API 需要的格式 (网络字节序的 u32)
    #[cfg(target_os = "windows")]
    fn ip_to_u32(addr: &std::net::IpAddr) -> u32 {
        match addr {
            std::net::IpAddr::V4(v4) => {
                let octets = v4.octets();
                // 网络字节序 (大端)
                u32::from_be_bytes(octets)
            }
            std::net::IpAddr::V6(_) => 0, // IPv6 需要不同处理
        }
    }

    /// 将端口转换为网络字节序
    #[cfg(target_os = "windows")]
    fn port_to_network_order(port: u16) -> u32 {
        ((port as u32) << 8) | ((port as u32) >> 8)
    }
}

impl ConnectionOptimizer for WindowsConnectionOptimizer {
    fn close_connection(&self, conn: &TcpConnection) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use std::mem::size_of;

            // MIB_TCPROW 结构体
            #[repr(C)]
            #[allow(non_snake_case)]
            struct MIB_TCPROW {
                dwState: u32,
                dwLocalAddr: u32,
                dwLocalPort: u32,
                dwRemoteAddr: u32,
                dwRemotePort: u32,
            }

            // 状态常量 - 设置为 MIB_TCP_STATE_DELETE_TCB (12) 来关闭连接
            const MIB_TCP_STATE_DELETE_TCB: u32 = 12;

            // 构造 MIB_TCPROW 结构
            let row = MIB_TCPROW {
                dwState: MIB_TCP_STATE_DELETE_TCB,
                dwLocalAddr: Self::ip_to_u32(&conn.local_addr),
                dwLocalPort: Self::port_to_network_order(conn.local_port),
                dwRemoteAddr: Self::ip_to_u32(&conn.remote_addr),
                dwRemotePort: Self::port_to_network_order(conn.remote_port),
            };

            // 调用 SetTcpEntry
            type SetTcpEntryFn = unsafe extern "system" fn(*const MIB_TCPROW) -> u32;

            let iphlpapi = unsafe {
                windows::Win32::System::LibraryLoader::LoadLibraryW(
                    windows::core::w!("iphlpapi.dll")
                )
            };

            match iphlpapi {
                Ok(handle) => {
                    let proc_addr = unsafe {
                        windows::Win32::System::LibraryLoader::GetProcAddress(
                            handle,
                            windows::core::s!("SetTcpEntry")
                        )
                    };

                    if let Some(addr) = proc_addr {
                        let set_tcp_entry: SetTcpEntryFn = unsafe { std::mem::transmute(addr) };
                        let result = unsafe { set_tcp_entry(&row) };

                        if result == 0 {
                            Ok(())
                        } else {
                            Err(NetOptError::SystemError(
                                format!("SetTcpEntry 失败, 错误码: {}. 需要管理员权限。", result)
                            ))
                        }
                    } else {
                        Err(NetOptError::SystemError("无法获取 SetTcpEntry 函数地址".into()))
                    }
                }
                Err(e) => Err(NetOptError::SystemError(format!("加载 iphlpapi.dll 失败: {}", e)))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = conn;
            Err(NetOptError::UnsupportedPlatform("Not Windows".into()))
        }
    }

    fn close_connections_by_state(&self, pid: u32, state: TcpState) -> Result<usize> {
        // 获取进程的所有连接
        let stats = self.monitor.get_process_stats(pid)?;
        let connections = stats.connections;

        let mut closed = 0;
        for conn in connections {
            if conn.state == state {
                if self.close_connection(&conn).is_ok() {
                    closed += 1;
                }
            }
        }

        Ok(closed)
    }

    fn optimize_process(&self, pid: u32, policy: &AppPolicy) -> Result<crate::optimizer::OptimizationAction> {
        use crate::optimizer::{OptimizationAction, ActionType};

        let stats = self.monitor.get_process_stats(pid)?;
        let mut connections_affected = 0;
        let mut action_type = ActionType::None;
        let mut reason = String::new();

        // 检查 TIME_WAIT 阈值
        if let Some(threshold) = policy.time_wait_threshold {
            if stats.time_wait > threshold {
                reason = format!(
                    "TIME_WAIT({}) 超过阈值({}), 正在清理",
                    stats.time_wait, threshold
                );

                let closed = self.close_connections_by_state(pid, TcpState::TimeWait)?;
                connections_affected += closed;
                action_type = ActionType::CloseConnections;
            }
        }

        // 检查 CLOSE_WAIT 阈值
        if let Some(threshold) = policy.close_wait_threshold {
            if stats.close_wait > threshold {
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "CLOSE_WAIT({}) 超过阈值({}), 正在清理",
                    stats.close_wait, threshold
                ));

                let closed = self.close_connections_by_state(pid, TcpState::CloseWait)?;
                connections_affected += closed;
                action_type = ActionType::CloseConnections;
            }
        }

        if reason.is_empty() {
            reason = "连接状态正常，无需优化".into();
        }

        Ok(OptimizationAction {
            pid,
            process_name: policy.process_name.clone(),
            action_type,
            reason,
            connections_affected,
            success: true,
            error_message: None,
        })
    }

    fn supports_connection_control(&self) -> bool {
        true // Windows 支持通过 SetTcpEntry 控制连接
    }
}

