//! 应用级优化策略管理
//! 
//! 允许为不同应用设置不同的TCP连接优化策略

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单个应用的优化策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPolicy {
    /// 应用名称（进程名）
    pub process_name: String,

    /// 可选：完整路径匹配
    pub exe_path: Option<String>,

    /// 是否启用自动优化
    pub auto_optimize: bool,

    /// TIME_WAIT连接阈值，超过则告警 (None表示不限制)
    pub time_wait_threshold: Option<usize>,

    /// CLOSE_WAIT连接阈值，超过则告警 (None表示不限制)
    pub close_wait_threshold: Option<usize>,

    /// 最大允许连接数 (None表示不限制)
    pub max_connections: Option<usize>,

    /// 当超过阈值时的动作
    pub threshold_action: ThresholdAction,

    /// 优先级（数字越小优先级越高）
    pub priority: u8,

    /// 备注
    pub note: String,
}

/// 超过阈值时的动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdAction {
    /// 仅告警
    Alert,
    /// 尝试优化（清理无用连接）
    Optimize,
    /// 强制重启进程（危险操作，需确认）
    RestartProcess,
    /// 忽略
    Ignore,
}

impl Default for AppPolicy {
    fn default() -> Self {
        Self {
            process_name: String::new(),
            exe_path: None,
            auto_optimize: true,
            // TIME_WAIT 是正常的 TCP 关闭状态，会在 2-4 分钟后自动释放
            // 设置较高阈值，避免过度干预正常的 TCP 行为
            time_wait_threshold: Some(300),
            // CLOSE_WAIT 表示应用程序未正确关闭连接，这才是真正需要清理的
            // 这些连接不会自动消失，应该积极处理
            close_wait_threshold: Some(30),
            max_connections: None,
            threshold_action: ThresholdAction::Alert,
            priority: 100,
            note: String::new(),
        }
    }
}

impl AppPolicy {
    /// 创建高性能应用策略（如游戏、下载器）
    /// TIME_WAIT 允许较多（正常行为），但 CLOSE_WAIT 严格控制
    pub fn high_performance(process_name: &str) -> Self {
        Self {
            process_name: process_name.to_string(),
            auto_optimize: true,
            time_wait_threshold: Some(500),  // TIME_WAIT 宽松
            close_wait_threshold: Some(50),  // CLOSE_WAIT 严格
            max_connections: None,
            threshold_action: ThresholdAction::Optimize,
            priority: 10,
            ..Default::default()
        }
    }

    /// 创建服务器应用策略（如Web服务器、数据库）
    /// 服务器通常有更多连接，但 CLOSE_WAIT 仍需控制
    pub fn server(process_name: &str) -> Self {
        Self {
            process_name: process_name.to_string(),
            auto_optimize: true,
            time_wait_threshold: Some(1000), // 服务器 TIME_WAIT 更宽松
            close_wait_threshold: Some(100), // CLOSE_WAIT 仍需控制
            max_connections: Some(10000),
            threshold_action: ThresholdAction::Alert,
            priority: 5,
            ..Default::default()
        }
    }

    /// 创建限制性策略（如可疑程序）
    pub fn restricted(process_name: &str) -> Self {
        Self {
            process_name: process_name.to_string(),
            auto_optimize: true,
            time_wait_threshold: Some(50),
            close_wait_threshold: Some(20),
            max_connections: Some(100),
            threshold_action: ThresholdAction::Optimize,
            priority: 1,
            ..Default::default()
        }
    }

    /// 创建采集/爬虫工具策略
    /// 特点：大量短连接，需要积极释放 CLOSE_WAIT，对 TIME_WAIT 宽容
    pub fn crawler(process_name: &str) -> Self {
        Self {
            process_name: process_name.to_string(),
            auto_optimize: true,
            // TIME_WAIT 是正常的，高并发采集会产生很多，不需要太激进
            time_wait_threshold: Some(500),
            // CLOSE_WAIT 必须积极处理！这是导致程序卡死的主因
            // 设置较低阈值，一旦超过就清理
            close_wait_threshold: Some(20),
            max_connections: None, // 不限制最大连接
            threshold_action: ThresholdAction::Optimize, // 自动优化
            priority: 5,
            note: "采集工具专用策略：积极清理 CLOSE_WAIT".to_string(),
            ..Default::default()
        }
    }
}

/// 策略管理器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyManager {
    /// 应用策略映射 (进程名 -> 策略)
    policies: HashMap<String, AppPolicy>,
    
    /// 全局默认策略
    pub default_policy: AppPolicy,
    
    /// 白名单进程（不做任何干预）
    pub whitelist: Vec<String>,
    
    /// 黑名单进程（总是限制）
    pub blacklist: Vec<String>,
}

impl PolicyManager {
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.default_policy = AppPolicy::default();
        manager
    }
    
    /// 添加或更新策略
    pub fn set_policy(&mut self, policy: AppPolicy) {
        self.policies.insert(policy.process_name.clone(), policy);
    }
    
    /// 获取进程的策略（如果没有特定策略则返回默认）
    pub fn get_policy(&self, process_name: &str) -> &AppPolicy {
        self.policies.get(process_name).unwrap_or(&self.default_policy)
    }
    
    /// 移除策略
    pub fn remove_policy(&mut self, process_name: &str) -> Option<AppPolicy> {
        self.policies.remove(process_name)
    }
    
    /// 获取所有策略
    pub fn all_policies(&self) -> Vec<&AppPolicy> {
        self.policies.values().collect()
    }
    
    /// 进程是否在白名单
    pub fn is_whitelisted(&self, process_name: &str) -> bool {
        self.whitelist.iter().any(|w| process_name.contains(w))
    }
    
    /// 进程是否在黑名单
    pub fn is_blacklisted(&self, process_name: &str) -> bool {
        self.blacklist.iter().any(|b| process_name.contains(b))
    }
}

