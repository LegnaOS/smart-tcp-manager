//! NetOpt Service - 后台TCP优化服务
//!
//! 提供:
//! - 定时监控TCP连接状态
//! - 自动检测问题进程
//! - 根据策略自动优化
//! - IPC接口供GUI调用

use netopt_core::platform::{create_monitor, create_config_manager, has_admin_privileges};
use netopt_core::policy::PolicyManager;
use netopt_core::optimizer::OptimizationEngine;
use netopt_core::monitor::detect_anomalies;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

/// 服务配置
struct ServiceConfig {
    /// 监控间隔（秒）
    monitor_interval: u64,
    /// TIME_WAIT 告警阈值
    time_wait_threshold: usize,
    /// 是否启用自动优化
    auto_optimize: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            monitor_interval: 30,
            time_wait_threshold: 200,
            auto_optimize: false, // 默认不自动优化，只告警
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("NetOpt Service 启动中...");
    
    // 检查权限
    if !has_admin_privileges() {
        warn!("未检测到管理员权限，某些功能可能受限");
    }
    
    let config = ServiceConfig::default();
    let monitor = create_monitor();
    let config_mgr = create_config_manager();
    let policy_mgr = PolicyManager::new();
    let engine = OptimizationEngine::new(policy_mgr);
    
    // 显示当前TCP配置
    match config_mgr.get_current_config() {
        Ok(tcp_config) => {
            info!("当前TCP配置:");
            if let Some(v) = tcp_config.max_user_port {
                info!("  MaxUserPort: {}", v);
            }
            if let Some(v) = tcp_config.time_wait_delay {
                info!("  TcpTimedWaitDelay: {}s", v);
            }
        }
        Err(e) => warn!("无法读取TCP配置: {}", e),
    }
    
    // 启动监控循环
    let mut ticker = interval(Duration::from_secs(config.monitor_interval));
    
    info!("开始监控，间隔: {}秒", config.monitor_interval);
    
    loop {
        ticker.tick().await;
        
        match monitor.get_system_stats() {
            Ok(stats) => {
                info!(
                    "系统状态: 总连接={}, 端口使用={:.1}%, TIME_WAIT={}, CLOSE_WAIT={}",
                    stats.total_connections,
                    stats.port_usage_percent,
                    stats.by_state.get(&netopt_core::TcpState::TimeWait).unwrap_or(&0),
                    stats.by_state.get(&netopt_core::TcpState::CloseWait).unwrap_or(&0),
                );
                
                // 检测问题进程
                for proc_stats in &stats.by_process {
                    let anomalies = detect_anomalies(proc_stats);
                    for anomaly in anomalies {
                        match anomaly.severity {
                            netopt_core::monitor::Severity::Critical => {
                                error!("[{}] {}: {}", proc_stats.process_name, anomaly.message, anomaly.suggestion);
                            }
                            netopt_core::monitor::Severity::Warning => {
                                warn!("[{}] {}", proc_stats.process_name, anomaly.message);
                            }
                            _ => {}
                        }
                    }
                    
                    // 自动优化（如果启用）
                    if config.auto_optimize {
                        let actions = engine.analyze_and_decide(proc_stats);
                        for action in actions {
                            if action.connections_affected > 0 {
                                info!("优化动作: {} - {}", action.process_name, action.reason);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("获取系统状态失败: {}", e);
            }
        }
    }
}

