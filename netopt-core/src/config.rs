//! 配置持久化模块
//! 
//! 提供策略和设置的保存/加载功能

use crate::policy::PolicyManager;
use crate::i18n::Language;
use crate::Result;
use crate::NetOptError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

/// 应用配置（持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 界面语言
    pub language: Language,
    
    /// 自动刷新
    pub auto_refresh: bool,
    
    /// 刷新间隔（秒）
    pub refresh_interval: u64,
    
    /// 策略管理器
    pub policy_manager: PolicyManager,
    
    /// 配置版本（用于迁移）
    pub version: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: Language::Chinese,
            auto_refresh: true,
            refresh_interval: 5,
            policy_manager: PolicyManager::new(),
            version: 1,
        }
    }
}

impl AppConfig {
    /// 获取配置文件路径
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        } else if cfg!(target_os = "macos") {
            dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
        } else {
            PathBuf::from(".")
        };
        
        let app_dir = config_dir.join("smart-tcp-manager");
        fs::create_dir_all(&app_dir).map_err(|e| NetOptError::IoError(e))?;
        
        Ok(app_dir.join("config.json"))
    }
    
    /// 从文件加载配置
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)?;
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| NetOptError::SystemError(format!("配置解析失败: {}", e)))?;
        
        Ok(config)
    }
    
    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| NetOptError::SystemError(format!("配置序列化失败: {}", e)))?;
        
        fs::write(&path, content)?;
        Ok(())
    }
    
    /// 导出策略到指定文件
    pub fn export_policies(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.policy_manager)
            .map_err(|e| NetOptError::SystemError(format!("策略序列化失败: {}", e)))?;
        
        fs::write(path, content)?;
        Ok(())
    }
    
    /// 从文件导入策略
    pub fn import_policies(&mut self, path: &PathBuf) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let policy_manager: PolicyManager = serde_json::from_str(&content)
            .map_err(|e| NetOptError::SystemError(format!("策略解析失败: {}", e)))?;
        
        self.policy_manager = policy_manager;
        Ok(())
    }
}

/// 配置目录相关函数
pub mod dirs {
    use std::path::PathBuf;
    
    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        }
        
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        }
    }
}

