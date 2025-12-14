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
/// 按优先级尝试加载系统字体
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // macOS 中文字体路径（按优先级排序）
    #[cfg(target_os = "macos")]
    let font_paths = [
        "/System/Library/Fonts/PingFang.ttc",           // 苹方 - macOS 默认
        "/System/Library/Fonts/STHeiti Light.ttc",      // 华文黑体
        "/Library/Fonts/Hiragino Sans GB.ttc",          // 冬青黑体
        "/Library/Fonts/Arial Unicode.ttf",             // Arial Unicode
    ];

    // Windows 中文字体路径
    #[cfg(target_os = "windows")]
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",                 // 微软雅黑
        "C:\\Windows\\Fonts\\simsun.ttc",               // 宋体
        "C:\\Windows\\Fonts\\simhei.ttf",               // 黑体
        "C:\\Windows\\Fonts\\ARIALUNI.TTF",             // Arial Unicode
    ];

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let font_paths: [&str; 0] = [];

    // 尝试加载字体
    for path in font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            // 设置为首选字体
            fonts.families.entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace)
                .or_default()
                .push("chinese".to_owned());

            tracing::info!("已加载中文字体: {}", path);
            break;
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
    Policies,
    Settings,
    Help,
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
                if ui.selectable_label(self.current_view == View::Help, self.t(TextKey::Help)).clicked() {
                    self.current_view = View::Help;
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
                        ui.label(egui::RichText::new(self.t(TextKey::AdminRequired)).color(egui::Color32::from_rgb(255, 100, 100)));
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
                View::Help => self.show_help(ui),
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
        // 首次加载或数据过期时自动刷新
        let should_refresh = self.stats.is_none()
            || (self.app_config.auto_refresh
                && self.last_refresh.elapsed() > Duration::from_secs(self.app_config.refresh_interval));

        if should_refresh && !self.is_refreshing {
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
                egui::Color32::from_rgb(255, 150, 50) // 橙色，比黄色更显眼
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

                let tw_color = if proc.time_wait > 100 { egui::Color32::from_rgb(255, 150, 50) } else { egui::Color32::WHITE };
                ui.colored_label(tw_color, proc.time_wait.to_string());

                let cw_color = if proc.close_wait > 50 { egui::Color32::RED } else { egui::Color32::WHITE };
                ui.colored_label(cw_color, proc.close_wait.to_string());

                let health_color = if proc.health_score >= 80 {
                    egui::Color32::GREEN
                } else if proc.health_score >= 50 {
                    egui::Color32::from_rgb(255, 150, 50)
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

        // Collect existing policies for checking
        let existing_policies: std::collections::HashSet<String> =
            self.app_config.policy_manager.all_policies().iter()
            .map(|p| p.process_name.clone()).collect();

        // Collect actions to perform after iteration: (process_name, template_name)
        let mut add_policy_for: Option<(String, String)> = None;

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
                ui.label(egui::RichText::new("").strong()); // 策略列
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

                    // 如果已有策略显示"已配置"，否则显示策略模板下拉菜单
                    if existing_policies.contains(&proc.process_name) {
                        ui.label(egui::RichText::new("✓ 已配置").color(egui::Color32::GREEN).small());
                    } else {
                        let proc_name = proc.process_name.clone();
                        egui::ComboBox::from_id_salt(format!("add_policy_{}", proc.pid))
                            .selected_text(t_add)
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(false, "📊 默认策略").clicked() {
                                    add_policy_for = Some((proc_name.clone(), "default".to_string()));
                                }
                                if ui.selectable_label(false, "🚀 高性能").clicked() {
                                    add_policy_for = Some((proc_name.clone(), "high_performance".to_string()));
                                }
                                if ui.selectable_label(false, "🕷️ 采集/爬虫").clicked() {
                                    add_policy_for = Some((proc_name.clone(), "crawler".to_string()));
                                }
                                if ui.selectable_label(false, "🖥️ 服务器").clicked() {
                                    add_policy_for = Some((proc_name.clone(), "server".to_string()));
                                }
                                if ui.selectable_label(false, "🔒 受限").clicked() {
                                    add_policy_for = Some((proc_name.clone(), "restricted".to_string()));
                                }
                            });
                    }
                    ui.end_row();
                }
            });
        });

        // Apply action outside of closure
        if let Some((process_name, template)) = add_policy_for {
            let policy = match template.as_str() {
                "high_performance" => AppPolicy::high_performance(&process_name),
                "crawler" => AppPolicy::crawler(&process_name),
                "server" => AppPolicy::server(&process_name),
                "restricted" => AppPolicy::restricted(&process_name),
                _ => AppPolicy { process_name: process_name.clone(), ..AppPolicy::default() },
            };
            self.app_config.policy_manager.set_policy(policy);
            self.status_message = format!("{}: {} ({})", t_added, process_name, template);
            self.config_dirty = true;
        }
    }

    /// 策略管理视图
    fn show_policies(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t(TextKey::PolicyManagement));
        ui.label(self.t(TextKey::PolicyDescription));
        ui.add_space(10.0);

        let policy_names: Vec<String> = self.app_config.policy_manager.all_policies()
            .iter().map(|p| p.process_name.clone()).collect();

        if policy_names.is_empty() {
            ui.label(self.t(TextKey::NoPolicies));
        } else {
            let t_delete = self.t(TextKey::DeletePolicy);
            let mut policy_to_delete: Option<String> = None;

            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for name in &policy_names {
                    let policy = self.app_config.policy_manager.get_policy(name).clone();

                    egui::CollapsingHeader::new(&policy.process_name)
                        .default_open(true)
                        .show(ui, |ui| {
                            // 自动优化开关
                            ui.horizontal(|ui| {
                                ui.label(self.t(TextKey::AutoOptimize));
                                let mut auto_opt = policy.auto_optimize;
                                if ui.checkbox(&mut auto_opt, "").changed() {
                                    let mut p = policy.clone();
                                    p.auto_optimize = auto_opt;
                                    self.app_config.policy_manager.set_policy(p);
                                    self.config_dirty = true;
                                }
                            });

                            egui::Grid::new(format!("policy_{}", name)).num_columns(3).show(ui, |ui| {
                                // TIME_WAIT阈值
                                ui.label(self.t(TextKey::TimeWaitThreshold));
                                let mut tw = policy.time_wait_threshold.unwrap_or(200) as i32;
                                if ui.add(egui::Slider::new(&mut tw, 50..=1000)).changed() {
                                    let mut p = policy.clone();
                                    p.time_wait_threshold = Some(tw as usize);
                                    self.app_config.policy_manager.set_policy(p);
                                    self.config_dirty = true;
                                }
                                ui.label(egui::RichText::new("推荐: 100-500").small().weak());
                                ui.end_row();

                                // CLOSE_WAIT阈值
                                ui.label(self.t(TextKey::CloseWaitThreshold));
                                let mut cw = policy.close_wait_threshold.unwrap_or(50) as i32;
                                if ui.add(egui::Slider::new(&mut cw, 10..=200)).changed() {
                                    let mut p = policy.clone();
                                    p.close_wait_threshold = Some(cw as usize);
                                    self.app_config.policy_manager.set_policy(p);
                                    self.config_dirty = true;
                                }
                                ui.label(egui::RichText::new("推荐: 20-100").small().weak());
                                ui.end_row();

                                // 最大连接数
                                ui.label(self.t(TextKey::MaxConnections));
                                let mut max = policy.max_connections.unwrap_or(0) as i32;
                                if ui.add(egui::Slider::new(&mut max, 0..=10000).text("0=不限")).changed() {
                                    let mut p = policy.clone();
                                    p.max_connections = if max > 0 { Some(max as usize) } else { None };
                                    self.app_config.policy_manager.set_policy(p);
                                    self.config_dirty = true;
                                }
                                ui.label(egui::RichText::new("推荐: 1000-5000").small().weak());
                                ui.end_row();

                                // 超阈值动作
                                ui.label(self.t(TextKey::ThresholdAction));
                                let current_action = policy.threshold_action;
                                egui::ComboBox::from_id_salt(format!("action_{}", name))
                                    .selected_text(match current_action {
                                        ThresholdAction::Alert => self.t(TextKey::ActionAlert),
                                        ThresholdAction::Optimize => self.t(TextKey::ActionOptimize),
                                        ThresholdAction::RestartProcess => self.t(TextKey::ActionRestart),
                                        ThresholdAction::Ignore => self.t(TextKey::ActionIgnore),
                                    })
                                    .show_ui(ui, |ui| {
                                        for action in [ThresholdAction::Alert, ThresholdAction::Optimize, ThresholdAction::Ignore] {
                                            let label = match action {
                                                ThresholdAction::Alert => self.t(TextKey::ActionAlert),
                                                ThresholdAction::Optimize => self.t(TextKey::ActionOptimize),
                                                ThresholdAction::Ignore => self.t(TextKey::ActionIgnore),
                                                _ => continue,
                                            };
                                            if ui.selectable_label(current_action == action, label).clicked() && current_action != action {
                                                let mut p = policy.clone();
                                                p.threshold_action = action;
                                                self.app_config.policy_manager.set_policy(p);
                                                self.config_dirty = true;
                                            }
                                        }
                                    });
                                ui.label("");
                                ui.end_row();
                            });

                            ui.horizontal(|ui| {
                                if ui.button(t_delete).clicked() {
                                    policy_to_delete = Some(name.clone());
                                }
                            });
                        });
                }
            });

            if let Some(name) = policy_to_delete {
                self.app_config.policy_manager.remove_policy(&name);
                self.status_message = format!("{}: {}", self.t(TextKey::PolicyDeleted), name);
                self.config_dirty = true;
            }
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // 说明文字
        ui.label(egui::RichText::new(self.t(TextKey::PolicyTip)).weak());
    }

    /// 系统设置视图
    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t(TextKey::TcpSettings));

        if !self.is_admin {
            ui.colored_label(
                egui::Color32::from_rgb(255, 100, 100),
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
                ui.colored_label(egui::Color32::from_rgb(255, 150, 50), self.t(TextKey::RebootRequired));
            }
        });
    }

    /// 帮助视图
    fn show_help(&self, ui: &mut egui::Ui) {
        ui.heading(self.t(TextKey::HelpTitle));
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // 关于
            egui::CollapsingHeader::new(egui::RichText::new(self.t(TextKey::HelpAbout)).strong())
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(self.t(TextKey::HelpAboutDesc));
                });
            ui.add_space(10.0);

            // 功能说明
            egui::CollapsingHeader::new(egui::RichText::new(self.t(TextKey::HelpFeatures)).strong())
                .default_open(true)
                .show(ui, |ui| {
                    ui.collapsing(self.t(TextKey::HelpDashboard), |ui| {
                        ui.label(self.t(TextKey::HelpDashboardDesc));
                    });
                    ui.collapsing(self.t(TextKey::HelpProcesses), |ui| {
                        ui.label(self.t(TextKey::HelpProcessesDesc));
                    });
                    ui.collapsing(self.t(TextKey::HelpPolicies), |ui| {
                        ui.label(self.t(TextKey::HelpPoliciesDesc));
                    });
                    ui.collapsing(self.t(TextKey::HelpSettingsHelp), |ui| {
                        ui.label(self.t(TextKey::HelpSettingsDesc));
                    });
                });
            ui.add_space(10.0);

            // TCP 状态说明
            egui::CollapsingHeader::new(egui::RichText::new(self.t(TextKey::HelpTcpStates)).strong())
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(self.t(TextKey::HelpTcpStatesDesc));
                });
            ui.add_space(10.0);

            // 常见问题
            egui::CollapsingHeader::new(egui::RichText::new(self.t(TextKey::HelpTroubleshooting)).strong())
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(self.t(TextKey::HelpTroubleshootingDesc));
                });
            ui.add_space(20.0);

            // 版本信息
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(self.t(TextKey::HelpVersion)).strong());
                ui.label(env!("CARGO_PKG_VERSION"));
            });
        });
    }
}

fn mgr_requires_reboot() -> bool {
    let mgr = create_config_manager();
    mgr.requires_reboot()
}

