//! NetOpt GUI - TCP连接优化管理界面
//!
//! 提供直观的GUI界面管理TCP连接，支持中英文切换

use eframe::egui;
use netopt_core::platform::{create_monitor, create_config_manager, has_admin_privileges, platform_name};
use netopt_core::{SystemTcpStats, TcpState, TcpSystemConfig};
use netopt_core::{I18n, Language, TextKey, AppConfig};
use netopt_core::policy::{AppPolicy, ThresholdAction};

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// 后台线程消息
enum BgMessage {
    StatsResult(Result<SystemTcpStats, String>),
}

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
        Box::new(|cc| {
            // 加载中文字体
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(NetOptApp::new(cc)))
        }),
    )
}

/// 设置支持中文的字体
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 加载系统中文字体
    #[cfg(target_os = "macos")]
    {
        // macOS: 使用苹方或华文黑体
        if let Ok(font_data) = std::fs::read("/System/Library/Fonts/PingFang.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families.entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace)
                .or_default()
                .push("chinese".to_owned());
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: 使用微软雅黑
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            fonts.families.entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace)
                .or_default()
                .push("chinese".to_owned());
        }
    }

    ctx.set_fonts(fonts);
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
    is_admin: bool,
    tcp_config: TcpSystemConfig,
    status_message: String,

    // 国际化
    i18n: I18n,

    // 持久化配置
    app_config: AppConfig,
    config_dirty: bool,

    // 后台刷新
    bg_receiver: Receiver<BgMessage>,
    bg_sender: Sender<BgMessage>,
    is_refreshing: bool,
}

impl NetOptApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_mgr = create_config_manager();
        let tcp_config = config_mgr.get_current_config().unwrap_or_default();

        // 加载持久化配置
        let app_config = AppConfig::load().unwrap_or_default();

        let mut i18n = I18n::new();
        i18n.set_language(app_config.language);

        let (bg_sender, bg_receiver) = channel();

        Self {
            current_view: View::Dashboard,
            stats: None,
            last_refresh: Instant::now() - Duration::from_secs(100),
            is_admin: has_admin_privileges(),
            tcp_config,
            status_message: String::new(),
            i18n,
            app_config,
            config_dirty: false,
            bg_receiver,
            bg_sender,
            is_refreshing: false,
        }
    }

    /// 在后台线程刷新统计数据（非阻塞）
    fn refresh_stats_async(&mut self) {
        if self.is_refreshing {
            return; // 已在刷新中
        }
        self.is_refreshing = true;
        self.status_message = self.i18n.t(TextKey::Refreshing).to_string();

        let sender = self.bg_sender.clone();
        std::thread::spawn(move || {
            let monitor = create_monitor();
            let result = monitor.get_system_stats()
                .map_err(|e| e.to_string());
            let _ = sender.send(BgMessage::StatsResult(result));
        });
    }

    /// 处理后台消息
    fn process_bg_messages(&mut self) {
        while let Ok(msg) = self.bg_receiver.try_recv() {
            match msg {
                BgMessage::StatsResult(result) => {
                    self.is_refreshing = false;
                    self.last_refresh = Instant::now();
                    match result {
                        Ok(stats) => {
                            self.stats = Some(stats);
                            self.status_message = format!("{} - {}", self.i18n.t(TextKey::RefreshSuccess), platform_name());
                        }
                        Err(e) => {
                            self.status_message = format!("{}: {}", self.i18n.t(TextKey::RefreshFailed), e);
                        }
                    }
                }
            }
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.app_config.save() {
            self.status_message = format!("保存配置失败: {}", e);
        } else {
            self.config_dirty = false;
        }
    }

    fn t(&self, key: TextKey) -> &'static str {
        self.i18n.t(key)
    }
}

impl eframe::App for NetOptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理后台消息
        self.process_bg_messages();

        // 自动刷新（非阻塞）
        if self.app_config.auto_refresh && !self.is_refreshing && self.last_refresh.elapsed() > Duration::from_secs(self.app_config.refresh_interval) {
            self.refresh_stats_async();
        }

        // 顶部导航栏
        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🌐 Smart TCP Manager");
                ui.separator();

                if ui.selectable_label(self.current_view == View::Dashboard, self.t(TextKey::Dashboard)).clicked() {
                    self.current_view = View::Dashboard;
                }
                if ui.selectable_label(self.current_view == View::Processes, self.t(TextKey::Processes)).clicked() {
                    self.current_view = View::Processes;
                }
                if ui.selectable_label(self.current_view == View::Policies, self.t(TextKey::Policies)).clicked() {
                    self.current_view = View::Policies;
                }
                if ui.selectable_label(self.current_view == View::Settings, self.t(TextKey::Settings)).clicked() {
                    self.current_view = View::Settings;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 语言切换
                    let current_lang = self.i18n.current_language();
                    egui::ComboBox::from_id_salt("lang_selector")
                        .selected_text(current_lang.display_name())
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(current_lang == Language::Chinese, "中文").clicked() {
                                self.i18n.set_language(Language::Chinese);
                                self.app_config.language = Language::Chinese;
                                self.config_dirty = true;
                            }
                            if ui.selectable_label(current_lang == Language::English, "English").clicked() {
                                self.i18n.set_language(Language::English);
                                self.app_config.language = Language::English;
                                self.config_dirty = true;
                            }
                        });

                    ui.separator();

                    if !self.is_admin {
                        ui.label(egui::RichText::new(self.t(TextKey::AdminRequired)).color(egui::Color32::YELLOW));
                    } else {
                        ui.label(egui::RichText::new(self.t(TextKey::AdminGranted)).color(egui::Color32::GREEN));
                    }
                });
            });
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut auto_refresh = self.app_config.auto_refresh;
                    if ui.checkbox(&mut auto_refresh, self.t(TextKey::AutoRefresh)).changed() {
                        self.app_config.auto_refresh = auto_refresh;
                        self.config_dirty = true;
                    }
                    // 刷新按钮（显示刷新状态）
                    let refresh_btn = if self.is_refreshing {
                        ui.add_enabled(false, egui::Button::new("⏳"))
                    } else {
                        ui.button(self.t(TextKey::RefreshNow))
                    };
                    if refresh_btn.clicked() {
                        self.refresh_stats_async();
                    }

                    // 自动保存配置
                    if self.config_dirty {
                        self.save_config();
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

        // 持续刷新UI（在后台刷新时更频繁地刷新以响应结果）
        if self.is_refreshing {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else if self.app_config.auto_refresh {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl NetOptApp {
    /// 仪表盘视图
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        if self.stats.is_none() && !self.is_refreshing {
            self.refresh_stats_async();
        }

        let Some(stats) = &self.stats else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
            return;
        };

        ui.heading(self.t(TextKey::SystemOverview));
        ui.add_space(10.0);

        // 概览卡片
        let t_total = self.t(TextKey::TotalConnections);
        let t_avail = self.t(TextKey::AvailablePorts);
        let t_usage = self.t(TextKey::PortUsage);

        ui.horizontal(|ui| {
            Self::stat_card(ui, t_total, &stats.total_connections.to_string(), egui::Color32::LIGHT_BLUE);
            Self::stat_card(ui, t_avail, &stats.available_ports.to_string(), egui::Color32::LIGHT_GREEN);
            let color = if stats.port_usage_percent > 80.0 {
                egui::Color32::RED
            } else if stats.port_usage_percent > 50.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::LIGHT_GREEN
            };
            Self::stat_card(ui, t_usage, &format!("{:.1}%", stats.port_usage_percent), color);
        });

        ui.add_space(20.0);

        // 连接状态分布
        ui.heading(self.t(TextKey::ConnectionStateDistribution));

        let t_active = self.t(TextKey::ActiveConnections);
        let t_waiting = self.t(TextKey::WaitingClose);
        let t_attention = self.t(TextKey::NeedsAttention);
        let t_listen = self.t(TextKey::ListeningPorts);

        let states = [
            (TcpState::Established, "🟢", t_active),
            (TcpState::TimeWait, "🟡", t_waiting),
            (TcpState::CloseWait, "🔴", t_attention),
            (TcpState::Listen, "🔵", t_listen),
        ];

        egui::Grid::new("state_grid").striped(true).show(ui, |ui| {
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
        let t_proc = self.t(TextKey::ProcessName);
        let t_pid = self.t(TextKey::Pid);
        let t_conn = self.t(TextKey::Connections);
        let t_health = self.t(TextKey::HealthScore);

        ui.heading(self.t(TextKey::Top5Processes));
        egui::Grid::new("top_procs").striped(true).show(ui, |ui| {
            ui.label(t_proc);
            ui.label(t_pid);
            ui.label(t_conn);
            ui.label("TIME_WAIT");
            ui.label("CLOSE_WAIT");
            ui.label(t_health);
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
        // Clone data we need to avoid borrow conflicts
        let processes = match &self.stats {
            Some(stats) => stats.by_process.clone(),
            None => {
                ui.spinner();
                return;
            }
        };

        ui.heading(self.t(TextKey::ProcessDetails));
        ui.add_space(10.0);

        let t_proc = self.t(TextKey::ProcessName);
        let t_pid = self.t(TextKey::Pid);
        let t_health = self.t(TextKey::HealthScore);
        let t_add = self.t(TextKey::AddPolicy);
        let t_added = self.t(TextKey::PolicyAdded);

        // Collect actions to perform after iteration
        let mut add_policy_for: Option<String> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("proc_grid").striped(true).show(ui, |ui| {
                ui.label(egui::RichText::new(t_proc).strong());
                ui.label(egui::RichText::new(t_pid).strong());
                ui.label(egui::RichText::new("Total").strong());
                ui.label(egui::RichText::new("ESTABLISHED").strong());
                ui.label(egui::RichText::new("TIME_WAIT").strong());
                ui.label(egui::RichText::new("CLOSE_WAIT").strong());
                ui.label(egui::RichText::new("LISTEN").strong());
                ui.label(egui::RichText::new(t_health).strong());
                ui.label(egui::RichText::new(t_add).strong());
                ui.end_row();

                for proc in &processes {
                    ui.label(&proc.process_name);
                    ui.label(proc.pid.to_string());
                    ui.label(proc.total_connections.to_string());
                    ui.label(proc.established.to_string());
                    ui.label(proc.time_wait.to_string());
                    ui.label(proc.close_wait.to_string());
                    ui.label(proc.listen.to_string());
                    ui.label(format!("{}%", proc.health_score));

                    if ui.button(t_add).clicked() {
                        add_policy_for = Some(proc.process_name.clone());
                    }
                    ui.end_row();
                }
            });
        });

        // Apply action outside of closure
        if let Some(process_name) = add_policy_for {
            let policy = AppPolicy::default();
            self.app_config.policy_manager.set_policy(AppPolicy {
                process_name: process_name.clone(),
                ..policy
            });
            self.status_message = format!("{}: {}", t_added, process_name);
            self.config_dirty = true;
        }
    }

    /// 策略管理视图
    fn show_policies(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t(TextKey::PolicyManagement));
        ui.label(self.t(TextKey::PolicyDescription));
        ui.add_space(10.0);

        let policies: Vec<_> = self.app_config.policy_manager.all_policies().iter().map(|p| (*p).clone()).collect();

        if policies.is_empty() {
            ui.label(self.t(TextKey::NoPolicies));
            return;
        }

        let t_auto = self.t(TextKey::AutoOptimize);
        let t_enabled = self.t(TextKey::Enabled);
        let t_disabled = self.t(TextKey::Disabled);
        let t_tw = self.t(TextKey::TimeWaitThreshold);
        let t_cw = self.t(TextKey::CloseWaitThreshold);
        let t_max = self.t(TextKey::MaxConnections);
        let t_unlimited = self.t(TextKey::Unlimited);
        let t_action = self.t(TextKey::ThresholdAction);
        let t_alert = self.t(TextKey::ActionAlert);
        let t_optimize = self.t(TextKey::ActionOptimize);
        let t_restart = self.t(TextKey::ActionRestart);
        let t_ignore = self.t(TextKey::ActionIgnore);
        let t_delete = self.t(TextKey::DeletePolicy);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for policy in &policies {
                egui::CollapsingHeader::new(&policy.process_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new(format!("policy_{}", policy.process_name)).show(ui, |ui| {
                            ui.label(format!("{}:", t_auto));
                            ui.label(if policy.auto_optimize { t_enabled } else { t_disabled });
                            ui.end_row();

                            ui.label(format!("{}:", t_tw));
                            ui.label(policy.time_wait_threshold.map(|v| v.to_string()).unwrap_or_else(|| t_unlimited.to_string()));
                            ui.end_row();

                            ui.label(format!("{}:", t_cw));
                            ui.label(policy.close_wait_threshold.map(|v| v.to_string()).unwrap_or_else(|| t_unlimited.to_string()));
                            ui.end_row();

                            ui.label(format!("{}:", t_max));
                            ui.label(policy.max_connections.map(|v| v.to_string()).unwrap_or_else(|| t_unlimited.to_string()));
                            ui.end_row();

                            ui.label(format!("{}:", t_action));
                            ui.label(match policy.threshold_action {
                                ThresholdAction::Alert => t_alert,
                                ThresholdAction::Optimize => t_optimize,
                                ThresholdAction::RestartProcess => t_restart,
                                ThresholdAction::Ignore => t_ignore,
                            });
                            ui.end_row();
                        });

                        ui.horizontal(|ui| {
                            if ui.button(t_delete).clicked() {
                                self.app_config.policy_manager.remove_policy(&policy.process_name);
                                self.status_message = format!("{}: {}", self.t(TextKey::PolicyDeleted), policy.process_name);
                                self.config_dirty = true;
                            }
                        });
                    });
            }
        });

        ui.add_space(20.0);

        // 预设策略模板
        ui.heading(self.t(TextKey::QuickTemplates));
        ui.horizontal(|ui| {
            if ui.button(self.t(TextKey::GameMode)).clicked() {
                self.app_config.policy_manager.set_policy(AppPolicy::high_performance("game"));
                self.status_message = self.t(TextKey::TemplateAdded).into();
                self.config_dirty = true;
            }
            if ui.button(self.t(TextKey::ServerMode)).clicked() {
                self.app_config.policy_manager.set_policy(AppPolicy::server("server"));
                self.status_message = self.t(TextKey::TemplateAdded).into();
                self.config_dirty = true;
            }
            if ui.button(self.t(TextKey::RestrictedMode)).clicked() {
                self.app_config.policy_manager.set_policy(AppPolicy::restricted("suspicious"));
                self.status_message = self.t(TextKey::TemplateAdded).into();
                self.config_dirty = true;
            }
        });
    }

    /// 系统设置视图
    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t(TextKey::TcpSettings));

        if !self.is_admin {
            ui.colored_label(
                egui::Color32::YELLOW,
                self.t(TextKey::AdminRequiredForSettings)
            );
            ui.add_space(10.0);
        }

        ui.add_space(10.0);

        // 预设配置
        ui.heading(self.t(TextKey::QuickConfig));
        let t_loaded = self.t(TextKey::ConfigLoaded);
        ui.horizontal(|ui| {
            if ui.button(self.t(TextKey::HighPerformance)).clicked() {
                self.tcp_config = TcpSystemConfig::high_performance();
                self.status_message = t_loaded.into();
            }
            if ui.button(self.t(TextKey::Conservative)).clicked() {
                self.tcp_config = TcpSystemConfig::conservative();
                self.status_message = t_loaded.into();
            }
            if ui.button(self.t(TextKey::ReadCurrent)).clicked() {
                let mgr = create_config_manager();
                if let Ok(c) = mgr.get_current_config() {
                    self.tcp_config = c;
                    self.status_message = t_loaded.into();
                }
            }
        });

        ui.add_space(20.0);

        // 详细配置
        ui.heading(self.t(TextKey::DetailedConfig));
        let t_rec = self.t(TextKey::Recommended);
        egui::Grid::new("config_grid").show(ui, |ui| {
            // MaxUserPort
            ui.label(self.t(TextKey::MaxUserPort));
            let mut port = self.tcp_config.max_user_port.unwrap_or(5000);
            if ui.add(egui::Slider::new(&mut port, 1024..=65534)).changed() {
                self.tcp_config.max_user_port = Some(port);
            }
            ui.label(format!("{}: 65534", t_rec));
            ui.end_row();

            // TcpTimedWaitDelay
            ui.label(self.t(TextKey::TimeWaitDelay));
            let mut delay = self.tcp_config.time_wait_delay.unwrap_or(240);
            if ui.add(egui::Slider::new(&mut delay, 30..=300)).changed() {
                self.tcp_config.time_wait_delay = Some(delay);
            }
            ui.label(format!("{}: 30s", t_rec));
            ui.end_row();

            // 动态端口起始
            ui.label(self.t(TextKey::DynamicPortStart));
            let mut start = self.tcp_config.dynamic_port_start.unwrap_or(1025);
            if ui.add(egui::Slider::new(&mut start, 1025..=49151)).changed() {
                self.tcp_config.dynamic_port_start = Some(start);
            }
            ui.label(format!("{}: 10000", t_rec));
            ui.end_row();
        });

        ui.add_space(20.0);

        // 应用按钮
        let t_applied = self.t(TextKey::ConfigApplied);
        let t_failed = self.t(TextKey::ApplyFailed);
        ui.horizontal(|ui| {
            let apply_btn = ui.add_enabled(self.is_admin, egui::Button::new(self.t(TextKey::ApplyConfig)));
            if apply_btn.clicked() {
                let mgr = create_config_manager();
                match mgr.apply_config(&self.tcp_config) {
                    Ok(_) => {
                        self.status_message = t_applied.into();
                    }
                    Err(e) => {
                        self.status_message = format!("{}: {}", t_failed, e);
                    }
                }
            }

            if mgr_requires_reboot() {
                ui.colored_label(egui::Color32::YELLOW, self.t(TextKey::RebootRequired));
            }
        });
    }
}

fn mgr_requires_reboot() -> bool {
    let mgr = create_config_manager();
    mgr.requires_reboot()
}

