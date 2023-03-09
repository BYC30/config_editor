use eframe::{
    egui::{self, DragValue},
    epaint::Color32,
};
use serde::{Deserialize, Serialize};

use crate::app::theme;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppCfg {
    show: bool,
    show_setting: bool,
    base_theme: i32,
    custom_theme: theme::Theme,
    pub show_undo: bool,
}

impl Default for AppCfg {
    fn default() -> Self {
        Self {
            show: false,
            show_setting: false,
            base_theme: 0,
            custom_theme: theme::MOCHA,
            show_undo: false,
        }
    }
}

impl AppCfg {
    pub fn show(&mut self) {
        self.show = true;
    }

    pub fn color_grid(ui: &mut egui::Ui, title: &str, color: &mut Color32) {
        ui.label(title);
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(color);

            let mut r = color.r();
            let mut g = color.g();
            let mut b = color.b();
            let mut a = color.a();

            ui.add(DragValue::new(&mut r).prefix("r:").clamp_range(0.0..=255.0));
            ui.add(DragValue::new(&mut g).prefix("g:").clamp_range(0.0..=255.0));
            ui.add(DragValue::new(&mut b).prefix("b:").clamp_range(0.0..=255.0));
            ui.add(DragValue::new(&mut a).prefix("a:").clamp_range(0.0..=255.0));

            *color = Color32::from_rgba_premultiplied(r, g, b, a);
        });
        ui.end_row();
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        let old = self.clone();
        egui::Window::new("🔧应用配置")
            .open(&mut self.show)
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("UI设置");
                        if ui.button("打开").clicked() {
                            self.show_setting = true;
                        }
                        ui.end_row();

                        ui.add(egui::Label::new("样式"));
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.base_theme, 0, "Dark");
                            ui.radio_value(&mut self.base_theme, 3, "MOCHA");
                            ui.radio_value(&mut self.base_theme, 2, "MACCHIATO");
                            ui.radio_value(&mut self.base_theme, 1, "FRAPPE");
                            ui.radio_value(&mut self.base_theme, 4, "自定义");
                        });
                        ui.end_row();

                        ui.add(egui::Label::new("自定义"));
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("复制");
                                if ui.button("MOCHA").clicked() {
                                    self.custom_theme = theme::MOCHA.clone();
                                }
                                if ui.button("MACCHIATO").clicked() {
                                    self.custom_theme = theme::MACCHIATO.clone();
                                }
                                if ui.button("FRAPPE").clicked() {
                                    self.custom_theme = theme::FRAPPE.clone();
                                }
                            });

                            egui::CollapsingHeader::new("自定义颜色")
                                .id_source("custom_theme")
                                .show(ui, |ui| {
                                    egui::Grid::new("my_grid")
                                        .num_columns(2)
                                        .spacing([40.0, 4.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            AppCfg::color_grid(
                                                ui,
                                                "背景颜色",
                                                &mut self.custom_theme.base,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "表格间隔颜色",
                                                &mut self.custom_theme.surface0,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "超链接颜色",
                                                &mut self.custom_theme.rosewater,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "错误颜色",
                                                &mut self.custom_theme.maroon,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "警告颜色",
                                                &mut self.custom_theme.peach,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "选中颜色",
                                                &mut self.custom_theme.blue,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "文字颜色",
                                                &mut self.custom_theme.text,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "边框颜色",
                                                &mut self.custom_theme.overlay1,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "控件悬浮颜色",
                                                &mut self.custom_theme.surface2,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "控件激活颜色",
                                                &mut self.custom_theme.surface1,
                                            );
                                            AppCfg::color_grid(
                                                ui,
                                                "输入框背景颜色",
                                                &mut self.custom_theme.crust,
                                            );
                                        });
                                });
                        });
                        ui.end_row();

                        ui.add(egui::Label::new("撤销提示"));
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_undo, "显示");
                        });
                        ui.end_row();
                    });
            });

        egui::Window::new("🔧 Settings")
            .open(&mut self.show_setting)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });
        if old == *self {
            return;
        }
        self.update_cfg(ctx);
    }

    fn update_theme(idx: i32, custom: theme::Theme, ctx: &egui::Context) {
        let theme = match idx {
            0 => {
                let mut visual = egui::Visuals::dark();
                visual.panel_fill = Color32::from_rgb(30, 30, 30);
                visual.faint_bg_color = Color32::from_rgb(40, 40, 40);
                visual.collapsing_header_frame = true;
                visual.slider_trailing_fill = true;
                ctx.set_visuals(visual);
                return;
            }
            1 => theme::FRAPPE,
            2 => theme::MACCHIATO,
            3 => theme::MOCHA,
            4 => custom,
            _ => theme::MOCHA,
        };

        theme::set_theme(&ctx, theme);
    }

    pub fn update_cfg(&self, ctx: &egui::Context) {
        AppCfg::update_theme(self.base_theme, self.custom_theme.clone(), ctx);
    }
}
