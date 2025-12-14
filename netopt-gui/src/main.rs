//! NetOpt GUI - TCP连接优化管理界面
//!
//! 提供直观的GUI界面管理TCP连接

use eframe::egui;
use netopt_core::platform::{create_monitor, create_config_manager, has_admin_privileges, platform_name};
use netopt_core::{SystemTcpStats, TcpState, ProcessTcpStats, TcpSystemConfig};
use netopt_core::policy::{PolicyManager, AppPolicy, ThresholdAction};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Smart TCP Manager",
        options,
        Box::new(|cc| Ok(Box::new(NetOptApp::new(cc)))),
    )
}

/// 应用主视图
#[derive(Default, PartialEq)]
enum View {
    #[default]
    Dashboard,
    Processes,
    Settings,
    Policies,
}

/// 主应用状态
struct NetOptApp {
    current_view: View,
    stats: Option<SystemTcpStats>,
    last_refresh: Instant,
    auto_refresh: bool,
    refresh_interval: u64,
    is_admin: bool,
    policy_manager: PolicyManager,
    selected_process: Option<u32>,
    config: TcpSystemConfig,
    status_message: String,
}

impl NetOptApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_mgr = create_config_manager();
        let config = config_mgr.get_current_config().unwrap_or_default();
        
        Self {
            current_view: View::Dashboard,
            stats: None,
            last_refresh: Instant::now() - Duration::from_secs(100),
            auto_refresh: true,
            refresh_interval: 5,
            is_admin: has_admin_privileges(),
            policy_manager: PolicyManager::new(),
            selected_process: None,
            config,
            status_message: String::new(),
        }
    }
    
    fn refresh_stats(&mut self) {
        let monitor = create_monitor();
        match monitor.get_system_stats() {
            Ok(stats) => {
                self.stats = Some(stats);
                self.status_message = format!("刷新成功 - {}", platform_name());
            }
            Err(e) => {
                self.status_message = format!("刷新失败: {}", e);
            }
        }
        self.last_refresh = Instant::now();
    }
}

impl eframe::App for NetOptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 自动刷新
        if self.auto_refresh && self.last_refresh.elapsed() > Duration::from_secs(self.refresh_interval) {
            self.refresh_stats();
        }
        
        // 顶部导航栏
        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🌐 Smart TCP Manager");
                ui.separator();
                
                if ui.selectable_label(self.current_view == View::Dashboard, "📊 仪表盘").clicked() {
                    self.current_view = View::Dashboard;
                }
                if ui.selectable_label(self.current_view == View::Processes, "📋 进程列表").clicked() {
                    self.current_view = View::Processes;
                }
                if ui.selectable_label(self.current_view == View::Policies, "📜 策略管理").clicked() {
                    self.current_view = View::Policies;
                }
                if ui.selectable_label(self.current_view == View::Settings, "⚙️ 系统设置").clicked() {
                    self.current_view = View::Settings;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.is_admin {
                        ui.label(egui::RichText::new("⚠️ 需要管理员权限").color(egui::Color32::YELLOW));
                    } else {
                        ui.label(egui::RichText::new("✓ 管理员").color(egui::Color32::GREEN));
                    }
                });
            });
        });
        
        // 底部状态栏
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.auto_refresh, "自动刷新");
                    if ui.button("🔄 立即刷新").clicked() {
                        self.refresh_stats();
                    }
                });
            });
        });
        
        // 主内容区
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                View::Dashboard => self.show_dashboard(ui),
                View::Processes => self.show_processes(ui),
                View::Policies => self.show_policies(ui),
                View::Settings => self.show_settings(ui),
            }
        });
        
        // 持续刷新UI
        if self.auto_refresh {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl NetOptApp {
    /// 仪表盘视图
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        if self.stats.is_none() {
            self.refresh_stats();
        }

        let Some(stats) = &self.stats else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
            return;
        };

        ui.heading("系统TCP连接概览");
        ui.add_space(10.0);

        // 概览卡片
        ui.horizontal(|ui| {
            // 总连接数
            Self::stat_card(ui, "总连接数", &stats.total_connections.to_string(), egui::Color32::LIGHT_BLUE);
            // 可用端口
            Self::stat_card(ui, "可用端口", &stats.available_ports.to_string(), egui::Color32::LIGHT_GREEN);
            // 端口使用率
            let color = if stats.port_usage_percent > 80.0 {
                egui::Color32::RED
            } else if stats.port_usage_percent > 50.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::LIGHT_GREEN
            };
            Self::stat_card(ui, "端口使用率", &format!("{:.1}%", stats.port_usage_percent), color);
        });

        ui.add_space(20.0);

        // 连接状态分布
        ui.heading("连接状态分布");
        egui::Grid::new("state_grid").striped(true).show(ui, |ui| {
            ui.label("状态");
            ui.label("数量");
            ui.label("占比");
            ui.label("状态");
            ui.end_row();

            let states = [
                (TcpState::Established, "🟢", "活跃连接"),
                (TcpState::TimeWait, "🟡", "等待关闭"),
                (TcpState::CloseWait, "🔴", "需注意"),
                (TcpState::Listen, "🔵", "监听端口"),
            ];

            for (state, icon, desc) in states {
                let count = stats.by_state.get(&state).unwrap_or(&0);
                let percent = if stats.total_connections > 0 {
                    (*count as f32 / stats.total_connections as f32) * 100.0
                } else {
                    0.0
                };

                ui.label(format!("{} {}", icon, state));
                ui.label(count.to_string());
                ui.label(format!("{:.1}%", percent));
                ui.label(desc);
                ui.end_row();
            }
        });

        ui.add_space(20.0);

        // Top 5 进程
        ui.heading("连接数Top 5进程");
        egui::Grid::new("top_procs").striped(true).show(ui, |ui| {
            ui.label("进程");
            ui.label("PID");
            ui.label("连接数");
            ui.label("TIME_WAIT");
            ui.label("CLOSE_WAIT");
            ui.label("健康度");
            ui.end_row();

            for proc in stats.by_process.iter().take(5) {
                ui.label(&proc.process_name);
                ui.label(proc.pid.to_string());
                ui.label(proc.total_connections.to_string());

                let tw_color = if proc.time_wait > 100 { egui::Color32::YELLOW } else { egui::Color32::WHITE };
                ui.colored_label(tw_color, proc.time_wait.to_string());

                let cw_color = if proc.close_wait > 50 { egui::Color32::RED } else { egui::Color32::WHITE };
                ui.colored_label(cw_color, proc.close_wait.to_string());

                let health_color = if proc.health_score >= 80 {
                    egui::Color32::GREEN
                } else if proc.health_score >= 50 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.colored_label(health_color, format!("{}%", proc.health_score));
                ui.end_row();
            }
        });
    }

    fn stat_card(ui: &mut egui::Ui, title: &str, value: &str, color: egui::Color32) {
        egui::Frame::none()
            .fill(egui::Color32::from_gray(40))
            .rounding(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(title);
                    ui.colored_label(color, egui::RichText::new(value).size(28.0).strong());
                });
            });
    }

    /// 进程列表视图
    fn show_processes(&mut self, ui: &mut egui::Ui) {
        let Some(stats) = &self.stats else {
            ui.label("加载中...");
            return;
        };

        ui.heading("进程TCP连接详情");
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("proc_grid").striped(true).show(ui, |ui| {
                ui.label(egui::RichText::new("进程名").strong());
                ui.label(egui::RichText::new("PID").strong());
                ui.label(egui::RichText::new("总连接").strong());
                ui.label(egui::RichText::new("ESTABLISHED").strong());
                ui.label(egui::RichText::new("TIME_WAIT").strong());
                ui.label(egui::RichText::new("CLOSE_WAIT").strong());
                ui.label(egui::RichText::new("LISTEN").strong());
                ui.label(egui::RichText::new("健康度").strong());
                ui.label(egui::RichText::new("操作").strong());
                ui.end_row();

                for proc in &stats.by_process {
                    ui.label(&proc.process_name);
                    ui.label(proc.pid.to_string());
                    ui.label(proc.total_connections.to_string());
                    ui.label(proc.established.to_string());
                    ui.label(proc.time_wait.to_string());
                    ui.label(proc.close_wait.to_string());
                    ui.label(proc.listen.to_string());
                    ui.label(format!("{}%", proc.health_score));

                    if ui.button("添加策略").clicked() {
                        let policy = AppPolicy::default();
                        self.policy_manager.set_policy(AppPolicy {
                            process_name: proc.process_name.clone(),
                            ..policy
                        });
                        self.status_message = format!("已为 {} 添加默认策略", proc.process_name);
                    }
                    ui.end_row();
                }
            });
        });
    }

    /// 策略管理视图
    fn show_policies(&mut self, ui: &mut egui::Ui) {
        ui.heading("应用策略管理");
        ui.label("为不同应用配置不同的TCP连接优化策略");
        ui.add_space(10.0);

        let policies: Vec<_> = self.policy_manager.all_policies().iter().map(|p| (*p).clone()).collect();

        if policies.is_empty() {
            ui.label("暂无策略，请在进程列表中添加");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for policy in &policies {
                egui::CollapsingHeader::new(&policy.process_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new(format!("policy_{}", policy.process_name)).show(ui, |ui| {
                            ui.label("自动优化:");
                            ui.label(if policy.auto_optimize { "✓ 开启" } else { "✗ 关闭" });
                            ui.end_row();

                            ui.label("TIME_WAIT阈值:");
                            ui.label(policy.time_wait_threshold.to_string());
                            ui.end_row();

                            ui.label("CLOSE_WAIT阈值:");
                            ui.label(policy.close_wait_threshold.to_string());
                            ui.end_row();

                            ui.label("最大连接数:");
                            ui.label(if policy.max_connections == 0 {
                                "不限制".to_string()
                            } else {
                                policy.max_connections.to_string()
                            });
                            ui.end_row();

                            ui.label("超阈值动作:");
                            ui.label(match policy.threshold_action {
                                ThresholdAction::Alert => "告警",
                                ThresholdAction::Optimize => "自动优化",
                                ThresholdAction::RestartProcess => "重启进程",
                                ThresholdAction::Ignore => "忽略",
                            });
                            ui.end_row();
                        });

                        ui.horizontal(|ui| {
                            if ui.button("🗑 删除策略").clicked() {
                                self.policy_manager.remove_policy(&policy.process_name);
                                self.status_message = format!("已删除 {} 的策略", policy.process_name);
                            }
                        });
                    });
            }
        });

        ui.add_space(20.0);

        // 预设策略模板
        ui.heading("快速应用模板");
        ui.horizontal(|ui| {
            if ui.button("🎮 游戏模式").clicked() {
                self.policy_manager.set_policy(AppPolicy::high_performance("game"));
                self.status_message = "已添加游戏模式策略模板".into();
            }
            if ui.button("🖥 服务器模式").clicked() {
                self.policy_manager.set_policy(AppPolicy::server("server"));
                self.status_message = "已添加服务器模式策略模板".into();
            }
            if ui.button("🔒 限制模式").clicked() {
                self.policy_manager.set_policy(AppPolicy::restricted("suspicious"));
                self.status_message = "已添加限制模式策略模板".into();
            }
        });
    }

    /// 系统设置视图
    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("TCP系统参数设置");

        if !self.is_admin {
            ui.colored_label(
                egui::Color32::YELLOW,
                "⚠️ 需要管理员权限才能修改系统设置"
            );
            ui.add_space(10.0);
        }

        ui.add_space(10.0);

        // 预设配置
        ui.heading("快速配置");
        ui.horizontal(|ui| {
            if ui.button("🚀 高性能配置").clicked() {
                self.config = TcpSystemConfig::high_performance();
                self.status_message = "已加载高性能配置（未应用）".into();
            }
            if ui.button("🛡 保守配置").clicked() {
                self.config = TcpSystemConfig::conservative();
                self.status_message = "已加载保守配置（未应用）".into();
            }
            if ui.button("🔄 读取当前").clicked() {
                let mgr = create_config_manager();
                if let Ok(c) = mgr.get_current_config() {
                    self.config = c;
                    self.status_message = "已读取当前系统配置".into();
                }
            }
        });

        ui.add_space(20.0);

        // 详细配置
        ui.heading("详细配置");
        egui::Grid::new("config_grid").show(ui, |ui| {
            // MaxUserPort
            ui.label("最大用户端口 (MaxUserPort):");
            let mut port = self.config.max_user_port.unwrap_or(5000);
            if ui.add(egui::Slider::new(&mut port, 1024..=65534)).changed() {
                self.config.max_user_port = Some(port);
            }
            ui.label("推荐: 65534");
            ui.end_row();

            // TcpTimedWaitDelay
            ui.label("TIME_WAIT等待时间 (秒):");
            let mut delay = self.config.time_wait_delay.unwrap_or(240);
            if ui.add(egui::Slider::new(&mut delay, 30..=300)).changed() {
                self.config.time_wait_delay = Some(delay);
            }
            ui.label("推荐: 30秒");
            ui.end_row();

            // 动态端口起始
            ui.label("动态端口起始:");
            let mut start = self.config.dynamic_port_start.unwrap_or(1025);
            if ui.add(egui::Slider::new(&mut start, 1025..=49151)).changed() {
                self.config.dynamic_port_start = Some(start);
            }
            ui.label("推荐: 10000");
            ui.end_row();
        });

        ui.add_space(20.0);

        // 应用按钮
        ui.horizontal(|ui| {
            let apply_btn = ui.add_enabled(self.is_admin, egui::Button::new("✅ 应用配置"));
            if apply_btn.clicked() {
                let mgr = create_config_manager();
                match mgr.apply_config(&self.config) {
                    Ok(_) => {
                        self.status_message = "配置已应用！可能需要重启系统生效。".into();
                    }
                    Err(e) => {
                        self.status_message = format!("应用失败: {}", e);
                    }
                }
            }

            if mgr_requires_reboot() {
                ui.colored_label(egui::Color32::YELLOW, "⚠️ 修改后需要重启系统生效");
            }
        });
    }
}

fn mgr_requires_reboot() -> bool {
    let mgr = create_config_manager();
    mgr.requires_reboot()
}

