use std::{collections::{HashMap, HashSet}, fmt::format, process::Command, arch::x86_64::_MM_FROUND_CUR_DIRECTION, path::PathBuf};
use eframe::{egui::{self, Ui, RichText, output}, App, epaint::Color32, glow::GEOMETRY_OUTPUT_TYPE};
use anyhow::{Result, bail};
use itertools::Itertools;
use crate::{utils, error, data_table::{DataTable, self}, data_field::FieldInfo};

#[derive(Debug)]
struct TabConfig {
    name: String,
    tabs: Vec<String>,
}

#[derive(Debug)]
struct LinkInfo{
    table: String,
    field: String,
}

#[derive(Debug, Clone)]
pub struct TempleteInfo{
    title: String,
    table: String,
    content: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct MenuInfo{
    menu: String,
    name: String,
    exe: String,
    hotkey: Option<egui::Key>,
}

impl MenuInfo {
    fn check_hotkey(&self, ui: &egui::Ui) {
        if !ui.input().modifiers.ctrl {return;}
        let hk = &self.hotkey;
        if hk.is_none() {return;}
        let hk = hk.unwrap();
        if !ui.input().key_pressed(hk.clone()) {return;}
        self.trigger();
    }

    fn _trigger(&self) -> Result<()> {
        
        let path = std::env::current_exe()?;
        let path = dunce::canonicalize(path)?;

        let mut exe_path = path.clone();
        exe_path.pop();
        exe_path.push(self.exe.clone());
        println!("exe_path {:?}", exe_path);
        let exe_path = dunce::canonicalize(exe_path)?;
        let exe_path_str = exe_path.to_str().unwrap().replace("\\", "\\\\");
        let mut exe_dir = exe_path.clone();
        exe_dir.pop();
        let exe_dir_str = exe_dir.to_str().unwrap().replace("\\", "\\\\");

        println!("dir[{}] exe[{}]", exe_dir_str, exe_path_str);

        let cur_dir = std::env::current_dir()?;
        std::env::set_current_dir(exe_dir.clone())?;
        let lua = mlua::Lua::new();
        let cmd = format!("local f = io.popen('cmd /C \"{}\"') f:close()", exe_path_str);
        println!("Exec cmd {}", cmd);
        lua.load(&cmd).exec()?;
        std::env::set_current_dir(cur_dir)?;

        return Ok(());
    }

    fn trigger(&self){
        let ret = self._trigger();
        match ret {
            Ok(_) => {},
            Err(e) => {
                let msg = format!("执行命令[{}]失败:{}", self.exe, e);
                utils::msg(msg, "错误".to_string());
            }
        }
    }

}

#[derive(Debug)]
pub struct SkillEditorApp {
    inited: bool,

    field_group: HashMap<String, Vec<FieldInfo>>,
    tab_cfg: Vec<TabConfig>,
    data_table: HashMap<String, DataTable>,
    templete: HashMap<String, Vec<TempleteInfo>>,
    menus: Vec<MenuInfo>,

    // UI 相关数据
    cur_view: usize,
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
}

impl SkillEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_font".to_owned(),
            egui::FontData::from_static(include_bytes!("../chinese.simhei.ttf")),
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
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self::default()
    }

    pub fn _save_data(&self) -> Result<()> {
        let mut path = std::env::current_exe()?;
        path.pop();

        for group in &self.tab_cfg {
            for table_key in &group.tabs {
                let data_table = self.data_table.get(table_key);
                if data_table.is_none() {continue;}
                let data_table = data_table.unwrap();

                let mut idx = 0;
                for output_type in &data_table.output_type {
                    if data_table.output_path.len() > idx {
                        let mut p = path.clone();
                        let path = data_table.output_path.get(idx).unwrap();
                        p.push(path.clone());
                        if !path.ends_with(".csv") {
                            let f = format!("{}.csv", table_key);
                            p.push(f);
                        }
                        data_table._save_csv(p, output_type)?;
                    }
                    idx = idx + 1;
                }
                let p = data_table.get_save_json()?;
                data_table._save_json(p)?;
            }
        }

        Ok(())
    }

    pub fn save_data(&mut self){
        let ret = self._save_data();
        match ret {
            Ok(_) => {self.msg("导出成功".to_string(), "导出成功".to_string())},
            Err(e) => {self.msg(format!("导出失败:{:?}", e), "导出失败".to_string())},
        }
    }

    fn load_menu_config(&mut self) -> Result<()> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("config.xlsx");
        let range = utils::open_excel2(&path, "菜单配置")?;

        let mut idx = 0;
        for row in range.rows() {
            idx = idx + 1;
            if idx <= 1 {continue;}
            let menu = row[0].to_string();
            let name = row[1].to_string();
            if name.is_empty() {continue;}
            let exe = row[2].to_string();
            let hotkey = row[3].to_string();
            let hotkey = utils::translate_key(&hotkey);
            
            self.menus.push(MenuInfo { menu, name, exe, hotkey });
        }

        return Ok(()); 
    }

    fn load_field_config(&mut self) -> Result<()> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("config.xlsx");
        let range = utils::open_excel2(&path, "字段配置")?;

        let mut idx = 0;
        for row in range.rows() {
            idx = idx + 1;
            if idx <= 1 {continue;}
            let table_key = row[0].to_string();
            if table_key.is_empty() {continue;}
            let name = row[1].to_string();
            if name.is_empty() {continue;}
            let val_type = row[2].to_string();
            let editor_type = row[3].to_string();
            let opt = row[4].to_string();
            let default = row[5].to_string();
            let title = row[6].to_string();
            let desc = row[7].to_string();
            let group = row[8].to_string();
            let link_table = row[9].to_string();
            let export = row[10].to_string().to_lowercase() != "true";
            let output_header = row[11].to_string();
            let output_header:Vec<String> = match output_header.as_str() {
                ""=> Vec::new(),
                _=>{serde_json::from_str(&output_header)?}
            };
            
            println!("parse field[{}] opt[{}] default[{}] editor_type[{}] export[{}]", name, opt, default, editor_type, export);
            let field = FieldInfo::parse(name, title, desc, group, val_type, editor_type, opt, default, link_table, export, output_header)?;
            if self.field_group.contains_key(&table_key) {
                let group = self.field_group.get_mut(&table_key).unwrap();
                group.push(field);
            }
            else{
                self.field_group.insert(table_key.clone(), vec![field]);
            }
        }

        return Ok(());
    }

    fn load_tab_config(&mut self) -> Result<()>{
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("config.xlsx");
        let range = utils::open_excel2(&path, "页签配置")?;

        let mut idx = 0;
        for row in range.rows() {
            idx = idx + 1;
            if idx <= 1 {continue;}
            let table_key = row[0].to_string();
            if table_key.is_empty() {continue;}
            let tab = row[1].to_string();
            if tab.is_empty() {continue;}
            let show_name = row[2].to_string();
            let show_field = row[3].to_string();
            let master_table = row[4].to_string();
            let master_field = row[5].to_string();
            let group_field = row[6].to_string();
            let output_type = row[7].to_string();
            let output_type:Vec<String> = match output_type.as_str() {
                ""=> Vec::new(),
                _=>{serde_json::from_str(&output_type)?}
            };
            let output_path = row[8].to_string();
            let output_path:Vec<String> = match output_path.as_str() {
                ""=> Vec::new(),
                _=>{serde_json::from_str(&output_path)?}
            };
           
            let mut found = false;
            for one in &mut self.tab_cfg {
                if one.name == tab {
                    one.tabs.push(table_key.clone());
                    found = true;
                }
            }
            if !found {self.tab_cfg.push(TabConfig{name:tab.clone(), tabs:vec![table_key.clone()]});}
            let mut info = Vec::new();
            if self.field_group.contains_key(&table_key) {
                let group_field = FieldInfo::parse("__Group__".to_string(), "分组".to_string(), "编辑器分组".to_string(), "分组".to_string(), "S".to_string(), "Text".to_string(), String::new(), "默认分组".to_string(), String::new(), false, Vec::new())?;
                let sub_group_field = FieldInfo::parse("__SubGroup__".to_string(), "子分组".to_string(), "编辑器子分组".to_string(), "分组".to_string(), "S".to_string(), "Text".to_string(), String::new(), "默认子分组".to_string(), String::new(), false, Vec::new())?;
                let field = self.field_group.get_mut(&table_key).unwrap();
                field.insert(0, sub_group_field);
                field.insert(0, group_field);
                for one in field {
                    info.push(one.clone());
                }
            }
            let mut templete = Vec::new();
            if self.templete.contains_key(&table_key) {
                let t = self.templete.get(&table_key).unwrap();
                for one in t {
                    templete.push(one.clone());
                }
            }
            let data_table = DataTable::new(table_key.clone(), tab, show_name, show_field, master_table, master_field, group_field, output_type, output_path, info, templete);
            self.data_table.insert(table_key.clone(), data_table);
        }
        return Ok(());
    }

    fn load_data(&mut self) -> Result<()> {
        for (k, v) in &mut self.data_table {
            v.load_data();
        }
        return Ok(());
    }

    fn load_templete(&mut self) -> Result<()> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("config.xlsx");
        let range = utils::open_excel2(&path, "模板配置")?;

        let mut idx = 0;
        for row in range.rows() {
            idx = idx + 1;
            if idx <= 1 {continue;}
            let table_key = row[0].to_string();
            if table_key.is_empty() {continue;}
            let title = row[1].to_string();
            if title.is_empty() {continue;}
            let table = row[2].to_string();
            let content = row[3].to_string();
            let ret = serde_json::from_str(content.as_str());
            if ret.is_err() {
                let msg = format!("模板[{}]的内容[{}]解析失败", title, content);
                self.msg(msg, "错误".to_string());
                continue;
            }
            let content = ret.unwrap();
            if !self.templete.contains_key(&table_key) {
                self.templete.insert(table_key.clone(), Vec::new());
            }
            let list = self.templete.get_mut(&table_key).unwrap();
            list.push(TempleteInfo { title, table, content });
        }
        return Ok(());
    }

    pub fn _load_config(&mut self) -> Result<()> {
        self.load_field_config()?;
        self.load_templete()?;
        self.load_tab_config()?;
        self.load_data()?;
        self.load_menu_config()?;
        return Ok(());
    }

    pub fn load_config(&mut self) {
        if self.inited {return;}
        let ret = self._load_config();
        self.inited = true;
        match ret {
            Ok(_) => {},
            Err(e) => {
                self.msg(format!("读取配置失败:{:?}", e), "错误".to_string())
            },
        }
    }
}

// UI 相关接口
impl SkillEditorApp {
    fn msg(&self, content:String, title:String){
        println!("MsgBox: {}", content);
        rfd::MessageDialog::new()
            .set_title(title.as_str())
            .set_description(content.as_str())
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }

    fn draw_menu(&mut self, ctx: &egui::Context){
        egui::TopBottomPanel::top("menu").show(ctx, |ui|{
            egui::menu::bar(ui, |ui|{
                egui::widgets::global_dark_light_mode_switch(ui);
                if ui.button("保存").clicked(){ self.save_data();}
                if ui.input().key_pressed(egui::Key::S) && ui.input().modifiers.ctrl {
                    self.save_data();
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
                    if found {continue;}
                    list.push((one.menu.clone(), vec![one.clone()]));
                }
                for (menu, v) in list {
                    if menu.is_empty() {
                        for menu_info in v {
                            if ui.button(&menu_info.name).clicked(){ menu_info.trigger(); }
                        }
                    }else{
                        ui.menu_button(menu, |ui| {
                            for menu_info in v {
                                if ui.button(&menu_info.name).clicked(){ menu_info.trigger(); }
                            }
                        });
                    }
                }
            });
        });
        egui::TopBottomPanel::top("tables").show(ctx, |ui|{
            egui::menu::bar(ui, |ui|{
                let mut idx = 0;
                for one in &self.tab_cfg {
                    if ui.selectable_label(idx == self.cur_view, &one.name).clicked() {
                        self.cur_view = idx;
                    }
                    idx = idx + 1;
                }
            });
        });
    }

    fn draw_view(&mut self, ctx: &egui::Context) {
        let cfg = self.tab_cfg.get(self.cur_view);
        if cfg.is_none() {return;}
        let cfg = cfg.unwrap();
        let size = ctx.used_size();
        let unit = cfg.tabs.len() as f32 * 2.0;
        let width = size.x / unit - unit * 4.0;

        let mut copy_table = String::new();
        let mut copy_master_val = String::new();
        let mut idx = 0;
        let mut click_table = String::new();
        for key in &cfg.tabs {
            idx = idx + 1;
            let show_table = self.data_table.get(key);
            if show_table.is_none() {
                let msg = format!("表格[{}]未找到", key);
                SkillEditorApp::draw_empty_table(ctx, msg, width, idx);
                continue;
            }
            let const_one = show_table.unwrap();
            if !const_one.error.is_empty() {
                SkillEditorApp::draw_empty_table(ctx, const_one.error.clone(), width, idx);
                continue;
            }

            let mut cur_master_val = String::new();
            let master_table = const_one.master_table.clone();
            if !const_one.master_table.is_empty() {
                let master_table = self.data_table.get(&const_one.master_table);
                if master_table.is_some() {
                    let master_table = master_table.unwrap();
                    cur_master_val = master_table.get_cur_key();
                }
            }

            let data_table = self.data_table.get_mut(key).unwrap();
            if !copy_table.is_empty() && master_table == copy_table {
                println!("copytable master_table[{}] copy[{}] cur[{}]", master_table, copy_master_val, cur_master_val);
                let copy_list = data_table.get_show_name_list(&data_table.master_field, &copy_master_val, false, &"".to_string());
                for (_, one) in copy_list {
                    for (_, two) in one {
                        for (_, idx, _, _) in two {
                            data_table.copy_row(idx.clone() as usize, &cur_master_val);
                        }
                    }
                }
            }
            
            let mut show_all = None;
            let mut show_all_bool = false;
            if !data_table.master_field.is_empty() {
                show_all = Some(data_table.show_all);
                show_all_bool = data_table.show_all;
            }
            if !click_table.is_empty() && click_table == data_table.master_table {
                data_table.update_cur_row(&cur_master_val);
            }
            let list = data_table.get_show_name_list(&data_table.master_field, &cur_master_val, show_all_bool, &data_table.search);
            let (click, op, create_tmp) = SkillEditorApp::draw_list(ctx, idx, width - width * 0.4, &data_table.show_name, &list, data_table.cur_row, &mut data_table.search, &mut show_all, &data_table.templete, &mut data_table.templete_idx);
            if show_all.is_some() { data_table.show_all = show_all.unwrap(); }
            if click.is_some() {
                data_table.cur_row = click.unwrap().clone();
                click_table = data_table.table_name.clone();
            }
            if !create_tmp.is_empty() {
                self.templete_target = key.clone();
                self.templete_table = create_tmp;
                self.templete_data = HashMap::new();
                let t = data_table.templete.get(data_table.templete_idx as usize).unwrap();
                self.templete_content = t.content.clone();
                self.show_templete = true;
            }

            if op == 1 {
                data_table.create_row(&cur_master_val);
            }
            if op == 2 {
                data_table.delete_cur_row();
            }
            if op == 3 {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("xlsm", &["xlsm", "xlsx"])
                    .pick_file() {
                        match data_table.import_excel(path, data_table.table_name.clone()){
                            Ok(_) => {utils::msg("导入成功".to_string(), "成功".to_string())},
                            Err(e) => {
                                let msg = format!("导入失败: {:?}", e);
                                utils::msg(msg, "失败".to_string());
                            }
                        }
                    }
            }

            if op == 4 {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("xlsx", &["xlsx"])
                    .save_file() {
                        match data_table.export_excel(path, data_table.table_name.clone()){
                            Ok(_) => {utils::msg("导出成功".to_string(), "成功".to_string())},
                            Err(e) => {
                                let msg = format!("导出失败: {:?}", e);
                                utils::msg(msg, "失败".to_string());
                            }
                        }
                    }
            }

            if op == 5 {
                copy_master_val = data_table.get_cur_key();
                data_table.copy_cur_row(&cur_master_val);
                copy_table = key.clone();
            }
            let link_info = SkillEditorApp::draw_data(ctx, idx, data_table, width + width * 0.4);
            if link_info.is_some() {
                let link_info = link_info.unwrap();
                self.link_table = link_info.table;
                self.link_src_table = key.clone();
                self.link_src_field = link_info.field;
                self.show_link = true;
                println!("ShowLinkWindow src[{}] field[{}] link[{}]", self.link_src_table, self.link_src_field, self.link_table);
            }
        }
    }

    fn draw_list(ctx: &egui::Context, idx:i32, width: f32, title:&str, list:&HashMap<String, HashMap<String, Vec<(String, i32, i32, bool)>>>, cur: i32, search:&mut String, show_all: &mut Option<bool>, templete:&Vec<TempleteInfo>, tmp_idx:&mut i32) -> (Option<i32>, i32, String) {
        let mut ret = None;
        let mut op = 0;
        let id = format!("list_panel_{}", idx);
        let mut all = false;
        let mut create_templete = String::new();
        egui::SidePanel::left(id)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width);

                ui.horizontal(|ui|{
                    ui.heading(title);
                });
                ui.horizontal(|ui|{
                    if ui.button("+").on_hover_text("新增配置").clicked() {op=1;}
                    if ui.button("-").on_hover_text("删除配置").clicked() {op=2;}
                    if ui.button("‖").on_hover_text("复制配置").clicked() {op=5;}
                    if ui.button("↓").on_hover_text("导入配置").clicked() {op=3;}
                    if ui.button("↑").on_hover_text("导出配置").clicked() {op=4;}
                    if show_all.is_some() {
                        all = show_all.unwrap();
                        ui.checkbox(&mut all, "").on_hover_text("显示全部");
                        *show_all = Some(all);
                    }
                });
                if templete.len() > 0 {
                    ui.horizontal(|ui|{
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
                ui.horizontal(|ui|{
                    ui.text_edit_singleline(search);
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (group, one) in list.iter().sorted_by_key(|a|{a.0}) {
                        egui::CollapsingHeader::new(group)
                        .default_open(true)
                        .show(ui, |ui| {
                            for (sub_group, two) in one.iter().sorted_by_key(|a|{a.0}) {
                                egui::CollapsingHeader::new(sub_group)
                                .default_open(true)
                                .show(ui, |ui| {
                                    for (name, idx, _key_num, dup) in two {
                                        let mut txt = RichText::new(name);
                                        if *dup {
                                            txt = txt.color(Color32::RED);
                                        }
                                        if ui.selectable_label(*idx == cur, txt)
                                        .clicked(){
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

    fn draw_data(ctx: &egui::Context, idx:i32, data_table: &mut DataTable, width: f32) -> Option<LinkInfo> {
        let map = data_table.data.get_mut(data_table.cur_row as usize);
        let id1 = format!("detail_panel_{}", idx);
        let ret = egui::SidePanel::left(id1)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(width);
            if map.is_none() {
                return None;
            }else{
                let mut map = map.unwrap();
                let click = SkillEditorApp::_draw_data(ui, idx, &data_table.info, &mut map, data_table.cur);
                if click.is_none() {return None;}
                let idx = click.unwrap();
                data_table.cur = idx;
                let click_field = data_table.info.get(idx as usize);
                if click_field.is_none(){return None;}
                let click_field = click_field.unwrap();
                if click_field.link_table.is_empty() {return None;}
                return Some(LinkInfo { table: click_field.link_table.clone(), field: click_field.name.clone() });
            }
        });
        return ret.inner;
    }

    fn _draw_data(ui: &mut egui::Ui, idx:i32, field: &Vec<FieldInfo>, mut map: &mut HashMap<String, String>, select_field:i32) -> Option<i32> {
        let id1 = format!("detail_panel_{}", idx);
        let id2 = format!("detail_desc_panel_{}", idx);
        let mut ret = None;
        let select = field.get(select_field as usize);
        if select.is_some() {
            egui::TopBottomPanel::bottom(id2)
                .resizable(false)
                .show_inside(ui, |ui|{
                    let select = select.unwrap();
                    let mut desc = select.desc.as_str();
                    let txt1 = egui::TextEdit::multiline(&mut desc)
                        .desired_width(f32::INFINITY);
                    ui.add(txt1);
                });
        }

        let mut draw_info:Vec<(String, Vec<(i32, FieldInfo)>)> = Vec::new();
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

        let scroll = egui::ScrollArea::vertical().auto_shrink([false;2]);
        let size = ui.available_size();
        scroll.show(ui, |ui|{
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
                        .striped(true)
                        .min_col_width(size.x/2.0 - 64.0);
                    grid.show(ui, |ui|{
                        for (idx, one) in vec {
                            let f = one.create_ui(&mut map, ui, select_field == idx - 1);
                            if f {
                                click_flag = true;
                                click_idx = idx - 1;
                            }
                            ui.end_row();
                        }
                    });
                });
            }
        });
        return ret;
    }

    fn draw_empty_table(ctx: &egui::Context, msg:String, width: f32, idx: i32) {
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

    fn _draw_link_window(ctx: &egui::Context, title:String, open:bool, field: &Vec<FieldInfo>, map: &mut HashMap<String, String>, select_field:i32) -> (bool, Option<i32>){
        let mut state = open;
        let mut click = None;
        egui::Window::new("关联表")
        .open(&mut state)
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading(title);
            click = SkillEditorApp::_draw_data(ui, -1, field, map, select_field)
        });
        return (state, click);
    }

    fn _draw_link_window_ret(&mut self, ctx:&egui::Context) -> Result<()> {
        if !self.show_link {return Ok(());}
        let src_table = self.data_table.get(&self.link_src_table);
        if src_table.is_none(){ bail!(error::AppError::HintMsg(format!("原始表[{}]未找到", self.link_src_table)));}
        let src_table = src_table.unwrap();
        let link_val = src_table.get_field_val(&self.link_src_field);
        let data_table = self.data_table.get_mut(&self.link_table);
        if data_table.is_none() { bail!(error::AppError::HintMsg(format!("数据表[{}]未找到", self.link_table)));}
        let data_table = data_table.unwrap();
        let mut link_idx: i32 = -1;
        let mut idx = 0;
        for row in &data_table.data {
            let key = utils::map_get_string(row, &data_table.key_name, "");
            if key == link_val{
                link_idx = idx;
                break;
            }
            idx = idx + 1;
        }
        let map = data_table.data.get_mut(link_idx as usize);
        if map.is_none() { bail!(error::AppError::HintMsg(format!("数据表[{}]主键[{}]未找到", self.link_table, link_val)));}
        let map = map.unwrap();
        let key = utils::map_get_string(map, &data_table.key_name, "");
        let show = utils::map_get_string(map, &data_table.show_field, "");
        let title = format!("关联表:{} - [{}]{}", data_table.show_name, key, show);
        let (show, click) = SkillEditorApp::_draw_link_window(ctx, title, self.show_link, &data_table.info, map, data_table.cur);
        self.show_link = show;
        if click.is_some() {
            data_table.cur = click.unwrap();
        }
        return Ok(());
    }

    fn draw_link_window(&mut self, ctx:&egui::Context) {
        let ret = self._draw_link_window_ret(ctx);
        match ret {
            Ok(_) => {},
            Err(e) => {
                let cur_table = self.data_table.get(&self.link_src_table);
                if cur_table.is_none() {return;}
                let link_val = cur_table.unwrap().get_field_val(&self.link_src_field);
                let data_table = self.data_table.get_mut(&self.link_table);
                let show_button = data_table.is_some();
                let (show, new) = SkillEditorApp::draw_empty_link_window(ctx, e.to_string(), self.show_link, show_button);
                self.show_link = show;
                if !new {return;}
                if data_table.is_none(){return;}
                let data_table = data_table.unwrap();
                data_table.create_row(&String::new());
                let idx = data_table.data.len()-1;
                let new_row = data_table.data.get_mut(idx).unwrap();
                new_row.insert(data_table.key_name.clone(), link_val);
            }
        }
    }

    fn draw_empty_link_window(ctx: &egui::Context, msg:String, open:bool, show_button:bool) -> (bool, bool){
        let mut state = open;
        let mut new = false;
        egui::Window::new("关联表")
        .open(&mut state)
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            let err_info = egui::RichText::new(msg).color(Color32::RED);
            ui.label(err_info);
            if !show_button {return;}
            new = ui.button("新建").clicked();
        });
        return (state, new);
    }

    fn draw_templete(&mut self, ctx:&egui::Context) {
        let mut create = false;
        egui::Window::new("关联表")
        .open(&mut self.show_templete)
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            let field = self.field_group.get(&self.templete_table);
            if field.is_some() {
                if ui.button("创建").clicked() {
                    create = true;
                    let data_table = self.data_table.get(&self.templete_target);
                    if data_table.is_none() {return;}
                    let data_table = data_table.unwrap();
                    let mut cur_master_val = String::new();
                    if !data_table.master_table.is_empty() {
                        let master = data_table.master_table.clone();
                        let master_table = self.data_table.get(&master);
                        if master_table.is_some() {
                            let master_table = master_table.unwrap();
                            cur_master_val = master_table.get_cur_key();
                        }
                    }
                    let data_table = self.data_table.get_mut(&self.templete_target).unwrap();
                    data_table.create_row(&cur_master_val);
                    let idx = data_table.data.len() - 1;
                    let new_row = data_table.data.get_mut(idx).unwrap();
                    let mut data:HashMap<String, String> = HashMap::new();
                    for (k, v) in &self.templete_content {
                        let mut val = v.clone();
                        for (kk, vv) in &self.templete_data {
                            let templete_key = format!("%{}%", kk);
                            val = val.replace(templete_key.as_str(), vv.as_str());
                        }
                        data.insert(k.clone(), val);
                    }
                    for (k, v) in data{
                        new_row.insert(k, v);
                    }
                }


                let field = field.unwrap();
                let click = SkillEditorApp::_draw_data(ui, -2, field, &mut self.templete_data, self.templete_data_idx);

                if click.is_some(){ self.templete_data_idx = click.unwrap(); }
            }
            else{
                let err_info = egui::RichText::new(format!("模板[{}]字段配置未找到", self.templete_table)).color(Color32::RED);
                ui.label(err_info);
            }
        });
        if create {
            self.show_templete = false;
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
            cur_view: 0,
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
        }
    }
}

impl eframe::App for SkillEditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.load_config();
        self.draw_menu(ctx);
        self.draw_view(ctx);
        self.draw_link_window(ctx);
        self.draw_templete(ctx);
    }
}

