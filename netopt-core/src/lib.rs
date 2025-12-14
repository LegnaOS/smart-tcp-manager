//! NetOpt Core - 跨平台TCP连接优化核心库
//!
//! 提供：
//! - TCP系统参数管理（MaxUserPort, TcpTimedWaitDelay等）
//! - 进程级TCP连接监控
//! - 智能连接优化策略
//! - 无用连接自动清理

pub mod tcp_config;
pub mod monitor;
pub mod optimizer;
pub mod policy;
pub mod platform;

pub use tcp_config::*;
pub use monitor::*;
pub use optimizer::*;
pub use policy::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetOptError {
    #[error("需要管理员/root权限")]
    PermissionDenied,
    
    #[error("平台不支持此操作: {0}")]
    UnsupportedPlatform(String),
    
    #[error("参数无效: {0}")]
    InvalidParameter(String),
    
    #[error("系统调用失败: {0}")]
    SystemError(String),
    
    #[error("进程不存在: PID {0}")]
    ProcessNotFound(u32),
    
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NetOptError>;

/// TCP连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TcpState {
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
    Closed,
    Unknown,
}

impl std::fmt::Display for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpState::Listen => write!(f, "LISTEN"),
            TcpState::SynSent => write!(f, "SYN_SENT"),
            TcpState::SynReceived => write!(f, "SYN_RCVD"),
            TcpState::Established => write!(f, "ESTABLISHED"),
            TcpState::FinWait1 => write!(f, "FIN_WAIT_1"),
            TcpState::FinWait2 => write!(f, "FIN_WAIT_2"),
            TcpState::CloseWait => write!(f, "CLOSE_WAIT"),
            TcpState::Closing => write!(f, "CLOSING"),
            TcpState::LastAck => write!(f, "LAST_ACK"),
            TcpState::TimeWait => write!(f, "TIME_WAIT"),
            TcpState::Closed => write!(f, "CLOSED"),
            TcpState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// 单个TCP连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcpConnection {
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: TcpState,
    pub pid: u32,
    pub process_name: String,
}

/// 进程的TCP连接统计
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProcessTcpStats {
    pub pid: u32,
    pub process_name: String,
    pub exe_path: Option<String>,
    pub total_connections: usize,
    pub established: usize,
    pub time_wait: usize,
    pub close_wait: usize,
    pub listen: usize,
    pub other: usize,
    /// 健康评分 0-100，越低越需要优化
    pub health_score: u8,
}

/// 系统整体TCP统计
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemTcpStats {
    pub total_connections: usize,
    pub by_state: std::collections::HashMap<TcpState, usize>,
    pub by_process: Vec<ProcessTcpStats>,
    pub available_ports: usize,
    pub port_usage_percent: f32,
}

