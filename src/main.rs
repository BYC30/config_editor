#![windows_subsystem = "windows"]
use std::{fs, path::PathBuf, collections::HashMap};

use calamine::{open_workbook_auto, Reader};
use eframe::{egui::{self, Ui}, App};
use anyhow::{Result, bail};
use serde_json::json;
use xlsxwriter::{FormatColor, FormatBorder, FormatAlignment};

mod error;
mod utils;

#[derive(Debug)]
struct SkillEditorApp {
    inited: bool,

    tab_list: Vec<DataGroup>,
    data_config: HashMap<String, Vec<DataConfig>>,

    data_map: HashMap<String, DataTable>,

    // UI 相关数据
    cur_view: String,
}
#[derive(Debug)]
struct DataGroup {
    title: String,
    tab: String,
    output_dir1: String,
    output_type1: String,
    output_dir2: String,
    output_type2: String,
}
#[derive(Debug)]
struct DataConfig{
    parent: String,
    id: String,
    title: String,
    path: String,
    tab: String,
    master_key: Option<String>,
    master_id: Option<String>,
    group_key: Option<String>,
    show_name: String,
}

impl DataConfig {
    fn key(&self) -> String {
        return DataConfig::get_key(&self.parent, &self.id);
    }

    fn get_key(parent:&String, id:&String) -> String {
        return format!("{}_{}", parent, id);
    }

    fn get_save_json(&self) -> Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("save_data");
        path.push(&self.parent);
        let f = format!("{}.json", self.tab);
        path.push(f);
        return Ok(path);
    }

    fn load_data_table(&self, master_key:Option<String>, group_key:Option<String>) -> Result<DataTable> {
        let mut ret = DataTable{
            show_name: self.show_name.clone(),
            key_name: String::new(),
            info: Vec::new(),
            data: Vec::new(),
            cur: 0,
            cur_row: 0,
            search:String::new(),
            master_key,
            group_key,
        };
        let range = utils::open_excel(&self.path, self.tab.as_str())?;

        let mut col:u32 = 0;
        let max_size = range.get_size().1 as u32;
        println!("max_size[{}]", max_size);
        loop {
            col = col + 1;
            if col > max_size {break;}
            let title = utils::get_cell(&range, 0, col - 1);
            let desc = utils::get_cell(&range, 1, col - 1);
            let default = utils::get_cell(&range, 2, col - 1);
            let field = utils::get_cell(&range, 3, col - 1);
            let name = utils::get_cell(&range, 4, col - 1);
            if field.is_empty() || name.is_empty() {continue;}
            println!("ReadField title[{}] desc[{}] field[{}] name[{}] default[{}]", title, desc, field, name, default);
            let field_info = FieldInfo::parse(name, title, default, desc, field, col - 1)?;
            if field_info.is_key {
                ret.key_name = field_info.name.clone();
            }
            ret.info.push(field_info);
        }
        let path = self.get_save_json()?;
        ret.load_json(&path)?;
        if ret.data.len() <= 0 { // 没有数据, 尝试从excel中读取
            let mut row = 4;
            let max_size = range.get_size().0 as u32;
            loop {
                row = row + 1;
                if row > max_size {break;}
                let mut map = HashMap::new();
                
                let mut key = String::new();
                for one in &ret.info {
                    let v = utils::get_cell(&range, row, one.col);
                    if one.is_key {
                        key = v.clone();
                    }
                    map.insert(one.name.clone(), v);
                }
                if key.is_empty() {continue;}
                ret.data.push(map);
            }
        }
        return Ok(ret);
    }
}

#[derive(Debug)]
struct DataTable {
    show_name: String,
    key_name: String,
    master_key: Option<String>,
    group_key: Option<String>,

    info: Vec<FieldInfo>,
    data: Vec<HashMap<String, String>>,

    // UI 相关
    cur: i32,
    cur_row: i32,
    search: String,
}

impl DataTable {
    fn _save_csv(&self, path: PathBuf, out_type: &String) -> Result<()> {
        let mut content = String::new();
        // 表头
        let mut type_line = Vec::new();
        let mut name_line = Vec::new();
        for one in &self.info {
            let mut tmp = one.origin.clone();
            if tmp.contains(",") || tmp.contains("\r") || tmp.contains("\n")
                || tmp.contains("\'") || tmp.contains("\"") {
                tmp = format!("\"{}\"", tmp);
            }
            type_line.push(tmp);
            name_line.push(one.name.clone());
        }
        content.push_str(type_line.join(",").as_str());
        content.push_str("\r\n");
        content.push_str(name_line.join(",").as_str());
        content.push_str("\r\n");

        // 内容
        for row in &self.data {
            let mut one_line = Vec::new();
            for one in &self.info {
                let v = match row.get(&one.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                
                let mut tmp;
                if out_type == "scsv" {
                    tmp = v.trim()
                    .replace("'", "\\'")
                    .replace("\"", "\"\"");
                }else {
                    tmp = v.trim()
                        .replace("'", "\\'")
                        .replace("\"", "\\\"");
                }
                if tmp.contains(",") || tmp.contains("\r") || tmp.contains("\n")
                    || tmp.contains("\'") || tmp.contains("\"") {
                    tmp = format!("\"{}\"", tmp);
                }
                if one.val_type == EFieldType::Bool {
                    tmp = tmp.to_lowercase();
                }
                one_line.push(tmp);
            }
            content.push_str(one_line.join(",").as_str());
            content.push_str("\r\n");
        }

        let mut p = path.clone();
        p.pop();
        if !p.exists() {
            std::fs::create_dir_all(p.clone())?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn _save_json(&self, path: PathBuf) -> Result<()> {
        let mut js = json!([]);
        let arr = js.as_array_mut().unwrap();
        for row in &self.data {
            let mut obj = json!({});
            let obj_map = obj.as_object_mut().unwrap();
            for one in &self.info {
                let v = match row.get(&one.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                
                let tmp = v.trim();
                obj_map.insert(one.name.clone(), serde_json::Value::String(tmp.to_string()));
            }
            arr.push(obj);
        }

        let mut p = path.clone();
        p.pop();
        if !p.exists() {
            std::fs::create_dir_all(p.clone())?;
        }
        fs::write(path, serde_json::to_string(arr)?)?;
        Ok(())
    }

    fn load_json(&mut self, path:&PathBuf) -> Result<()> {
        if !path.exists() {return Ok(());}
        let s = std::fs::read_to_string(path)?;
        let data: Vec<HashMap<String, String>> = serde_json::from_str(&s)?;
        self.data = data;
        Ok(())
    }

    fn get_cur_val(&self) -> String {
        let mut ret = String::new();
        let row = self.data.get(self.cur_row as usize);
        if row.is_none() {return ret;}
        let row = row.unwrap();
        let v = row.get(&self.key_name);
        if v.is_none() {return ret;}
        let v = v.unwrap();
        ret = v.clone();
        return ret;
    }

    fn _create_row(&mut self, master_val: &String) -> HashMap<String, String> {
        let mut row = HashMap::new();
        let mut max = 1;
        let mut max_group = 1;
        let mut group_key = String::new();
        let mut master_key = String::new();
        if self.group_key.is_some() {group_key = self.group_key.clone().unwrap();}
        if self.master_key.is_some() {master_key = self.master_key.clone().unwrap();}
        for one in &self.data {
            let key_val = utils::map_get_i32(&one, &self.key_name);
            if key_val >= max {max = key_val + 1;}
            if !group_key.is_empty() && !master_key.is_empty() {
                let master = one.get(&master_key);
                if master.is_some() {
                    let master = master.unwrap();
                    if master == master_val {
                        let group_val = utils::map_get_i32(&one, &group_key);
                        if group_val >= max_group {max_group = group_val + 1;}
                    }
                }
            }
        }

        for one in &self.info {
            let mut v = one.default.clone();
            if one.val_type == EFieldType::Number {v = one.suffix.clone();}
            if group_key == one.name {v = max_group.to_string();}
            if one.is_key { v = max.to_string();}
            if master_key == one.name {v = master_val.clone();}
            row.insert(one.name.clone(), v);
        }
        return row;
    }

    fn create_row(&mut self, master_val: &String) {
        let row = self._create_row(master_val);
        self.data.push(row);
        self.cur_row = self.data.len() as i32 - 1;
    }

    fn delete_cur_row(&mut self) {
        self.data.remove(self.cur_row as usize);
    }

    fn copy_row(&mut self, idx: usize, master_val: &String) {
        println!("copy_row {}", idx);
        let len = self.data.len();
        let cur_row = idx;
        if cur_row >= len {return;}
        let mut new_row = self._create_row(&master_val);
        let cur_row = self.data.get(cur_row as usize);
        if cur_row.is_none() {return;}
        let cur_row = cur_row.unwrap();

        let mut group_key = String::new();
        let mut master_key = String::new();
        if self.group_key.is_some() {group_key = self.group_key.clone().unwrap();}
        if self.master_key.is_some() {master_key = self.master_key.clone().unwrap();}
        for one in &self.info {
            if one.is_key {continue;}
            if group_key == one.name {continue;}
            if master_key == one.name {continue;}
            let cur = cur_row.get(&one.name);
            if cur.is_none() {continue;}
            let cur = cur.unwrap();
            new_row.insert(one.name.clone(), cur.clone());
        }
        self.data.push(new_row);
        self.cur_row = self.data.len() as i32 - 1;
    }

    fn copy_cur_row(&mut self, master_val:&String) {
        self.copy_row(self.cur_row as usize, master_val);
    }

    fn get_show_name_list(&self, key:&Option<String>, id:&String) -> Vec<(String, i32)> {
        let mut ret = Vec::new();

        let mut idx = 0;
        for one in &self.data {
            let name = self.get_one_show_name(one);
            idx = idx + 1;
            if name.is_none() {continue;}
            let name = name.unwrap();
            if key.is_some() {
                let k = key.clone().unwrap();
                let rel_id = one.get(&k);
                if rel_id.is_none() {continue;}
                let rel_id = rel_id.unwrap();
                if rel_id != id {continue;}
            }
            ret.push((name, idx - 1));
        }
        return ret;
    }

    fn get_one_show_name(&self, map:&HashMap<String, String>) -> Option<String> {
        let v = map.get(&self.key_name);
        if v.is_none() {return None;}
        let v = v.unwrap();
        let name = match map.get(&self.show_name) {
            None => String::new(),
            Some(s) => s.clone(),
        };

        let name = format!("[{}]{}", v, name);
        return Some(name);
    }

    fn import_excel(&mut self, path:PathBuf, tab:String) -> Result<()> {
        let mut workbook= open_workbook_auto(path)?;
        let range = workbook.worksheet_range(&tab)
            .ok_or(error::AppError::SheetNotFound(tab))??;

        let mut data = Vec::new();
        let mut row = 3;
        let max_size = range.get_size().0 as u32;
        loop {
            row = row + 1;
            if row > max_size {break;}
            let mut map = HashMap::new();
            
            let mut key = String::new();
            for one in &self.info {
                let v = utils::get_cell(&range, row, one.col);
                if one.is_key {
                    key = v.clone();
                }
                map.insert(one.name.clone(), v);
            }
            if key.is_empty() {continue;}
            data.push(map);
        }
        self.data = data;
        Ok(())
    }

    fn export_excel(&self, path:PathBuf, tab:String) -> Result<()> {
        let default_path = "./导出.xlsx";
        let p = match path.to_str() {
            Some(s)=>{s},
            None => {default_path},
        };
        let wb = xlsxwriter::Workbook::new(p);
        let mut sheet = wb.add_worksheet(Some(&tab))?;
        sheet.freeze_panes(4, 1);
        sheet.set_column(0, 1, 25.0, None)?;
        let format_title = wb.add_format().set_bg_color(FormatColor::Custom(0xffc000))
            .set_text_wrap()
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlignment::CenterAcross)
            .set_align(FormatAlignment::VerticalCenter);

        for one in &self.info {
            let col = one.col as u16;
            sheet.write_string(0, col, &one.title, Some(&format_title))?;
            sheet.write_string(1, col, &one.desc, Some(&format_title))?;
            sheet.write_string(2, col, &one.origin, Some(&format_title))?;
            sheet.write_string(3, col, &one.name, Some(&format_title))?;
        }

        let mut row_idx = 3;
        for row in &self.data {
            row_idx = row_idx + 1;
            for one in &self.info {
                let col = one.col as u16;
                let v = row.get(&one.name);
                if v.is_none() {continue;}
                let col = one.col as u16;
                sheet.write_string(row_idx, col, v.unwrap(), None)?;
            }
        }

        wb.close()?;
        Ok(())        
    }
}

#[derive(Debug, PartialEq)]
enum EFieldType {
    Bool,
    Number,
    Str,
    Expr,
    Table,
}
#[derive(Debug)]
struct FieldInfo {
    name: String,
    title: String,
    desc: String,
    val_type: EFieldType,
    is_key: bool,
    is_array: bool,
    suffix: String,
    col: u32,
    origin: String,
    default: String,
}

impl FieldInfo {

    fn create_one_ui(&self, val: &String, ui: &mut egui::Ui) -> (bool, String) {
        let mut flag = false;
        let mut ret = String::new();

        match self.val_type {
            EFieldType::Bool => {
                let mut v = val.to_lowercase() == "true";
                let one = ui.checkbox(&mut v, "");
                if one.gained_focus() || one.changed() {
                    flag = true;
                }
                ret = if v {"True".to_string()} else {"false".to_string()};
            },
            EFieldType::Number => {
                let mut v = val.clone();
                let old = v.clone();

                let txt1 = egui::TextEdit::multiline(&mut v)
                    .desired_width(f32::INFINITY);
                if ui.add(txt1).gained_focus(){
                    flag = true;
                }

                let num = v.parse::<f32>();

                match num {
                    Ok(n) => {
                        ret = v;
                    },
                    Err(e) => {
                        if v.is_empty() {
                            ret = "0".to_string();
                        }else{
                            ret = old;
                            let content = format!("字段[{}]输入[{}]错误", self.title, v);
                            utils::msg(content, "错误".to_string());
                        }
                    },
                };
            },
            EFieldType::Str => {
                let mut v = val.clone();

                let txt1 = egui::TextEdit::multiline(&mut v)
                    .desired_width(f32::INFINITY);
                if ui.add(txt1).gained_focus(){
                    flag = true;
                }
                ret = v;
            },
            EFieldType::Expr => {
                let mut v = val.clone();

                let txt1 = egui::TextEdit::multiline(&mut v)
                    .desired_width(f32::INFINITY);
                if ui.add(txt1).gained_focus(){
                    flag = true;
                }
                ret = v;
            },
            EFieldType::Table => {
                let mut v = val.clone();

                let txt1 = egui::TextEdit::multiline(&mut v)
                    .desired_width(f32::INFINITY);
                if ui.add(txt1).gained_focus(){
                    flag = true;
                }
                ret = v;
            },
        }
        return (flag, ret);
    }

    fn create_ui(&self, map: &mut HashMap<String, String>, ui: &mut egui::Ui, selected: bool) -> bool {
        let mut flag = false;
        let mut title = self.title.clone();
        if title.is_empty() {title = self.name.clone();}
        if ui.selectable_label(selected, &title).clicked(){
            flag = true;
        }

        let val = map.get(&self.name);
        let v = match val {
            Some(s) => {s.clone()},
            None => {String::new()},
        };

        if self.is_array {
            let mut arr:Vec<&str> = v.split(";").collect();
            let mut new = Vec::new();
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui|{
                    if ui.button("+").clicked() {
                        arr.push("");
                    }
                    if ui.button("-").clicked() {
                        arr.pop();
                    }
                });
                for one in arr {
                    let s = one.to_string();
                    let (f, ret) = self.create_one_ui(&s, ui);
                    if f {flag = true};
                    new.push(ret);
                }
                let s = new.join(";");
                map.insert(self.name.clone(), s);
            });    
        }else{
            let (f, ret) = self.create_one_ui(&v, ui);
            if f {flag = true;}
            map.insert(self.name.clone(), ret);
        }
        return flag;
    }
}

impl FieldInfo {
    fn parse(name:String, title:String, default:String, desc:String, field_type:String, col:u32) -> Result<FieldInfo> {
        let mut tmp = field_type.clone();
        let mut prefix = String::new();
        let arr:Vec<&str> = tmp.split("<").collect();
        if arr.len() == 2 {
            prefix = arr[0].to_string();
            tmp = arr[1].to_string();
        }
        else{
            tmp = arr[0].to_string();
        }
        let mut suffix = String::new();
        let arr:Vec<&str> = tmp.split(">").collect();
        let field = arr[0];
        let data_type = match field {
            "B" => EFieldType::Bool,
            "N" => EFieldType::Number,
            "S" => EFieldType::Str,
            "E" => EFieldType::Expr,
            "M" => EFieldType::Table,
            _ => {bail!(error::AppError::FieldTypeNotSupport(field.to_string()));}
        };
        if arr.len() == 2 {
            suffix = arr[1].to_string();
        }

        let mut is_key = false;
        if prefix == "K" {is_key = true;}
        let mut is_array = false;
        if prefix == "A" {is_array = true;}
        return Ok(FieldInfo { 
            name,
            title,
            desc,
            val_type: data_type,
            is_key,
            is_array,
            col,
            suffix,
            origin: field_type.clone(),
            default,
        });
    }
}

impl SkillEditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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

    fn _save_data(&self) -> Result<()> {
        let mut path = std::env::current_exe()?;
        path.pop();

        for group in &self.tab_list {
            let cfg = self.data_config.get(&group.tab);
            if cfg.is_none() {continue;}
            let cfg = cfg.unwrap();
            for one in cfg {
                let key = one.key();
                let data = self.data_map.get(&key);
                if data.is_none() {continue;}
                let data = data.unwrap();
                let mut p = path.clone();
                p.push(group.output_dir1.clone());
                let f = format!("{}.csv", one.tab);
                p.push(f);
                data._save_csv(p, &group.output_type1)?;
                let mut p = path.clone();
                p.push(group.output_dir2.clone());
                let f = format!("{}.csv", one.tab);
                p.push(f);
                data._save_csv(p, &group.output_type2)?;

                let p = one.get_save_json()?;
                data._save_json(p)?;
            }
        }

        Ok(())
    }

    fn save_data(&mut self){
        let ret = self._save_data();
        match ret {
            Ok(_) => {self.msg("导出成功".to_string(), "导出成功".to_string())},
            Err(e) => {self.msg(format!("导出失败:{:?}", e), "导出失败".to_string())},
        }
    }

    fn _load_config(&mut self) -> Result<()> {
        if self.inited {return Ok(())}
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("config.xlsx");
        let range = utils::open_excel2(&path, "config")?;
        let mut idx = 0;
        for row in range.rows() {
            idx = idx + 1;
            if idx <= 1 {continue;}
            let title = match row[0].get_string() {
                None => continue,
                Some(x) => x.to_string(),
            };
            let tab = match row[1].get_string() {
                None => continue,
                Some(x) => x.to_string(),
            };
            let output_dir1 = match row[2].get_string() {
                None => String::new(),
                Some(x) => x.to_string(),
            };
            let output_type1 = match row[3].get_string() {
                None => String::new(),
                Some(x) => x.to_string(),
            };
            let output_dir2 = match row[4].get_string() {
                None => String::new(),
                Some(x) => x.to_string(),
            };
            let output_type2 = match row[5].get_string() {
                None => String::new(),
                Some(x) => x.to_string(),
            };
            self.tab_list.push(DataGroup { title, tab, output_dir1, output_type1, output_dir2, output_type2 })
        }

        for one in &self.tab_list {
            let mut v: Vec<DataConfig> = Vec::new();
            let data = utils::open_excel2(&path, &one.tab)?;
            let mut idx = 0;
            for row in data.rows() {
                idx = idx + 1;
                if idx <= 1 {continue;}
                if row.len() < 8 {bail!(error::AppError::ConfigFormatError(one.tab.clone()))}
                let id = row[0].to_string();
                let title = match row[1].get_string() {
                    None => continue,
                    Some(x) => x.to_string(),
                };
                let path = match row[2].get_string() {
                    None => continue,
                    Some(x) => x.to_string(),
                };
                let tab = match row[3].get_string() {
                    None => continue,
                    Some(x) => x.to_string(),
                };
                let master_key = match row[4].get_string() {
                    None => None,
                    Some(x) => Some(x.to_string()),
                };
                let master_id = match row[5].get_float() {
                    None => None,
                    Some(x) => Some(x.to_string()),
                };
                let group_key = match row[6].get_string() {
                    None => None,
                    Some(x) => Some(x.to_string()),
                };
                let show_name = match row[7].get_string() {
                    None => String::new(),
                    Some(x) => x.to_string(),
                };

                let data_cfg = DataConfig {
                    id:id.clone(), 
                    title, 
                    path, 
                    tab, 
                    master_key: master_key.clone(), 
                    master_id,
                    group_key: group_key.clone(),
                    parent:
                    one.tab.clone(),
                    show_name
                };
                let table = data_cfg.load_data_table(master_key, group_key)?;
                let key = data_cfg.key();
                self.data_map.insert(key, table);
                v.push(data_cfg);
            }
            self.data_config.insert(one.tab.clone(), v);
        }

        self.inited = true;
        if self.tab_list.len()> 0 {
            let one = self.tab_list.get(0).unwrap();
            self.cur_view = one.tab.clone();
        }
        
        return Ok(());
    }

    fn load_config(&mut self) {
        let ret = self._load_config();
        match ret {
            Ok(_) => {},
            Err(e) => {self.msg(format!("读取配置失败:{:?}", e), "错误".to_string())},
        }
    }
}

// UI 相关接口
impl SkillEditorApp {
    fn msg(&self, content:String, title:String){
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
            });
        });
        egui::TopBottomPanel::top("tables").show(ctx, |ui|{
            egui::menu::bar(ui, |ui|{
                for one in &self.tab_list {
                    if ui.selectable_label(one.tab == self.cur_view, &one.title).clicked() {
                        self.cur_view = one.tab.clone();
                    }
                }
            });
        });
    }

    fn draw_view(&mut self, ctx: &egui::Context) {
        let cfg = self.data_config.get_mut(&self.cur_view);
        if cfg.is_none() {return;}
        let cfg = cfg.unwrap();
        let size = ctx.used_size();
        let unit = cfg.len() as f32 * 2.0;
        let width = size.x / unit - unit * 4.0;

        let mut idx = 0;
        let mut copy_id = String::new();
        let mut copy_master_val = String::new();
        for one in cfg {
            idx = idx + 1;
            let mut cur_master_val = String::new();
            let mut master_key = String::new();
            if let Some(master_id) = &one.master_id {
                master_key = DataConfig::get_key(&one.parent, master_id);
                let master_table = self.data_map.get_mut(&master_key);
                if master_table.is_some() {
                    let master_table = master_table.unwrap();
                    cur_master_val = master_table.get_cur_val();
                }
            }

            let key = one.key();
            let data_table = self.data_map.get_mut(&key);
            if data_table.is_none() {continue;}
            let data_table = data_table.unwrap();
            let list = data_table.get_show_name_list(&one.master_key, &cur_master_val);
            if !copy_id.is_empty() && master_key == copy_id {
                let copy_list = data_table.get_show_name_list(&one.master_key, &copy_master_val);
                for (_k, idx) in &copy_list {
                    data_table.copy_row(idx.clone() as usize, &cur_master_val);
                }
            }
            let (click, op) = SkillEditorApp::draw_list(ctx, idx, width - width * 0.4, &one.title, &list, data_table.cur_row, &mut data_table.search);
            if click.is_some() {
                data_table.cur_row = click.unwrap().clone();
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
                        match data_table.import_excel(path, one.tab.clone()){
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
                        match data_table.export_excel(path, one.tab.clone()){
                            Ok(_) => {utils::msg("导出成功".to_string(), "成功".to_string())},
                            Err(e) => {
                                let msg = format!("导出失败: {:?}", e);
                                utils::msg(msg, "失败".to_string());
                            }
                        }
                    }
            }

            if op == 5 {
                copy_master_val = data_table.get_cur_val();
                data_table.copy_cur_row(&cur_master_val);
                copy_id = one.key();
            }
            SkillEditorApp::draw_data(ctx, idx, data_table, width + width * 0.4);
        }
    }

    fn draw_list(ctx: &egui::Context, idx:i32, width: f32, title:&str, list:&Vec<(String, i32)>, cur: i32, search:&mut String) -> (Option<i32>, i32) {
        let mut ret = None;
        let mut op = 0;
        let id = format!("list_panel_{}", idx);
        egui::SidePanel::left(id)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width);

                ui.horizontal(|ui|{
                    ui.heading(title);
                    if ui.button("+").on_hover_text("新增配置").clicked() {op=1;}
                    if ui.button("-").on_hover_text("删除配置").clicked() {op=2;}
                    if ui.button("‖").on_hover_text("复制配置").clicked() {op=5;}
                    if ui.button("↓").on_hover_text("导入配置").clicked() {op=3;}
                    if ui.button("↑").on_hover_text("导出配置").clicked() {op=4;}
                });
                ui.horizontal(|ui|{
                    ui.text_edit_singleline(search);
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (one, idx) in list {
                        if search.is_empty() || one.contains(search.as_str()) {
                            if ui.selectable_label(*idx == cur, one).clicked(){
                                ret = Some(idx.clone());
                            }
                        }
                    }
                });
            });

        return (ret, op);
    }

    fn draw_data(ctx: &egui::Context, idx:i32, data_table: &mut DataTable, width: f32) {
        let id1 = format!("detail_panel_{}", idx);
        let id2 = format!("detail_desc_panel_{}", idx);

        let map = data_table.data.get_mut(data_table.cur_row as usize);
        
        egui::SidePanel::left(id1)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width);

                let field = &data_table.info;
                if map.is_none() {return;}

                let select = field.get(data_table.cur as usize);
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

                let mut map = map.unwrap();
                let scroll = egui::ScrollArea::vertical().auto_shrink([false;2]);
                let size = ui.available_size();
                scroll.show(ui, |ui|{
                    let grid_id = format!("detail_panel_grid_{}", idx);
                    let grid = egui::Grid::new(grid_id)
                        .num_columns(2)
                        .spacing([4.0, 4.0])
                        .striped(true)
                        .min_col_width(size.x/2.0 - 64.0);
                    grid.show(ui, |ui|{


                        let mut idx = 0;
                        let mut click_flag = false;
                        let mut click_idx = 0;
                        for one in field {
                            idx = idx + 1;
                            let mut data = String::new();
                            let f = one.create_ui(&mut map, ui, data_table.cur == idx - 1);
                            if f {
                                click_flag = true;
                                click_idx = idx - 1;
                            }

                            ui.end_row();
                        }
                        if click_flag {
                            data_table.cur = click_idx;
                        }
                    });

                });
            });
        }
    }


impl Default for SkillEditorApp {
    fn default() -> Self {
        Self {
            inited: false,
            tab_list: Vec::new(),
            data_config: HashMap::new(),
            data_map: HashMap::new(),

            cur_view: String::new(),
        }
    }
}

impl eframe::App for SkillEditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.load_config();
        self.draw_menu(ctx);
        self.draw_view(ctx);
    }
}

fn main() {
    let mut opt = eframe::NativeOptions::default();
    opt.maximized = true;
    eframe::run_native("技能编辑器", opt, Box::new(|cc| Box::new(SkillEditorApp::new(cc))));
}
