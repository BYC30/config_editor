//#![windows_subsystem = "windows"]
#[macro_use]
extern crate lazy_static;

mod data_table;
mod error;
mod utils;
mod app;
mod data_field;
mod saver;
mod syntax_highlight;
mod app_cfg;

use app::SkillEditorApp;

fn main() {
    let mut opt = eframe::NativeOptions::default();
    opt.maximized = true;
    let _ = eframe::run_native("技能编辑器", opt, Box::new(|cc| Box::new(SkillEditorApp::new(cc))));
}