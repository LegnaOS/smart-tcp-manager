//! TCP系统配置参数管理

use crate::{Result, NetOptError};
use serde::{Deserialize, Serialize};

/// TCP系统参数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpSystemConfig {
    /// 最大用户端口号 (Windows: MaxUserPort, macOS: net.inet.ip.portrange.last)
    pub max_user_port: Option<u32>,
    
    /// TIME_WAIT状态等待时间（秒）
    /// Windows: TcpTimedWaitDelay, macOS: net.inet.tcp.msl * 2
    pub time_wait_delay: Option<u32>,
    
    /// 动态端口起始 (Windows: 动态端口起始, macOS: net.inet.ip.portrange.first)
    pub dynamic_port_start: Option<u32>,
    
    /// 最大SYN重传次数
    pub max_syn_retransmissions: Option<u32>,
    
    /// TCP KeepAlive 时间（秒）
    pub keep_alive_time: Option<u32>,
    
    /// TCP KeepAlive 探测间隔（秒）
    pub keep_alive_interval: Option<u32>,
}

impl Default for TcpSystemConfig {
    fn default() -> Self {
        Self {
            max_user_port: None,
            time_wait_delay: None,
            dynamic_port_start: None,
            max_syn_retransmissions: None,
            keep_alive_time: None,
            keep_alive_interval: None,
        }
    }
}

impl TcpSystemConfig {
    /// 推荐的高性能配置
    pub fn high_performance() -> Self {
        Self {
            max_user_port: Some(65534),
            time_wait_delay: Some(30),
            dynamic_port_start: Some(10000),
            max_syn_retransmissions: Some(2),
            keep_alive_time: Some(60),
            keep_alive_interval: Some(10),
        }
    }
    
    /// 保守配置（安全但性能一般）
    pub fn conservative() -> Self {
        Self {
            max_user_port: Some(49152),
            time_wait_delay: Some(60),
            dynamic_port_start: Some(32768),
            max_syn_retransmissions: Some(3),
            keep_alive_time: Some(7200),
            keep_alive_interval: Some(75),
        }
    }
    
    /// 验证配置合法性
    pub fn validate(&self) -> Result<()> {
        if let Some(port) = self.max_user_port {
            if port < 1024 || port > 65535 {
                return Err(NetOptError::InvalidParameter(
                    format!("max_user_port 必须在 1024-65535 之间，当前: {}", port)
                ));
            }
        }
        
        if let Some(delay) = self.time_wait_delay {
            if delay < 30 || delay > 300 {
                return Err(NetOptError::InvalidParameter(
                    format!("time_wait_delay 推荐在 30-300 秒之间，当前: {}", delay)
                ));
            }
        }
        
        if let Some(start) = self.dynamic_port_start {
            if let Some(max) = self.max_user_port {
                if start >= max {
                    return Err(NetOptError::InvalidParameter(
                        format!("dynamic_port_start ({}) 必须小于 max_user_port ({})", start, max)
                    ));
                }
            }
        }
        
        Ok(())
    }
}

/// TCP配置管理器 trait
pub trait TcpConfigManager: Send + Sync {
    /// 获取当前系统配置
    fn get_current_config(&self) -> Result<TcpSystemConfig>;
    
    /// 应用新配置（需要管理员权限）
    fn apply_config(&self, config: &TcpSystemConfig) -> Result<()>;
    
    /// 获取系统默认配置
    fn get_default_config(&self) -> TcpSystemConfig;
    
    /// 检查是否有管理员权限
    fn has_admin_privileges(&self) -> bool;
    
    /// 配置是否需要重启生效
    fn requires_reboot(&self) -> bool;
}

