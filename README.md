# Smart TCP Manager

[English](#english) | [中文](#中文)

---

<a name="english"></a>
## English

A cross-platform TCP connection optimization tool with GUI, supporting Windows and macOS.

### Features

- 📊 **System Dashboard** - Real-time TCP connection overview, port usage statistics
- 📋 **Process Monitoring** - View TCP connection distribution per process
- 🏥 **Health Scoring** - Auto-detect problematic processes (excessive TIME_WAIT/CLOSE_WAIT)
- 📜 **Policy Engine** - Configure different optimization policies for different apps
- ⚙️ **System Tuning** - Modify TCP parameters (MaxUserPort, TcpTimedWaitDelay, etc.)
- 🌐 **i18n Support** - Chinese/English interface switching
- 💾 **Config Persistence** - Auto-save policies and settings

### Screenshots

```
┌─────────────────────────────────────────────────────────┐
│ 🌐 Smart TCP Manager                                    │
│ [Dashboard] [Processes] [Policies] [Settings]    [EN▼] │
├─────────────────────────────────────────────────────────┤
│ System TCP Overview                                      │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐                 │
│ │ Total    │ │ Available│ │ Usage    │                 │
│ │   1,234  │ │  64,300  │ │   1.9%   │                 │
│ └──────────┘ └──────────┘ └──────────┘                 │
└─────────────────────────────────────────────────────────┘
```

### Installation

#### From Source

```bash
# Clone the repository
git clone https://github.com/LegnaOS/smart-tcp-manager.git
cd smart-tcp-manager

# Build release version
cargo build --release

# Run GUI
./target/release/netopt-gui

# Run background service (requires admin)
sudo ./target/release/netopt-service
```

#### From Release

Download pre-built binaries from [Releases](https://github.com/LegnaOS/smart-tcp-manager/releases).

### Usage

```bash
# GUI Application
./netopt-gui

# Background Service (admin required for system modifications)
sudo ./netopt-service
```

### Platform Support

| Platform | Monitor | Modify Settings | Close Connections |
|----------|---------|-----------------|-------------------|
| macOS    | ✅      | ✅ (sysctl)     | ❌                |
| Windows  | ✅      | ✅ (Registry)   | ✅ (SetTcpEntry)  |

### Building for Multiple Platforms

```bash
# Run release script
./scripts/release.sh 1.0.0

# Publish to GitHub (requires gh CLI)
./scripts/github-release.sh 1.0.0
```

### License

MIT License

---

<a name="中文"></a>
## 中文

跨平台 TCP 连接优化工具，带图形界面，支持 Windows 和 macOS。

### 功能特性

- 📊 **系统仪表盘** - 实时 TCP 连接概览，端口使用统计
- 📋 **进程监控** - 查看每个进程的 TCP 连接状态分布
- 🏥 **健康评分** - 自动检测问题进程（TIME_WAIT/CLOSE_WAIT 过多）
- 📜 **策略引擎** - 为不同应用配置不同优化策略
- ⚙️ **系统调优** - 修改 TCP 参数（MaxUserPort, TcpTimedWaitDelay 等）
- 🌐 **国际化** - 支持中文/英文界面切换
- 💾 **配置持久化** - 自动保存策略和设置

### 安装

#### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/LegnaOS/smart-tcp-manager.git
cd smart-tcp-manager

# 编译 release 版本
cargo build --release

# 运行 GUI
./target/release/netopt-gui

# 运行后台服务（需要管理员权限）
sudo ./target/release/netopt-service
```

#### 从 Release 下载

从 [Releases](https://github.com/LegnaOS/smart-tcp-manager/releases) 下载预编译版本。

### 使用方法

```bash
# GUI 应用
./netopt-gui

# 后台服务（修改系统设置需要管理员权限）
sudo ./netopt-service
```

### 平台支持

| 平台     | 监控 | 修改设置        | 关闭连接          |
|----------|------|-----------------|-------------------|
| macOS    | ✅   | ✅ (sysctl)     | ❌                |
| Windows  | ✅   | ✅ (注册表)     | ✅ (SetTcpEntry)  |

### 多平台编译

```bash
# 运行发布脚本
./scripts/release.sh 1.0.0

# 发布到 GitHub（需要 gh CLI）
./scripts/github-release.sh 1.0.0
```

### 开源协议

MIT License

