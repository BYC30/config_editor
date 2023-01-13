//#![windows_subsystem = "windows"]

mod data_table;
mod error;
mod utils;
mod app;
mod data_field;

use app::SkillEditorApp;

fn main() {
    let mut opt = eframe::NativeOptions::default();
    opt.maximized = true;
    eframe::run_native("技能编辑器", opt, Box::new(|cc| Box::new(SkillEditorApp::new(cc))));
}