//! 国际化支持模块 (i18n)
//! 
//! 支持中文和英文界面

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Chinese,
    English,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::Chinese => "zh-CN",
            Language::English => "en-US",
        }
    }
    
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Chinese => "中文",
            Language::English => "English",
        }
    }
}

/// 翻译文本键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextKey {
    // 导航
    AppTitle,
    Dashboard,
    Processes,
    Policies,
    Settings,
    
    // 状态
    AdminRequired,
    AdminGranted,
    RefreshSuccess,
    RefreshFailed,
    Refreshing,
    AutoRefresh,
    RefreshNow,
    
    // 仪表盘
    SystemOverview,
    TotalConnections,
    AvailablePorts,
    PortUsage,
    ConnectionStateDistribution,
    ActiveConnections,
    WaitingClose,
    NeedsAttention,
    ListeningPorts,
    Top5Processes,
    ProcessName,
    Pid,
    Connections,
    HealthScore,
    
    // 进程列表
    ProcessDetails,
    AddPolicy,
    PolicyAdded,
    
    // 策略管理
    PolicyManagement,
    PolicyDescription,
    NoPolicies,
    AutoOptimize,
    Enabled,
    Disabled,
    TimeWaitThreshold,
    CloseWaitThreshold,
    MaxConnections,
    Unlimited,
    ThresholdAction,
    ActionAlert,
    ActionOptimize,
    ActionRestart,
    ActionIgnore,
    DeletePolicy,
    PolicyDeleted,
    SavePolicy,
    PolicySaved,
    PolicyTip,
    
    // 设置
    TcpSettings,
    AdminRequiredForSettings,
    QuickConfig,
    HighPerformance,
    Conservative,
    ReadCurrent,
    ConfigLoaded,
    DetailedConfig,
    MaxUserPort,
    TimeWaitDelay,
    DynamicPortStart,
    Recommended,
    ApplyConfig,
    ConfigApplied,
    ApplyFailed,
    RebootRequired,
    
    // 语言
    LanguageLabel,
    LanguageChanged,

    // 帮助
    Help,
    HelpTitle,
    HelpAbout,
    HelpAboutDesc,
    HelpFeatures,
    HelpFeaturesList,
    HelpUsage,
    HelpUsageDesc,
    HelpVersion,
}

/// 国际化管理器
#[derive(Debug, Clone)]
pub struct I18n {
    current_language: Language,
    texts: HashMap<(Language, TextKey), &'static str>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let mut texts = HashMap::new();
        Self::load_chinese(&mut texts);
        Self::load_english(&mut texts);
        
        Self {
            current_language: Language::Chinese,
            texts,
        }
    }
    
    pub fn set_language(&mut self, lang: Language) {
        self.current_language = lang;
    }
    
    pub fn current_language(&self) -> Language {
        self.current_language
    }
    
    pub fn t(&self, key: TextKey) -> &'static str {
        self.texts
            .get(&(self.current_language, key))
            .copied()
            .unwrap_or("???")
    }

    fn load_chinese(texts: &mut HashMap<(Language, TextKey), &'static str>) {
        use TextKey::*;
        let lang = Language::Chinese;

        // 导航
        texts.insert((lang, AppTitle), "Smart TCP Manager");
        texts.insert((lang, Dashboard), "📊 仪表盘");
        texts.insert((lang, Processes), "📋 进程列表");
        texts.insert((lang, Policies), "📜 策略管理");
        texts.insert((lang, Settings), "⚙️ 系统设置");

        // 状态
        texts.insert((lang, AdminRequired), "⚠️ 需要管理员权限");
        texts.insert((lang, AdminGranted), "✓ 管理员");
        texts.insert((lang, RefreshSuccess), "刷新成功");
        texts.insert((lang, RefreshFailed), "刷新失败");
        texts.insert((lang, Refreshing), "正在刷新...");
        texts.insert((lang, AutoRefresh), "自动刷新");
        texts.insert((lang, RefreshNow), "🔄 立即刷新");

        // 仪表盘
        texts.insert((lang, SystemOverview), "系统TCP连接概览");
        texts.insert((lang, TotalConnections), "总连接数");
        texts.insert((lang, AvailablePorts), "可用端口");
        texts.insert((lang, PortUsage), "端口使用率");
        texts.insert((lang, ConnectionStateDistribution), "连接状态分布");
        texts.insert((lang, ActiveConnections), "活跃连接");
        texts.insert((lang, WaitingClose), "等待关闭");
        texts.insert((lang, NeedsAttention), "需注意");
        texts.insert((lang, ListeningPorts), "监听端口");
        texts.insert((lang, Top5Processes), "连接数Top 5进程");
        texts.insert((lang, ProcessName), "进程名");
        texts.insert((lang, Pid), "PID");
        texts.insert((lang, Connections), "连接数");
        texts.insert((lang, HealthScore), "健康度");

        // 进程列表
        texts.insert((lang, ProcessDetails), "进程TCP连接详情");
        texts.insert((lang, AddPolicy), "添加策略");
        texts.insert((lang, PolicyAdded), "已添加策略");

        // 策略管理
        texts.insert((lang, PolicyManagement), "应用策略管理");
        texts.insert((lang, PolicyDescription), "为不同应用配置不同的TCP连接优化策略");
        texts.insert((lang, NoPolicies), "暂无策略，请在进程列表中添加");
        texts.insert((lang, AutoOptimize), "自动优化");
        texts.insert((lang, Enabled), "✓ 开启");
        texts.insert((lang, Disabled), "✗ 关闭");
        texts.insert((lang, TimeWaitThreshold), "TIME_WAIT阈值");
        texts.insert((lang, CloseWaitThreshold), "CLOSE_WAIT阈值");
        texts.insert((lang, MaxConnections), "最大连接数");
        texts.insert((lang, Unlimited), "不限制");
        texts.insert((lang, ThresholdAction), "超阈值动作");
        texts.insert((lang, ActionAlert), "告警");
        texts.insert((lang, ActionOptimize), "自动优化");
        texts.insert((lang, ActionRestart), "重启进程");
        texts.insert((lang, ActionIgnore), "忽略");
        texts.insert((lang, DeletePolicy), "🗑 删除");
        texts.insert((lang, PolicyDeleted), "已删除策略");
        texts.insert((lang, SavePolicy), "💾 保存修改");
        texts.insert((lang, PolicySaved), "策略已保存");
        texts.insert((lang, PolicyTip), "💡 提示：在进程列表中点击\"添加策略\"为特定进程创建优化规则。每个进程只能有一个策略。");

        // 设置
        texts.insert((lang, TcpSettings), "TCP系统参数设置");
        texts.insert((lang, AdminRequiredForSettings), "⚠️ 需要管理员权限才能修改系统设置");
        texts.insert((lang, QuickConfig), "快速配置");
        texts.insert((lang, HighPerformance), "🚀 高性能配置");
        texts.insert((lang, Conservative), "🛡 保守配置");
        texts.insert((lang, ReadCurrent), "🔄 读取当前");
        texts.insert((lang, ConfigLoaded), "配置已加载");
        texts.insert((lang, DetailedConfig), "详细配置");
        texts.insert((lang, MaxUserPort), "最大用户端口 (MaxUserPort)");
        texts.insert((lang, TimeWaitDelay), "TIME_WAIT等待时间 (秒)");
        texts.insert((lang, DynamicPortStart), "动态端口起始");
        texts.insert((lang, Recommended), "推荐");
        texts.insert((lang, ApplyConfig), "✅ 应用配置");
        texts.insert((lang, ConfigApplied), "配置已应用！可能需要重启系统生效。");
        texts.insert((lang, ApplyFailed), "应用失败");
        texts.insert((lang, RebootRequired), "⚠️ 修改后需要重启系统生效");

        // 语言
        texts.insert((lang, LanguageLabel), "🌐 语言");
        texts.insert((lang, LanguageChanged), "语言已切换");

        // 帮助
        texts.insert((lang, Help), "❓ 帮助");
        texts.insert((lang, HelpTitle), "Smart TCP Manager 帮助");
        texts.insert((lang, HelpAbout), "关于");
        texts.insert((lang, HelpAboutDesc), "Smart TCP Manager 是一款跨平台的 TCP 连接管理工具，帮助您监控和优化系统的网络连接。");
        texts.insert((lang, HelpFeatures), "主要功能");
        texts.insert((lang, HelpFeaturesList), "• 📊 仪表盘：实时监控系统 TCP 连接状态\n• 📋 进程列表：查看每个进程的连接详情\n• 📜 策略管理：为不同应用配置优化策略\n• ⚙️ 系统设置：调整 TCP 系统参数");
        texts.insert((lang, HelpUsage), "使用提示");
        texts.insert((lang, HelpUsageDesc), "• 部分功能需要管理员权限\n• 建议定期检查 TIME_WAIT 和 CLOSE_WAIT 状态\n• 高性能配置适合服务器环境\n• 保守配置适合普通桌面使用");
        texts.insert((lang, HelpVersion), "版本");
    }

    fn load_english(texts: &mut HashMap<(Language, TextKey), &'static str>) {
        use TextKey::*;
        let lang = Language::English;

        // Navigation
        texts.insert((lang, AppTitle), "Smart TCP Manager");
        texts.insert((lang, Dashboard), "📊 Dashboard");
        texts.insert((lang, Processes), "📋 Processes");
        texts.insert((lang, Policies), "📜 Policies");
        texts.insert((lang, Settings), "⚙️ Settings");

        // Status
        texts.insert((lang, AdminRequired), "⚠️ Admin Required");
        texts.insert((lang, AdminGranted), "✓ Admin");
        texts.insert((lang, RefreshSuccess), "Refresh Success");
        texts.insert((lang, RefreshFailed), "Refresh Failed");
        texts.insert((lang, Refreshing), "Refreshing...");
        texts.insert((lang, AutoRefresh), "Auto Refresh");
        texts.insert((lang, RefreshNow), "🔄 Refresh");

        // Dashboard
        texts.insert((lang, SystemOverview), "System TCP Overview");
        texts.insert((lang, TotalConnections), "Total Connections");
        texts.insert((lang, AvailablePorts), "Available Ports");
        texts.insert((lang, PortUsage), "Port Usage");
        texts.insert((lang, ConnectionStateDistribution), "Connection State Distribution");
        texts.insert((lang, ActiveConnections), "Active");
        texts.insert((lang, WaitingClose), "Waiting Close");
        texts.insert((lang, NeedsAttention), "Needs Attention");
        texts.insert((lang, ListeningPorts), "Listening");
        texts.insert((lang, Top5Processes), "Top 5 Processes by Connections");
        texts.insert((lang, ProcessName), "Process");
        texts.insert((lang, Pid), "PID");
        texts.insert((lang, Connections), "Connections");
        texts.insert((lang, HealthScore), "Health");

        // Process List
        texts.insert((lang, ProcessDetails), "Process TCP Details");
        texts.insert((lang, AddPolicy), "Add Policy");
        texts.insert((lang, PolicyAdded), "Policy Added");

        // Policy Management
        texts.insert((lang, PolicyManagement), "Policy Management");
        texts.insert((lang, PolicyDescription), "Configure different TCP optimization policies for different applications");
        texts.insert((lang, NoPolicies), "No policies. Add from process list.");
        texts.insert((lang, AutoOptimize), "Auto Optimize");
        texts.insert((lang, Enabled), "✓ Enabled");
        texts.insert((lang, Disabled), "✗ Disabled");
        texts.insert((lang, TimeWaitThreshold), "TIME_WAIT Threshold");
        texts.insert((lang, CloseWaitThreshold), "CLOSE_WAIT Threshold");
        texts.insert((lang, MaxConnections), "Max Connections");
        texts.insert((lang, Unlimited), "Unlimited");
        texts.insert((lang, ThresholdAction), "Threshold Action");
        texts.insert((lang, ActionAlert), "Alert");
        texts.insert((lang, ActionOptimize), "Auto Optimize");
        texts.insert((lang, ActionRestart), "Restart Process");
        texts.insert((lang, ActionIgnore), "Ignore");
        texts.insert((lang, DeletePolicy), "🗑 Delete");
        texts.insert((lang, PolicyDeleted), "Policy Deleted");
        texts.insert((lang, SavePolicy), "💾 Save");
        texts.insert((lang, PolicySaved), "Policy Saved");
        texts.insert((lang, PolicyTip), "💡 Tip: Click \"Add Policy\" in the process list to create optimization rules. Each process can only have one policy.");

        // Settings
        texts.insert((lang, TcpSettings), "TCP System Settings");
        texts.insert((lang, AdminRequiredForSettings), "⚠️ Admin privileges required to modify system settings");
        texts.insert((lang, QuickConfig), "Quick Config");
        texts.insert((lang, HighPerformance), "🚀 High Performance");
        texts.insert((lang, Conservative), "🛡 Conservative");
        texts.insert((lang, ReadCurrent), "🔄 Read Current");
        texts.insert((lang, ConfigLoaded), "Config Loaded");
        texts.insert((lang, DetailedConfig), "Detailed Config");
        texts.insert((lang, MaxUserPort), "Max User Port");
        texts.insert((lang, TimeWaitDelay), "TIME_WAIT Delay (seconds)");
        texts.insert((lang, DynamicPortStart), "Dynamic Port Start");
        texts.insert((lang, Recommended), "Recommended");
        texts.insert((lang, ApplyConfig), "✅ Apply Config");
        texts.insert((lang, ConfigApplied), "Config applied! System reboot may be required.");
        texts.insert((lang, ApplyFailed), "Apply Failed");
        texts.insert((lang, RebootRequired), "⚠️ System reboot required after changes");

        // Language
        texts.insert((lang, LanguageLabel), "🌐 Language");
        texts.insert((lang, LanguageChanged), "Language Changed");

        // Help
        texts.insert((lang, Help), "❓ Help");
        texts.insert((lang, HelpTitle), "Smart TCP Manager Help");
        texts.insert((lang, HelpAbout), "About");
        texts.insert((lang, HelpAboutDesc), "Smart TCP Manager is a cross-platform TCP connection management tool that helps you monitor and optimize your system's network connections.");
        texts.insert((lang, HelpFeatures), "Main Features");
        texts.insert((lang, HelpFeaturesList), "• 📊 Dashboard: Real-time TCP connection monitoring\n• 📋 Processes: View connection details per process\n• 📜 Policies: Configure optimization policies\n• ⚙️ Settings: Adjust TCP system parameters");
        texts.insert((lang, HelpUsage), "Tips");
        texts.insert((lang, HelpUsageDesc), "• Some features require admin privileges\n• Check TIME_WAIT and CLOSE_WAIT states regularly\n• High Performance config is suitable for servers\n• Conservative config is suitable for desktops");
        texts.insert((lang, HelpVersion), "Version");
    }
}

