pub mod action;
pub mod app_cfg;
pub mod syntax_highlight;
pub mod theme;
// mod convert;

use crate::data::{data_field::FieldInfo, data_table::DataTable};
use crate::{app::app_cfg::AppCfg, error, utils};
use anyhow::{bail, Result};
use eframe::{
    egui::{self, RichText},
    epaint::Color32,
};
use egui_notify::Toasts;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use walkdir::WalkDir;

use self::action::{ActionList, Location};

lazy_static! {
    pub static ref TEMPLETE_MAP_EXPR: Mutex<HashMap<String, TempleteInfo>> =
        Mutex::new(HashMap::new());
    pub static ref TEMPLETE_MAP_SUB_FIELD: Mutex<HashMap<String, TempleteInfo>> =
        Mutex::new(HashMap::new());
}

macro_rules! write_cfg {
    ($this:expr, $filename:expr) => {
        let mut current = std::env::current_exe()?;
        current.pop();
        current.push($filename);
        if !current.exists() {
            std::fs::create_dir_all(current.parent().unwrap())?;
        }
        std::fs::write(&current, include_bytes!(concat!("../../bin/", $filename)))?;
    };
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TabInfo {
    tab: String,
    master_table: String,
}

#[derive(Debug)]
struct TabConfig {
    group: String,
    name: String,
    tabs: Vec<TabInfo>,
}

#[derive(Debug)]
struct LinkInfo {
    table: String,
    field: String,
}

#[derive(Debug, Clone)]
pub struct TempleteInfo {
    pub title: String,
    pub table: String,
    pub content: HashMap<String, String>,
    pub expr: String,
    pub field: Vec<FieldInfo>,
}

#[derive(Debug, Clone)]
struct MenuInfo {
    menu: String,
    name: String,
    exe: String,
    hotkey: Option<egui::Key>,
}

impl MenuInfo {
    fn check_hotkey(&self, ui: &egui::Ui) {
        if !ui.input(|i| i.modifiers.ctrl) {
            return;
        }
        let hk = &self.hotkey;
        if hk.is_none() {
            return;
        }
        let hk = hk.unwrap();
        if !ui.input(|i| i.key_pressed(hk.clone())) {
            return;
        }
        self.trigger();
    }

    fn trigger(&self) {
        let ret = utils::exec_bat(&self.exe);
        match ret {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("执行命令[{}]失败:{}", self.exe, e);
                utils::msg(msg, "错误".to_string());
            }
        }
    }
}

pub struct SkillEditorApp {
    inited: bool,

    tab_cfg: Vec<TabConfig>,
    field_group: HashMap<String, Vec<FieldInfo>>,
    templete: HashMap<String, Vec<TempleteInfo>>,
    menus: Vec<MenuInfo>,
    data_table: HashMap<String, DataTable>,

    data_history: ActionList<HashMap<String, DataTable>, String>,
    location_history: ActionList<Location, ()>,

    // UI 相关数据
    last_location: Location,
    cur_location: Location,
    show_templete: bool,
    templete_target: String,
    templete_table: String,
    templete_data: HashMap<String, String>,
    templete_content: HashMap<String, String>,
    templete_data_idx: i32,
    show_link: bool,
    link_table: String,
    link_src_table: String,
    link_src_field: String,

    console_show: bool,

    cfg: AppCfg,
    toasts: Toasts,

    hotkey_redo: bool,
    hotkey_undo: bool,
}

impl SkillEditorApp {
    fn apply_action(&mut self, action: action::DataAction) {
        self.data_history.apply(action, &mut self.data_table);
        let action = action::MoveLocationAction {
            old_location: self.last_location.clone(),
            new_location: self.cur_location.clone(),
        };

        self.last_location = self.cur_location.clone();
        self.location_history
            .apply(Box::new(action), &mut self.cur_location);
    }

    fn undo(&mut self) {
        let info = self.data_history.undo(&mut self.data_table);
        if self.cfg.show_undo && info.is_some() {
            utils::toast(&mut self.toasts, "SHORT", info.unwrap());
        }
        self.location_history.undo(&mut self.cur_location);
    }

    fn redo(&mut self) {
        let info = self.data_history.redo(&mut self.data_table);
        if self.cfg.show_undo && info.is_some() {
            utils::toast(&mut self.toasts, "SHORT", info.unwrap());
        }
        self.location_history.redo(&mut self.cur_location);
    }
}

impl SkillEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_font".to_owned(),
            egui::FontData::from_static(include_bytes!("../../chinese.simhei.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_font".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("my_font".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        let mut visual = egui::Visuals::dark();
        visual.panel_fill = Color32::from_rgb(30, 30, 30);
        visual.faint_bg_color = Color32::from_rgb(40, 40, 40);
        visual.collapsing_header_frame = true;
        visual.slider_trailing_fill = true;
        cc.egui_ctx.set_visuals(visual);

        utils::hide_console_window();

        let mut ret = Self::default();

        if let Some(storage) = cc.storage {
            if let Some(cfg) = eframe::get_value(storage, eframe::APP_KEY) {
                ret.cfg = cfg;
                ret.cfg.update_cfg(&cc.egui_ctx);
            }
        }
        return ret;
    }

    pub fn save_data(&mut self, force: bool) {
        let mut reload = false;
        for (_, data_table) in &mut self.data_table {
            let result = data_table.save_json(force);
            match result {
                Ok((changed, msg)) => {
                    if changed {
                        utils::toast(
                            &mut self.toasts,
                            "SUCC",
                            format!("[{}]{}", data_table.table_name, msg),
                        );
                        if data_table.reload_editor {
                            reload = true;
                        }
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    utils::toast(
                        &mut self.toasts,
                        "ERRO",
                        format!("[{}]{}", data_table.table_name, msg),
                    );
                }
            };
        }

        if reload {
            self.load_config(true);
            utils::toast(&mut self.toasts, "SUCC", "重载配置成功");
        }
    }

    fn load_menu_config(&mut self) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct MenuConfig {
            menu: String,
            name: String,
            exe: String,
            hotkey: String,
        }

        let data: Vec<MenuConfig> =
            utils::load_dir_excel_cfg("save_data/editor_menu", "editor_menu")?;

        for one in data {
            let hotkey = utils::translate_key(&one.hotkey);

            self.menus.push(MenuInfo {
                menu: one.menu,
                name: one.name,
                exe: one.exe,
                hotkey,
            });
        }

        return Ok(());
    }

    fn load_field_config(&mut self) -> Result<()> {
        #[derive(Serialize, Deserialize, Debug)]
        struct FieldConfig {
            table_key: String,
            name: String,
            val_type: String,
            editor_type: String,
            opt: Vec<String>,
            default: String,
            title: String,
            desc: String,
            group: String,
            link_table: String,
            export: bool,
            output_header: Vec<String>,
        }

        let data: Vec<FieldConfig> =
            utils::load_dir_excel_cfg("save_data/editor_field", "editor_field")?;

        for one in data {
            let field = FieldInfo::parse(
                one.name,
                one.title,
                one.desc,
                one.group,
                one.val_type,
                one.editor_type,
                one.opt,
                one.default,
                one.link_table,
                one.export,
                one.output_header,
            )?;
            if self.field_group.contains_key(&one.table_key) {
                let group = self.field_group.get_mut(&one.table_key).unwrap();
                group.push(field.clone());
            } else {
                self.field_group
                    .insert(one.table_key.clone(), vec![field.clone()]);
            }
        }

        return Ok(());
    }

    fn load_tab_config(&mut self) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TableConfig {
            table_key: String,
            show_name: String,
            show_field: String,
            master_field: String,
            group_field: String,
            export_sort: String,
            output_type: Vec<String>,
            output_path: Vec<String>,

            #[serde(default)]
            post_exec: String,
            #[serde(default)]
            reload_editor: bool,
        }

        #[derive(Serialize, Deserialize)]
        struct TabCfg {
            group: String,
            title: String,
            tabs: Vec<TabInfo>,
        }

        let data: Vec<TabCfg> = utils::load_dir_excel_cfg("save_data/editor_tab", "editor_tab")?;

        let mut first = true;
        for one in data {
            if first {
                first = false;
                self.cur_location.cur_view_group = one.group.clone();
                self.last_location.cur_view_group = one.group.clone();
            }
            self.tab_cfg.push(TabConfig {
                group: one.group,
                name: one.title,
                tabs: one.tabs,
            });
        }

        let data: Vec<TableConfig> =
            utils::load_dir_excel_cfg("save_data/editor_table", "editor_table")?;

        for one in data {
            let mut info = Vec::new();
            if self.field_group.contains_key(&one.table_key) {
                let group_field = FieldInfo::parse(
                    "__Group__".to_string(),
                    "分组".to_string(),
                    "编辑器分组".to_string(),
                    "分组".to_string(),
                    "S".to_string(),
                    "Text".to_string(),
                    Vec::new(),
                    "默认分组".to_string(),
                    String::new(),
                    false,
                    Vec::new(),
                )?;
                let sub_group_field = FieldInfo::parse(
                    "__SubGroup__".to_string(),
                    "子分组".to_string(),
                    "编辑器子分组".to_string(),
                    "分组".to_string(),
                    "S".to_string(),
                    "Text".to_string(),
                    Vec::new(),
                    "默认子分组".to_string(),
                    String::new(),
                    false,
                    Vec::new(),
                )?;
                let field = self.field_group.get_mut(&one.table_key).unwrap();
                field.insert(0, sub_group_field);
                field.insert(0, group_field);
                for one in field {
                    info.push(one.clone());
                }
            }
            let mut templete = Vec::new();
            if self.templete.contains_key(&one.table_key) {
                let t = self.templete.get(&one.table_key).unwrap();
                for one in t {
                    templete.push(one.clone());
                }
            }
            let mut data_table = DataTable::new(
                one.table_key.clone(),
                one.show_name,
                one.show_field,
                one.master_field,
                one.group_field,
                one.export_sort,
                one.output_type,
                one.output_path,
                info,
                templete,
                one.post_exec,
            );
            data_table.reload_editor = one.reload_editor;
            self.data_table.insert(one.table_key.clone(), data_table);
        }

        return Ok(());
    }

    fn load_data(&mut self) -> Result<()> {
        for (_k, v) in &mut self.data_table {
            v.load_data();
        }
        return Ok(());
    }

    fn load_templete(&mut self) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TempleteConfig {
            table_key: String,
            title: String,
            table: String,
            content: HashMap<String, String>,
            expr: String,
            templete_type: String,
        }

        let mut templete_map = TEMPLETE_MAP_EXPR.lock().unwrap();
        templete_map.clear();
        let mut templete_sub_field_map = TEMPLETE_MAP_SUB_FIELD.lock().unwrap();
        templete_sub_field_map.clear();

        let data: Vec<TempleteConfig> =
            utils::load_dir_excel_cfg("save_data/editor_templete", "editor_templete")?;

        for one in data {
            if !self.templete.contains_key(&one.table_key) {
                self.templete.insert(one.table_key.clone(), Vec::new());
            }
            let list = self.templete.get_mut(&one.table_key).unwrap();

            println!("LoadTemplete[{}] type[{}]", one.table, one.templete_type);
            if self.field_group.contains_key(&one.table) {
                let field = self.field_group.get(&one.table).unwrap();
                let field = field.clone();
                let info = TempleteInfo {
                    title: one.title,
                    table: one.table,
                    content: one.content,
                    expr: one.expr,
                    field,
                };

                if one.templete_type == "Expr" {
                    templete_map.insert(info.table.clone(), info.clone());
                }
                if one.templete_type == "SubField" {
                    templete_sub_field_map.insert(info.table.clone(), info.clone());
                    println!("LoadSubField templete key[{}] v[{:?}]", info.table, info);
                }

                list.push(info.clone());
            } else {
                println!("模板[{}]的字段配置[{}]未找到", one.title, one.table);
            }
        }

        return Ok(());
    }

    pub fn create_default_config(&self) -> Result<()> {
        write_cfg!(self, "save_data/editor_field/编辑器_编辑器配置.xlsx");
        write_cfg!(self, "save_data/editor_tab/编辑器_编辑器配置.xlsx");
        write_cfg!(self, "save_data/editor_table/编辑器_编辑器配置.xlsx");
        write_cfg!(self, "save_data/editor_templete/编辑器_编辑器配置.xlsx");
        write_cfg!(self, "save_data/editor_templete/表达式模板_客户端.xlsx");
        Ok(())
    }

    pub fn _load_config(&mut self) -> Result<()> {
        self.field_group.clear();
        self.tab_cfg.clear();
        self.data_table.clear();
        self.templete.clear();
        self.menus.clear();

        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("save_data");
        if !path.exists() {
            self.create_default_config()?;
        }

        println!("读取字段配置");
        self.load_field_config()?;
        println!("读取模板配置");
        self.load_templete()?;
        println!("读取页签配置");
        self.load_tab_config()?;
        println!("读取菜单配置");
        self.load_menu_config()?;
        println!("读取数据");
        self.load_data()?;
        return Ok(());
    }

    pub fn load_config(&mut self, force: bool) {
        if self.inited && !force {
            return;
        }
        self.inited = false;
        let ret = self._load_config();
        self.inited = true;
        match ret {
            Ok(_) => {}
            Err(e) => utils::msg(format!("读取配置失败:{:?}", e), "错误".to_string()),
        }
    }
}

// UI 相关接口
impl SkillEditorApp {
    fn draw_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("💾保存(S)").clicked() {
                    self.save_data(true);
                }
                if ui.button("🔃重新载入").clicked() {
                    self.load_config(true);
                }
                if ui.button("↩撤销(Z)").clicked() || self.hotkey_undo {
                    self.undo();
                }
                if ui.button("↪重做(Y)").clicked() || self.hotkey_redo {
                    self.redo();
                }
                if ui.button("🔧应用配置").clicked() {
                    self.cfg.show();
                }
                if ui.button("🖥控制台").clicked() {
                    if self.console_show {
                        utils::hide_console_window();
                    } else {
                        utils::show_console_window();
                    }
                    self.console_show = !self.console_show;
                }
                if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl) {
                    // if ui.input().key_pressed(egui::Key::S) && ui.input().modifiers.ctrl {
                    self.save_data(ui.input(|i| i.modifiers.shift));
                }
                let mut list: Vec<(String, Vec<MenuInfo>)> = Vec::new();
                for one in &self.menus {
                    one.check_hotkey(ui);
                    let mut found = false;
                    for (menu, v) in &mut list {
                        if *menu == one.menu {
                            v.push(one.clone());
                            found = true;
                            break;
                        }
                    }
                    if found {
                        continue;
                    }
                    list.push((one.menu.clone(), vec![one.clone()]));
                }
                for (menu, v) in list {
                    if menu.is_empty() {
                        for menu_info in v {
                            if ui.button(&menu_info.name).clicked() {
                                menu_info.trigger();
                            }
                        }
                    } else {
                        ui.menu_button(menu, |ui| {
                            for menu_info in v {
                                if ui.button(&menu_info.name).clicked() {
                                    menu_info.trigger();
                                }
                            }
                        });
                    }
                }
            });
        });
        egui::TopBottomPanel::top("tables").show(ctx, |ui| {
            let mut idx = 0;

            let mut group_list: Vec<(String, Vec<(usize, String)>)> = Vec::new();
            for one in &self.tab_cfg {
                let mut found = false;
                for (group, list) in &mut group_list {
                    if one.group == *group {
                        list.push((idx, one.name.clone()));
                        found = true;
                        break;
                    }
                }
                if !found {
                    group_list.push((one.group.clone(), vec![(idx, one.name.clone())]));
                }
                idx = idx + 1;
            }
            let mut cur_group_list = Vec::new();
            egui::menu::bar(ui, |ui| {
                ui.label("页签分组:");
                for (group, list) in group_list {
                    if group == self.cur_location.cur_view_group {
                        cur_group_list = list;
                    }
                    if ui
                        .selectable_label(group == self.cur_location.cur_view_group, &group)
                        .clicked()
                    {
                        self.cur_location.cur_view_group = group;
                    }
                }
            });

            egui::menu::bar(ui, |ui| {
                ui.label("页签列表:");
                for (idx, name) in cur_group_list {
                    if ui
                        .selectable_label(idx == self.cur_location.cur_view, &name)
                        .clicked()
                    {
                        self.cur_location.cur_view = idx;
                    }
                }
            });
        });
    }

    fn get_list_next_idx(
        list: &HashMap<String, HashMap<String, Vec<(String, i32, i32, bool)>>>,
        cur: i32,
    ) -> i32 {
        let mut next = -1;
        let mut found = false;
        for (_, v) in list {
            for (_, vv) in v {
                let mut vec_idx: i32 = 0;
                for (_, idx, _, _) in vv {
                    if *idx == cur {
                        found = true;
                        break;
                    }
                    vec_idx = vec_idx + 1;
                }
                if found {
                    let len = vv.len() as i32;
                    let diff = if vec_idx + 1 >= len { -1 } else { 1 };
                    vec_idx = vec_idx + diff;
                    if vec_idx >= 0 && vec_idx < len {
                        next = vv.get(vec_idx as usize).unwrap().1;
                    }
                    break;
                }
            }
        }
        if next > cur {
            next = next - 1;
        } // idx 比当前大, 减一
        return next;
    }

    fn draw_view(&mut self, ctx: &egui::Context) {
        let cfg = self.tab_cfg.get(self.cur_location.cur_view as usize);
        if cfg.is_none() {
            return;
        }
        let cfg = cfg.unwrap();
        let size = ctx.available_rect().max;
        let unit = cfg.tabs.len() as f32;
        let width = (size.x - unit * 8.0 * 4.0) / unit; // 一个配置包含两个面板, 4条边

        let mut idx = 0;
        let mut click_table = String::new();
        let mut ops: Vec<Option<action::DataAction>> = Vec::new();
        // let mut ops = Vec::new();
        for tab_info in &cfg.tabs {
            idx = idx + 1;
            let show_table = self.data_table.get(&tab_info.tab);
            let err_msg = if show_table.is_none() {
                format!("表格[{}]未找到", tab_info.tab)
            } else {
                let const_one = show_table.unwrap();
                const_one.error.clone()
            };
            if !err_msg.is_empty() {
                SkillEditorApp::draw_empty_table(ctx, err_msg, width, idx);
                continue;
            }

            let mut cur_master_val = String::new();
            let master_table = tab_info.master_table.clone();
            if !master_table.is_empty() {
                let master_table = self.data_table.get(&master_table);
                if master_table.is_some() {
                    let master_table = master_table.unwrap();
                    cur_master_val = master_table.get_cur_key();
                }
            }

            let data_table = self.data_table.get_mut(&tab_info.tab).unwrap();

            let mut show_all = None;
            let mut show_all_bool = false;
            if !data_table.master_field.is_empty() {
                show_all = Some(data_table.show_all);
                show_all_bool = data_table.show_all;
            }
            if !click_table.is_empty() && click_table == tab_info.master_table {
                data_table.update_cur_row(&cur_master_val);
            }
            let list = data_table.get_show_name_list(
                &data_table.master_field,
                &cur_master_val,
                show_all_bool,
                &data_table.search,
            );
            let (click, op, create_tmp) = SkillEditorApp::draw_list(
                ctx,
                idx,
                width * 0.35,
                &data_table.show_name,
                &list,
                data_table.cur_row,
                &mut data_table.search,
                &mut show_all,
                &data_table.templete,
                &mut data_table.templete_idx,
            );
            if show_all.is_some() {
                data_table.show_all = show_all.unwrap();
            }
            if click.is_some() {
                data_table.cur_row = click.unwrap().clone();
                click_table = data_table.table_name.clone();
            }
            let mut changed = HashMap::new();
            let link_info =
                SkillEditorApp::draw_data(ctx, idx, data_table, width * (1.0 - 0.35), &mut changed);

            if link_info.is_some() {
                let link_info = link_info.unwrap();
                self.link_table = link_info.table;
                self.link_src_table = tab_info.tab.clone();
                self.link_src_field = link_info.field;
                self.show_link = true;
                println!(
                    "ShowLinkWindow src[{}] field[{}] link[{}]",
                    self.link_src_table, self.link_src_field, self.link_table
                );
            }

            let data_table = self.data_table.get(&tab_info.tab).unwrap();

            for (k, v) in changed {
                let name = data_table.table_name.clone();
                let row_idx = data_table.cur_row.clone() as usize;
                ops.push(action::UpdateAction::new(
                    &self.data_table,
                    &name,
                    row_idx,
                    &k,
                    &v,
                ));
            }
            if !create_tmp.is_empty() {
                let field_info = self.field_group.get(&create_tmp);
                if field_info.is_some() {
                    let field_info = field_info.unwrap();
                    self.templete_target = tab_info.tab.clone();
                    self.templete_table = create_tmp;
                    self.templete_data = HashMap::new();
                    for field in field_info {
                        self.templete_data
                            .insert(field.name.clone(), field.default_val.clone());
                    }
                    let t = data_table
                        .templete
                        .get(data_table.templete_idx as usize)
                        .unwrap();
                    self.templete_content = t.content.clone();
                    self.show_templete = true;
                }
            }

            if op == 1 {
                let data = data_table.create_row(&cur_master_val, 0);
                ops.push(action::AddAction::new(
                    &self.data_table,
                    &data_table.table_name,
                    data,
                ));
            }

            if op == 2 {
                let next = SkillEditorApp::get_list_next_idx(&list, data_table.cur_row);
                let table_name = data_table.table_name.clone();
                let cur_row = data_table.cur_row as usize;
                ops.push(action::DelAction::new(
                    &self.data_table,
                    &table_name,
                    cur_row,
                    next as usize,
                ));
            }
            if op == 3 {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("xlsm", &["xlsm", "xlsx"])
                    .pick_file()
                {
                    ops.push(action::ImportAction::new(
                        &self.data_table,
                        path,
                        data_table.table_name.clone(),
                    ));
                }
            }

            if op == 4 {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("xlsx", &["xlsx"])
                    .save_file()
                {
                    let table = data_table.table_name.clone();
                    let data_table = self.data_table.get(&table).unwrap();
                    match data_table.export_excel(path, table.clone()) {
                        Ok(_) => {
                            utils::toast(&mut self.toasts, "SUCC", format!("导出[{}]成功", table));
                        }
                        Err(e) => {
                            let msg = format!("导出失败: {:?}", e);
                            utils::toast(&mut self.toasts, "ERRO", msg);
                        }
                    }
                }
            }

            if op == 5 {
                let copy_master_val = data_table.get_cur_key();
                let data = data_table.copy_cur_row(&cur_master_val).unwrap();
                let mut copy_table = Vec::new();
                for one in &cfg.tabs {
                    if one.master_table == tab_info.tab {
                        copy_table.push(one.tab.clone());
                    }
                }
                ops.push(action::CopyAction::new(
                    &self.data_table,
                    &data_table.table_name,
                    data,
                    copy_table,
                    copy_master_val,
                ));
            }
        }

        for op in ops {
            if op.is_some() {
                self.apply_action(op.unwrap());
            }
        }
    }

    fn draw_list(
        ctx: &egui::Context,
        idx: i32,
        width: f32,
        title: &str,
        list: &HashMap<String, HashMap<String, Vec<(String, i32, i32, bool)>>>,
        cur: i32,
        search: &mut String,
        show_all: &mut Option<bool>,
        templete: &Vec<TempleteInfo>,
        tmp_idx: &mut i32,
    ) -> (Option<i32>, i32, String) {
        let mut ret = None;
        let mut op = 0;
        let id = format!("list_panel_{}", idx);
        let mut all = false;
        let mut create_templete = String::new();
        egui::SidePanel::left(id).resizable(false).show(ctx, |ui| {
            ui.set_width(width);

            ui.horizontal(|ui| {
                ui.heading(title);
            });
            ui.horizontal(|ui| {
                if ui.button("➕").on_hover_text("新增配置").clicked() {
                    op = 1;
                }
                if ui.button("❌").on_hover_text("删除配置").clicked() {
                    op = 2;
                }
                if ui.button("📋").on_hover_text("复制配置").clicked() {
                    op = 5;
                }
                if ui.button("📥").on_hover_text("导入配置").clicked() {
                    op = 3;
                }
                if ui.button("📤").on_hover_text("导出配置").clicked() {
                    op = 4;
                }
                if show_all.is_some() {
                    all = show_all.unwrap();
                    ui.checkbox(&mut all, "").on_hover_text("显示全部");
                    *show_all = Some(all);
                }
            });
            if templete.len() > 0 {
                ui.horizontal(|ui| {
                    let id = format!("{}_templete", idx);

                    let mut templete_name = String::new();
                    let mut templete_table = String::new();
                    let cur_templete = templete.get(*tmp_idx as usize);
                    if cur_templete.is_some() {
                        let cur = cur_templete.unwrap();
                        templete_name = cur.title.clone();
                        templete_table = cur.table.clone();
                    }

                    egui::ComboBox::from_id_source(id)
                        .selected_text(templete_name)
                        .show_ui(ui, |ui| {
                            let mut idx = 0;
                            for one in templete {
                                ui.selectable_value(tmp_idx, idx, one.title.clone());
                                idx = idx + 1;
                            }
                        });

                    if ui.button("创建模板").clicked() {
                        create_templete = templete_table;
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(search);
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (group, one) in list.iter().sorted_by_key(|a| a.0) {
                        egui::CollapsingHeader::new(group)
                            .default_open(true)
                            .show(ui, |ui| {
                                for (sub_group, two) in one.iter().sorted_by_key(|a| a.0) {
                                    egui::CollapsingHeader::new(sub_group)
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            for (name, idx, _key_num, dup) in two {
                                                let mut txt = RichText::new(name);
                                                if *dup {
                                                    txt = txt.color(Color32::RED);
                                                }
                                                if ui.selectable_label(*idx == cur, txt).clicked() {
                                                    ret = Some(idx.clone());
                                                }
                                            }
                                        });
                                }
                            });
                    }
                });
        });

        return (ret, op, create_templete);
    }

    fn draw_data(
        ctx: &egui::Context,
        idx: i32,
        data_table: &mut DataTable,
        width: f32,
        changed: &mut HashMap<String, String>,
    ) -> Option<LinkInfo> {
        let map = data_table.data.get_mut(data_table.cur_row as usize);
        let id1 = format!("detail_panel_{}", idx);
        let ret = egui::SidePanel::left(id1).resizable(false).show(ctx, |ui| {
            ui.set_width(width);
            if map.is_none() {
                return None;
            } else {
                let mut map = map.unwrap();
                ui.horizontal(|ui| {
                    let txt1 = egui::TextEdit::singleline(&mut data_table.detail_search)
                        .desired_width(f32::INFINITY);
                    ui.add(txt1);
                });

                let click = SkillEditorApp::_draw_data(
                    ui,
                    idx.to_string(),
                    &data_table.info,
                    &mut map,
                    data_table.cur,
                    &data_table.detail_search,
                    Some(changed),
                );
                if click.is_none() {
                    return None;
                }
                let idx = click.unwrap();
                data_table.cur = idx;
                let click_field = data_table.info.get(idx as usize);
                if click_field.is_none() {
                    return None;
                }
                let click_field = click_field.unwrap();
                if click_field.link_table.is_empty() {
                    return None;
                }
                return Some(LinkInfo {
                    table: click_field.link_table.clone(),
                    field: click_field.name.clone(),
                });
            }
        });
        return ret.inner;
    }

    pub fn _draw_data(
        ui: &mut egui::Ui,
        idx: String,
        field: &Vec<FieldInfo>,
        map: &mut HashMap<String, String>,
        select_field: i32,
        search: &String,
        changed: Option<&mut HashMap<String, String>>,
    ) -> Option<i32> {
        let id2 = format!("detail_desc_panel_{}", idx);
        let mut ret = None;
        let select = field.get(select_field as usize);

        if select.is_some() {
            egui::TopBottomPanel::bottom(id2)
                .resizable(false)
                .show_inside(ui, |ui| {
                    let select = select.unwrap();
                    let mut desc = select.desc.as_str();
                    let txt1 = egui::TextEdit::multiline(&mut desc).desired_width(f32::INFINITY);
                    ui.add(txt1);
                });
        }

        let mut draw_info: Vec<(String, Vec<(i32, FieldInfo)>)> = Vec::new();
        let mut idx = 0;
        for one in field {
            idx = idx + 1;
            let group = one.group.clone();
            let mut found = false;
            for (k, v) in &mut draw_info {
                if *k == group {
                    v.push((idx, one.clone()));
                    found = true;
                    break;
                }
            }
            if !found {
                draw_info.push((group, vec![(idx, one.clone())]));
            }
        }

        let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
        let size = ui.available_size();

        let mut changed_map = HashMap::new();
        scroll.show(ui, |ui| {
            let mut click_flag = false;
            let mut click_idx = 0;
            for (k, vec) in draw_info {
                egui::CollapsingHeader::new(k)
                    .default_open(true)
                    .show(ui, |ui| {
                        let grid_id = format!("detail_panel_grid_{}", idx);
                        let grid = egui::Grid::new(grid_id)
                            .num_columns(2)
                            .spacing([4.0, 4.0])
                            .min_col_width(size.x / 5.0)
                            .striped(true);
                        grid.show(ui, |ui| {
                            for (idx, one) in vec {
                                let val = map.get(&one.name);
                                let old = match val {
                                    Some(s) => s.clone(),
                                    None => String::new(),
                                };
                                let mut new = old.clone();

                                let f =
                                    one.create_ui(&mut new, ui, select_field == idx - 1, search, 0);
                                if f {
                                    click_flag = true;
                                    click_idx = idx - 1;
                                }
                                if old != new {
                                    if changed.is_some() {
                                        changed_map.insert(one.name.clone(), new.clone());
                                    } else {
                                        map.insert(one.name.clone(), new);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    });
            }
            if click_flag {
                ret = Some(click_idx);
            }

            if changed.is_some() {
                let ret_map = changed.unwrap();
                for (k, v) in changed_map {
                    ret_map.insert(k, v);
                }
            }
        });
        return ret;
    }

    fn draw_empty_table(ctx: &egui::Context, msg: String, width: f32, idx: i32) {
        let list_panel_id = format!("list_panel_{}", idx);
        egui::SidePanel::left(list_panel_id)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width);
                let err_info = egui::RichText::new(msg).color(Color32::RED);
                ui.label(err_info);
            });
        let list_panel_id = format!("detail_panel_{}", idx);
        egui::SidePanel::left(list_panel_id)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width);
            });
    }

    fn _draw_link_window(
        ctx: &egui::Context,
        title: String,
        open: bool,
        field: &Vec<FieldInfo>,
        map: &mut HashMap<String, String>,
        select_field: i32,
    ) -> (bool, Option<i32>) {
        let mut state = open;
        let mut click = None;
        egui::Window::new("关联表")
            .open(&mut state)
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading(title);
                click = SkillEditorApp::_draw_data(
                    ui,
                    "LinkWindow".to_string(),
                    field,
                    map,
                    select_field,
                    &String::new(),
                    None,
                );
            });
        return (state, click);
    }

    fn _draw_link_window_ret(&mut self, ctx: &egui::Context) -> Result<()> {
        if !self.show_link {
            return Ok(());
        }
        let src_table = self.data_table.get(&self.link_src_table);
        if src_table.is_none() {
            bail!(error::AppError::HintMsg(format!(
                "原始表[{}]未找到",
                self.link_src_table
            )));
        }
        let src_table = src_table.unwrap();
        let link_val = src_table.get_field_val(&self.link_src_field);
        let data_table = self.data_table.get_mut(&self.link_table);
        if data_table.is_none() {
            bail!(error::AppError::HintMsg(format!(
                "数据表[{}]未找到",
                self.link_table
            )));
        }
        let data_table = data_table.unwrap();
        let mut link_idx: i32 = -1;
        let mut idx = 0;
        for row in &data_table.data {
            let key = utils::map_get_string(row, &data_table.key_name, "");
            if key == link_val {
                link_idx = idx;
                break;
            }
            idx = idx + 1;
        }
        let map = data_table.data.get_mut(link_idx as usize);
        if map.is_none() {
            bail!(error::AppError::HintMsg(format!(
                "数据表[{}]主键[{}]未找到",
                self.link_table, link_val
            )));
        }
        let map = map.unwrap();
        let key = utils::map_get_string(map, &data_table.key_name, "");
        let show = utils::map_get_string(map, &data_table.show_field, "");
        let title = format!("关联表:{} - [{}]{}", data_table.show_name, key, show);
        let (show, click) = SkillEditorApp::_draw_link_window(
            ctx,
            title,
            self.show_link,
            &data_table.info,
            map,
            data_table.cur,
        );
        self.show_link = show;
        if click.is_some() {
            data_table.cur = click.unwrap();
        }
        return Ok(());
    }

    fn draw_link_window(&mut self, ctx: &egui::Context) {
        let ret = self._draw_link_window_ret(ctx);
        match ret {
            Ok(_) => {}
            Err(e) => {
                let cur_table = self.data_table.get(&self.link_src_table);
                if cur_table.is_none() {
                    return;
                }
                let link_val = cur_table.unwrap().get_field_val(&self.link_src_field);
                let data_table = self.data_table.get_mut(&self.link_table);
                let show_button = data_table.is_some();
                let (show, new) = SkillEditorApp::draw_empty_link_window(
                    ctx,
                    e.to_string(),
                    self.show_link,
                    show_button,
                );
                self.show_link = show;
                if !new {
                    return;
                }
                if data_table.is_none() {
                    return;
                }
                let data_table = data_table.unwrap();
                let mut data = data_table.create_row(&String::new(), 0);
                data.insert(data_table.key_name.clone(), link_val);
                let name = data_table.table_name.clone();
                let action = action::AddAction::new(&self.data_table, &name, data);
                if action.is_some() {
                    self.apply_action(action.unwrap());
                }
            }
        }
    }

    fn draw_empty_link_window(
        ctx: &egui::Context,
        msg: String,
        open: bool,
        show_button: bool,
    ) -> (bool, bool) {
        let mut state = open;
        let mut new = false;
        egui::Window::new("关联表")
            .open(&mut state)
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                let err_info = egui::RichText::new(msg).color(Color32::RED);
                ui.label(err_info);
                if !show_button {
                    return;
                }
                new = ui.button("新建").clicked();
            });
        return (state, new);
    }

    fn draw_templete(&mut self, ctx: &egui::Context) {
        let mut create = false;
        egui::Window::new("关联表")
            .open(&mut self.show_templete)
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                let field = self.field_group.get(&self.templete_table);
                if field.is_some() {
                    create = ui.button("创建").clicked();

                    let field = field.unwrap();
                    let click = SkillEditorApp::_draw_data(
                        ui,
                        "TempleteWindow".to_string(),
                        field,
                        &mut self.templete_data,
                        self.templete_data_idx,
                        &String::new(),
                        None,
                    );

                    if click.is_some() {
                        self.templete_data_idx = click.unwrap();
                    }
                } else {
                    let err_info =
                        egui::RichText::new(format!("模板[{}]字段配置未找到", self.templete_table))
                            .color(Color32::RED);
                    ui.label(err_info);
                }
            });
        if create {
            let cur_master_val = String::new();
            let data_table = self.data_table.get_mut(&self.templete_target).unwrap();
            let mut data = data_table.create_row(&cur_master_val, 0);
            for (k, v) in &self.templete_content {
                let mut val = v.clone();
                for (kk, vv) in &self.templete_data {
                    let templete_key = format!("%{}%", kk);
                    val = val.replace(templete_key.as_str(), vv.as_str());
                }
                data.insert(k.clone(), val);
            }
            let name = data_table.table_name.clone();
            let action = action::AddAction::new(&self.data_table, &name, data);
            if action.is_some() {
                self.apply_action(action.unwrap());
                self.show_templete = false;
            }
        }
    }
}

impl Default for SkillEditorApp {
    fn default() -> Self {
        Self {
            inited: false,
            field_group: HashMap::new(),
            tab_cfg: Vec::new(),
            data_table: HashMap::new(),
            data_history: action::ActionList::new(),
            location_history: action::ActionList::new(),
            last_location: Location::default(),
            cur_location: Location::default(),
            show_templete: false,
            templete: HashMap::new(),
            templete_target: String::new(),
            templete_table: String::new(),
            templete_data: HashMap::new(),
            templete_content: HashMap::new(),
            templete_data_idx: 0,
            show_link: false,
            link_table: String::new(),
            link_src_table: String::new(),
            link_src_field: String::new(),
            menus: Vec::new(),
            console_show: false,
            cfg: AppCfg::default(),
            toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            hotkey_redo: false,
            hotkey_undo: false,
        }
    }
}

fn match_hotkey_event(event: &egui::Event) -> (bool, i32) {
    match event {
        egui::Event::Key {
            key,
            pressed,
            repeat,
            modifiers,
        } => {
            if !*pressed {
                return (false, 0);
            }
            if *repeat {
                return (false, 0);
            }
            if modifiers.alt || modifiers.shift {
                return (false, 0);
            }
            if *key != egui::Key::Z && *key != egui::Key::Y {
                return (false, 0);
            }
            let ret = match *key {
                egui::Key::Z => 1,
                egui::Key::Y => 2,
                _ => 0,
            };
            return (true, ret);
        }
        _ => {
            return (false, 0);
        }
    }
}

fn disable_default_hotkey(ctx: &egui::Context) -> (bool, bool) {
    let mut redo = false;
    let mut undo = false;
    ctx.input_mut(|i| {
        let mut idx = 0;
        let mut remove_vec = Vec::new();
        for one in &i.events {
            let (remove, ret) = match_hotkey_event(one);
            if remove {
                if ret == 1 {
                    undo = true;
                }
                if ret == 2 {
                    redo = true;
                }
                remove_vec.push(idx);
            } else {
                idx = idx + 1;
            }
        }
        for one in remove_vec {
            i.events.remove(one);
        }
    });
    return (undo, redo);
}

impl eframe::App for SkillEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        (self.hotkey_undo, self.hotkey_redo) = disable_default_hotkey(ctx);

        self.load_config(false);
        self.draw_menu(ctx);
        self.draw_view(ctx);
        self.draw_link_window(ctx);
        self.draw_templete(ctx);
        self.cfg.ui(ctx);

        self.toasts.show(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.cfg);
    }
}
