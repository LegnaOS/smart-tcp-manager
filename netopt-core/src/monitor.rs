//! 进程级TCP连接监控模块

use crate::{Result, TcpConnection, TcpState, ProcessTcpStats, SystemTcpStats};
use std::collections::HashMap;

/// TCP连接监控器 trait
pub trait TcpMonitor: Send + Sync {
    /// 获取所有TCP连接
    fn get_all_connections(&self) -> Result<Vec<TcpConnection>>;
    
    /// 获取指定进程的连接
    fn get_process_connections(&self, pid: u32) -> Result<Vec<TcpConnection>>;
    
    /// 获取指定进程的统计信息
    fn get_process_stats(&self, pid: u32) -> Result<ProcessTcpStats>;
    
    /// 获取系统整体统计
    fn get_system_stats(&self) -> Result<SystemTcpStats>;
    
    /// 获取占用连接最多的进程列表
    fn get_top_processes(&self, limit: usize) -> Result<Vec<ProcessTcpStats>>;
    
    /// 获取问题进程（TIME_WAIT或CLOSE_WAIT过多）
    fn get_problematic_processes(&self, threshold: usize) -> Result<Vec<ProcessTcpStats>>;
}

/// 连接统计工具函数
pub fn calculate_stats(connections: &[TcpConnection]) -> HashMap<TcpState, usize> {
    let mut stats = HashMap::new();
    for conn in connections {
        *stats.entry(conn.state).or_insert(0) += 1;
    }
    stats
}

/// 按进程分组连接
pub fn group_by_process(connections: &[TcpConnection]) -> HashMap<u32, Vec<&TcpConnection>> {
    let mut groups: HashMap<u32, Vec<&TcpConnection>> = HashMap::new();
    for conn in connections {
        groups.entry(conn.pid).or_default().push(conn);
    }
    groups
}

/// 计算进程健康评分
/// 评分规则：
/// - 基础分 100
/// - TIME_WAIT > 100: -20分
/// - TIME_WAIT > 500: 额外-30分  
/// - CLOSE_WAIT > 50: -25分（更严重，可能是程序bug）
/// - 总连接数 > 1000: -10分
pub fn calculate_health_score(stats: &ProcessTcpStats) -> u8 {
    let mut score: i32 = 100;
    
    // TIME_WAIT 过多扣分
    if stats.time_wait > 100 {
        score -= 20;
    }
    if stats.time_wait > 500 {
        score -= 30;
    }
    
    // CLOSE_WAIT 过多扣分（这通常意味着程序bug）
    if stats.close_wait > 50 {
        score -= 25;
    }
    if stats.close_wait > 200 {
        score -= 25;
    }
    
    // 总连接数过多
    if stats.total_connections > 1000 {
        score -= 10;
    }
    if stats.total_connections > 5000 {
        score -= 15;
    }
    
    score.max(0).min(100) as u8
}

/// 连接异常检测结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionAnomaly {
    pub pid: u32,
    pub process_name: String,
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnomalyType {
    TooManyTimeWait,
    TooManyCloseWait,
    TooManyConnections,
    PortExhaustion,
    ConnectionLeak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// 检测连接异常
pub fn detect_anomalies(stats: &ProcessTcpStats) -> Vec<ConnectionAnomaly> {
    let mut anomalies = Vec::new();
    
    // TIME_WAIT 检测
    if stats.time_wait > 500 {
        anomalies.push(ConnectionAnomaly {
            pid: stats.pid,
            process_name: stats.process_name.clone(),
            anomaly_type: AnomalyType::TooManyTimeWait,
            severity: Severity::Critical,
            message: format!("TIME_WAIT连接数过多: {}", stats.time_wait),
            suggestion: "建议降低TcpTimedWaitDelay或检查是否频繁创建短连接".into(),
        });
    } else if stats.time_wait > 100 {
        anomalies.push(ConnectionAnomaly {
            pid: stats.pid,
            process_name: stats.process_name.clone(),
            anomaly_type: AnomalyType::TooManyTimeWait,
            severity: Severity::Warning,
            message: format!("TIME_WAIT连接数较高: {}", stats.time_wait),
            suggestion: "关注连接复用，考虑使用连接池".into(),
        });
    }
    
    // CLOSE_WAIT 检测（更严重）
    if stats.close_wait > 50 {
        anomalies.push(ConnectionAnomaly {
            pid: stats.pid,
            process_name: stats.process_name.clone(),
            anomaly_type: AnomalyType::TooManyCloseWait,
            severity: Severity::Critical,
            message: format!("CLOSE_WAIT连接数过多: {}，可能存在连接泄漏", stats.close_wait),
            suggestion: "检查程序是否正确关闭socket，可能需要重启应用".into(),
        });
    }
    
    anomalies
}

