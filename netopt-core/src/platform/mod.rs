//! 平台抽象层
//! 
//! 提供跨平台的TCP配置和监控能力

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

use crate::tcp_config::TcpConfigManager;
use crate::monitor::TcpMonitor;
use crate::optimizer::ConnectionOptimizer;

/// 创建平台特定的配置管理器
pub fn create_config_manager() -> Box<dyn TcpConfigManager> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsTcpConfigManager::new())
    }
    
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsTcpConfigManager::new())
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        compile_error!("Unsupported platform");
    }
}

/// 创建平台特定的监控器
pub fn create_monitor() -> Box<dyn TcpMonitor> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsTcpMonitor::new())
    }
    
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsTcpMonitor::new())
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        compile_error!("Unsupported platform");
    }
}

/// 创建平台特定的优化器
pub fn create_optimizer() -> Box<dyn ConnectionOptimizer> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsConnectionOptimizer::new())
    }
    
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsConnectionOptimizer::new())
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        compile_error!("Unsupported platform");
    }
}

/// 获取当前平台名称
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    { "Windows" }
    
    #[cfg(target_os = "macos")]
    { "macOS" }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { "Unknown" }
}

/// 检查是否有管理员/root权限
pub fn has_admin_privileges() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::is_elevated()
    }
    
    #[cfg(target_os = "macos")]
    {
        macos::is_root()
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { false }
}

