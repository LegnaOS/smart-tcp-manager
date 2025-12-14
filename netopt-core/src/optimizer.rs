//! 动态连接优化器
//! 
//! 提供按应用的动态TCP连接优化功能

use crate::{Result, NetOptError, ProcessTcpStats, TcpConnection, TcpState};
use crate::policy::{PolicyManager, AppPolicy, ThresholdAction};
use crate::monitor::{ConnectionAnomaly, Severity, AnomalyType};

/// 优化动作
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimizationAction {
    pub pid: u32,
    pub process_name: String,
    pub action_type: ActionType,
    pub reason: String,
    pub connections_affected: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    /// 关闭TIME_WAIT连接（需要系统支持）
    CloseTimeWait,
    /// 关闭CLOSE_WAIT连接
    CloseCloseWait,
    /// 发送RST重置连接
    ResetConnection,
    /// 通知进程优雅关闭
    GracefulShutdown,
    /// 无操作
    None,
}

/// 连接优化器 trait
pub trait ConnectionOptimizer: Send + Sync {
    /// 关闭指定连接
    fn close_connection(&self, conn: &TcpConnection) -> Result<()>;
    
    /// 批量关闭进程的特定状态连接
    fn close_connections_by_state(&self, pid: u32, state: TcpState) -> Result<usize>;
    
    /// 根据策略优化进程连接
    fn optimize_process(&self, pid: u32, policy: &AppPolicy) -> Result<OptimizationAction>;
    
    /// 检查是否支持连接级别操作（某些操作需要特权）
    fn supports_connection_control(&self) -> bool;
}

/// 优化决策引擎
pub struct OptimizationEngine {
    policy_manager: PolicyManager,
}

impl OptimizationEngine {
    pub fn new(policy_manager: PolicyManager) -> Self {
        Self { policy_manager }
    }
    
    /// 分析进程并决定优化动作
    pub fn analyze_and_decide(&self, stats: &ProcessTcpStats) -> Vec<OptimizationAction> {
        let mut actions = Vec::new();
        
        // 检查白名单
        if self.policy_manager.is_whitelisted(&stats.process_name) {
            return actions;
        }
        
        let policy = self.policy_manager.get_policy(&stats.process_name);
        
        // 检查是否需要优化
        if !policy.auto_optimize {
            return actions;
        }
        
        // TIME_WAIT 超阈值
        if let Some(threshold) = policy.time_wait_threshold {
            if stats.time_wait > threshold {
                let action = match policy.threshold_action {
                    ThresholdAction::Optimize => OptimizationAction {
                        pid: stats.pid,
                        process_name: stats.process_name.clone(),
                        action_type: ActionType::CloseTimeWait,
                        reason: format!(
                            "TIME_WAIT({})超过阈值({})",
                            stats.time_wait, threshold
                        ),
                        connections_affected: stats.time_wait - threshold,
                        success: false,
                        error_message: None,
                    },
                    ThresholdAction::Alert => OptimizationAction {
                        pid: stats.pid,
                        process_name: stats.process_name.clone(),
                        action_type: ActionType::None,
                        reason: format!(
                            "告警: TIME_WAIT({})超过阈值({})",
                            stats.time_wait, threshold
                        ),
                        connections_affected: 0,
                        success: true,
                        error_message: None,
                    },
                    _ => continue_action(stats),
                };
                actions.push(action);
            }
        }

        // CLOSE_WAIT 超阈值（更严重）
        if let Some(threshold) = policy.close_wait_threshold {
            if stats.close_wait > threshold {
                actions.push(OptimizationAction {
                    pid: stats.pid,
                    process_name: stats.process_name.clone(),
                    action_type: ActionType::CloseCloseWait,
                    reason: format!(
                        "CLOSE_WAIT({})超过阈值({})，可能存在连接泄漏",
                        stats.close_wait, threshold
                    ),
                    connections_affected: stats.close_wait,
                    success: false,
                    error_message: None,
                });
            }
        }
        
        actions
    }
    
    /// 获取策略管理器引用
    pub fn policy_manager(&self) -> &PolicyManager {
        &self.policy_manager
    }
    
    /// 获取可变策略管理器引用
    pub fn policy_manager_mut(&mut self) -> &mut PolicyManager {
        &mut self.policy_manager
    }
}

fn continue_action(stats: &ProcessTcpStats) -> OptimizationAction {
    OptimizationAction {
        pid: stats.pid,
        process_name: stats.process_name.clone(),
        action_type: ActionType::None,
        reason: "策略设置为忽略".into(),
        connections_affected: 0,
        success: true,
        error_message: None,
    }
}

