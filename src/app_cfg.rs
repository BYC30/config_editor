use eframe::{egui, epaint::Color32};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AppCfg{
    show: bool,
    show_setting: bool,
    base_theme: i32,
}

impl Default for AppCfg {
    fn default() -> Self {
        Self {
            show: false,
            show_setting: false,
            base_theme: 0,
        }
    }
}

impl AppCfg {
    pub fn show(&mut self){
        self.show = true;
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
        if old == *self { return; }
        self.update_cfg(ctx);
    }

    fn update_theme(idx: i32, ctx: &egui::Context) {
        let theme = match idx {
            0 => {
                let mut visual = egui::Visuals::dark();
                visual.panel_fill = Color32::from_rgb(30, 30, 30);
                visual.faint_bg_color = Color32::from_rgb(40, 40, 40);
                visual.collapsing_header_frame = true;
                visual.slider_trailing_fill = true;
                ctx.set_visuals(visual);
                return;
            },
            1 => {catppuccin_egui::FRAPPE},
            2 => {catppuccin_egui::MACCHIATO},
            3 => {catppuccin_egui::MOCHA},
            _ => {catppuccin_egui::MOCHA}
        };

        catppuccin_egui::set_theme(&ctx, theme);
    }

    pub fn update_cfg(&self, ctx: &egui::Context){
        AppCfg::update_theme(self.base_theme, ctx);
    }
}
