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

    // 全局默认设置
    GlobalDefaultSettings,
    GlobalDefaultDesc,
    ResetToDefault,
    DefaultsReset,
    ApplyToAll,
    AppliedToAll,
    
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
    HelpDashboard,
    HelpDashboardDesc,
    HelpProcesses,
    HelpProcessesDesc,
    HelpPolicies,
    HelpPoliciesDesc,
    HelpSettingsHelp,
    HelpSettingsDesc,
    HelpTcpStates,
    HelpTcpStatesDesc,
    HelpTroubleshooting,
    HelpTroubleshootingDesc,
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

        // 全局默认设置
        texts.insert((lang, GlobalDefaultSettings), "🌐 全局默认设置");
        texts.insert((lang, GlobalDefaultDesc), "没有单独策略的进程将使用此默认设置");
        texts.insert((lang, ResetToDefault), "🔄 恢复默认值");
        texts.insert((lang, DefaultsReset), "已恢复默认值");
        texts.insert((lang, ApplyToAll), "📋 应用到所有策略");
        texts.insert((lang, AppliedToAll), "已应用到所有策略");

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
        texts.insert((lang, HelpTitle), "📖 使用指南");
        texts.insert((lang, HelpAbout), "关于本软件");
        texts.insert((lang, HelpAboutDesc), "Smart TCP Manager 是一款跨平台的 TCP 连接管理和优化工具。\n支持 Windows、macOS 和 Linux，帮助您实时监控网络连接状态，\n识别潜在问题，并优化 TCP 参数以提升网络性能。");
        texts.insert((lang, HelpFeatures), "📋 功能说明");
        texts.insert((lang, HelpDashboard), "📊 仪表盘");
        texts.insert((lang, HelpDashboardDesc), "显示系统 TCP 连接的整体概览：\n  • 总连接数、可用端口数、端口使用率\n  • 各状态连接分布（ESTABLISHED、TIME_WAIT 等）\n  • Top 5 占用连接最多的进程及其健康度评分");
        texts.insert((lang, HelpProcesses), "📋 进程列表");
        texts.insert((lang, HelpProcessesDesc), "查看每个进程的详细连接信息：\n  • 进程名、PID、各状态连接数量\n  • 健康度评分（100分制，越高越好）\n  • 点击「添加策略」选择策略模板：\n    - 📊 默认策略：通用配置\n    - 🚀 高性能：游戏/下载器\n    - 🕷️ 采集/爬虫：积极清理 CLOSE_WAIT\n    - 🖥️ 服务器：高并发服务\n    - 🔒 受限：限制连接数");
        texts.insert((lang, HelpPolicies), "📜 策略管理");
        texts.insert((lang, HelpPoliciesDesc), "为不同应用配置个性化的优化规则：\n  • TIME_WAIT 阈值：建议 100-500（超过会触发动作）\n  • CLOSE_WAIT 阈值：建议 20-100（堆积表示程序未正确关闭连接）\n  • 最大连接数：限制单个进程的连接数量\n  • 超阈值动作：告警、自动优化、忽略\n\n💡 采集工具推荐使用「🕷️ 采集/爬虫」模板，积极清理 CLOSE_WAIT 防止卡死");
        texts.insert((lang, HelpSettingsHelp), "⚙️ 系统设置");
        texts.insert((lang, HelpSettingsDesc), "调整操作系统级别的 TCP 参数（需要管理员权限）：\n  • 最大用户端口：默认 5000，建议 32768-65534\n  • TIME_WAIT 延迟：默认 120秒，建议 30-60秒\n  • 动态端口起始：默认 49152，可按需调整");
        texts.insert((lang, HelpTcpStates), "🔍 TCP 状态说明");
        texts.insert((lang, HelpTcpStatesDesc), "• ESTABLISHED（绿色）：正常活跃连接\n• TIME_WAIT（黄色）：等待关闭的连接，过多会占用端口\n• CLOSE_WAIT（红色）：对方已关闭，等待本地关闭，堆积说明程序有问题\n• LISTEN（蓝色）：监听端口，等待连接\n• FIN_WAIT/LAST_ACK：正在关闭中的连接");
        texts.insert((lang, HelpTroubleshooting), "🛠 常见问题");
        texts.insert((lang, HelpTroubleshootingDesc), "Q: 端口使用率过高怎么办？\nA: 增大最大用户端口数，减小 TIME_WAIT 延迟时间\n\nQ: 某进程 CLOSE_WAIT 很多？\nA: 这是程序问题，建议重启该进程或联系开发者\n\nQ: 修改设置后不生效？\nA: 部分设置需要重启系统才能生效");
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

        // Global default settings
        texts.insert((lang, GlobalDefaultSettings), "🌐 Global Default Settings");
        texts.insert((lang, GlobalDefaultDesc), "Processes without specific policies will use these defaults");
        texts.insert((lang, ResetToDefault), "🔄 Reset to Defaults");
        texts.insert((lang, DefaultsReset), "Defaults Reset");
        texts.insert((lang, ApplyToAll), "📋 Apply to All Policies");
        texts.insert((lang, AppliedToAll), "Applied to All Policies");

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
        texts.insert((lang, HelpTitle), "📖 User Guide");
        texts.insert((lang, HelpAbout), "About");
        texts.insert((lang, HelpAboutDesc), "Smart TCP Manager is a cross-platform TCP connection management and optimization tool.\nSupports Windows, macOS and Linux. Monitor network connections in real-time,\nidentify potential issues, and optimize TCP parameters for better performance.");
        texts.insert((lang, HelpFeatures), "📋 Features");
        texts.insert((lang, HelpDashboard), "📊 Dashboard");
        texts.insert((lang, HelpDashboardDesc), "Overview of system TCP connections:\n  • Total connections, available ports, port usage\n  • Connection distribution by state (ESTABLISHED, TIME_WAIT, etc.)\n  • Top 5 processes with most connections and health scores");
        texts.insert((lang, HelpProcesses), "📋 Process List");
        texts.insert((lang, HelpProcessesDesc), "Detailed connection info for each process:\n  • Process name, PID, connection counts by state\n  • Health score (0-100, higher is better)\n  • Click 'Add Policy' to choose a template:\n    - 📊 Default: General purpose\n    - 🚀 High Performance: Games/Downloaders\n    - 🕷️ Crawler: Aggressively clean CLOSE_WAIT\n    - 🖥️ Server: High concurrency services\n    - 🔒 Restricted: Limit connections");
        texts.insert((lang, HelpPolicies), "📜 Policies");
        texts.insert((lang, HelpPoliciesDesc), "Configure per-application optimization rules:\n  • TIME_WAIT threshold: recommended 100-500\n  • CLOSE_WAIT threshold: recommended 20-100 (accumulation indicates leak)\n  • Max connections: limit connections per process\n  • Threshold action: alert, auto-optimize, or ignore\n\n💡 For crawlers/scrapers, use the '🕷️ Crawler' template to aggressively clean CLOSE_WAIT");
        texts.insert((lang, HelpSettingsHelp), "⚙️ Settings");
        texts.insert((lang, HelpSettingsDesc), "Adjust OS-level TCP parameters (requires admin):\n  • Max user ports: default 5000, recommended 32768-65534\n  • TIME_WAIT delay: default 120s, recommended 30-60s\n  • Dynamic port start: default 49152, adjust as needed");
        texts.insert((lang, HelpTcpStates), "🔍 TCP States Explained");
        texts.insert((lang, HelpTcpStatesDesc), "• ESTABLISHED (green): Active connections\n• TIME_WAIT (yellow): Waiting to close, too many will exhaust ports\n• CLOSE_WAIT (red): Peer closed, waiting for local close - accumulation indicates bug\n• LISTEN (blue): Listening ports waiting for connections\n• FIN_WAIT/LAST_ACK: Connections being closed");
        texts.insert((lang, HelpTroubleshooting), "🛠 Troubleshooting");
        texts.insert((lang, HelpTroubleshootingDesc), "Q: Port usage too high?\nA: Increase max user ports, reduce TIME_WAIT delay\n\nQ: Process has many CLOSE_WAIT?\nA: This is a program bug. Restart the process or contact developer\n\nQ: Settings don't take effect?\nA: Some settings require system reboot");
        texts.insert((lang, HelpVersion), "Version");
    }
}

