use std::{collections::HashMap, path::PathBuf};
use anyhow::{Result, bail};
use eframe::{egui, epaint::Color32};
use serde::{Serialize, Deserialize};

use crate::{error, app::{TEMPLETE_MAP_EXPR, TEMPLETE_MAP_SUB_FIELD, TempleteInfo}, utils::{self, map2tablestr}, syntax_highlight::show_code_view_ui};


#[derive(Debug, PartialEq, Clone)]
pub enum EFieldType {
    Bool,
    Number,
    Str,
    Expr,
    Table,
}

#[derive(Debug, PartialEq, Clone)]
pub enum EEditorType
{
    Const,
    Text,
    Enum,
    Check,
    UEFile,
    Blueprint,
    BitFlag,
    TempleteExpr,
    SubField,
}

#[derive(Debug, Clone)]
pub struct EnumOption{
    pub show: String,
    pub val: String,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub title: String,
    pub desc: String,
    pub group: String,
    pub val_type: EFieldType,
    pub editor_type: EEditorType,
    pub opt: Vec<EnumOption>,
    pub bit_name: Vec<String>,
    pub sub_field_key: String,
    pub default_val: String,
    pub link_table: String,
    pub export: bool,
    pub header: Vec<String>,
    
    pub is_key: bool,
    pub is_array: bool,
    pub suffix: String,
    pub origin: String,
}


impl FieldInfo {
    pub fn parse(name:String, title:String, desc:String, group:String, field_type:String, editor_type:String, opt_str:Vec<String>,default:String,link_table:String,export:bool, header:Vec<String>) -> Result<FieldInfo> {
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
        let editor_type = match editor_type.as_str() {
            "Const" => EEditorType::Const,
            "Text" => EEditorType::Text,
            "Enum" => EEditorType::Enum,
            "Check" => EEditorType::Check,
            "UEFile" => EEditorType::UEFile,
            "Blueprint" => EEditorType::Blueprint,
            "BitFlag" => EEditorType::BitFlag,
            "TempleteExpr" => EEditorType::TempleteExpr,
            "SubField" => EEditorType::SubField,
            _ => {bail!(error::AppError::EditorTypeNotSupport(editor_type))}
        };
        let mut opt: Vec<EnumOption> = Vec::new();
        if editor_type == EEditorType::Enum {
            for one in &opt_str {
                let one:Vec<&str> = one.split(":").collect();
                if one.len() >= 2 {
                    let val = one.get(0).unwrap().clone().trim().to_string();
                    let show = one.get(1).unwrap().clone().trim().to_string();
                    opt.push(EnumOption{show, val})
                }
            }
        }
        let mut bit_name = Vec::new();
        if editor_type == EEditorType::BitFlag {
            for one in &opt_str {
                bit_name.push(one.clone());
            }
        }
        let mut sub_field_key = String::new();
        if editor_type == EEditorType::SubField {
            if opt_str.len() > 0 {
                sub_field_key = opt_str[0].clone();
            }
        }
        return Ok(FieldInfo { 
            name,
            title,
            desc,
            group,
            val_type: data_type,
            editor_type,
            opt,
            bit_name,
            is_key,
            is_array,
            suffix,
            origin: field_type.clone(),
            default_val: default,
            link_table,
            export,
            header,
            sub_field_key,
        });
    }
}

fn uasset2str(path: PathBuf, is_bp: bool) -> Result<String> {
    let exe_path = dunce::canonicalize(path.clone())?;
    let path_str = exe_path.to_str().unwrap().to_string();
    let file_name = path.file_name();
    if file_name.is_none() {bail!(error::AppError::UEFileNameNotFound(path_str))}
    let file_name = file_name.unwrap().to_str();
    if file_name.is_none() {bail!(error::AppError::UEFileNameNotFound(path_str))}
    let file_name = file_name.unwrap();
    let ret = path_str.find("\\Content\\");
    if ret.is_none() {bail!(error::AppError::UEFileContentNotFound(path_str))}
    let ret = ret.unwrap();
    println!("{}", path_str);
    let path_str = path_str[ret+9..path_str.len()].to_string();
    println!("{}", path_str);
    if !path_str.ends_with(".uasset") {bail!(error::AppError::UEFileNotUasset(path_str))}
    let name_without_ext = file_name.replace(".uasset", "");
    let mut replace = format!(".{}", name_without_ext);
    if is_bp {replace = format!("{}_C", replace);}
    let path_str = path_str.replace(".uasset", &replace.as_str());
    let path_str = format!("/Game/{}", path_str).replace("\\\\", "/").replace("\\", "/");
    return Ok(path_str)
}

#[derive(Serialize, Deserialize)]
struct TempleteData{
    id: String,
    data: HashMap<String, String>,
}

impl TempleteData {
    fn get_expr(&self, info: &TempleteInfo) -> String {
        let mut expr = info.expr.clone();
        for (kk, vv) in &self.data {
            let templete_key = format!("%{}%", kk);
            expr = expr.replace(templete_key.as_str(), vv.as_str());
        }
        return expr;
    }
}

impl FieldInfo {
    fn draw_one_templete(&self, field:&Vec<FieldInfo>, mut map:&mut HashMap<String, String>, ui: &mut egui::Ui, idx:i32) -> bool {
        let mut draw_info:Vec<(String, Vec<(i32, FieldInfo)>)> = Vec::new();
        let mut idx = idx * 10000;
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
        
        // let size = ui.available_size();
        let mut click_flag = false;
        let grid_id = format!("detail_panel_grid_{}", idx);
        let grid = egui::Grid::new(grid_id)
            .num_columns(2)
            .spacing([4.0, 4.0])
            // .min_col_width(size.x/3.0)
            .striped(true);
        grid.show(ui, |ui|{
            for one in field {
                let f = one.create_ui(&mut map, ui, false, &String::new(), idx);
                if f {
                    click_flag = true;
                }
                ui.end_row();
            }
        });

        return click_flag;
    }

    fn draw_templete(&self, data:&mut Vec<TempleteData>, ui: &mut egui::Ui, idx:i32) -> (bool, String){
        let templete = TEMPLETE_MAP_EXPR.lock().unwrap();
        let mut click = false;
        let mut ret = Vec::new();
        let mut list = Vec::new();
        let mut first = String::new();
        for (k, v) in &*templete {
            if first.is_empty() {first = k.clone();}
            list.push((v.title.clone(), k.clone()));
        }
        list.sort();
        if first.is_empty() {
            let err_info = egui::RichText::new("无可用模板, 请配置模板功能页").color(Color32::YELLOW);
            ui.label(err_info);
            return (false, String::new());
        }

        let mut child_idx = 0;
        for one in data {
            child_idx = child_idx + 1;
            let mut key = &one.id;
            let mut reset = false;
            if !templete.contains_key(key) {
                key = &first;
                reset = true;
            }
            let info = templete.get(key).unwrap();
            let id = format!("{}_{}_{}_combobox", self.name, idx, child_idx);
            let mut new_id = key.clone();
            egui::ComboBox::from_id_source(id)
                .selected_text(info.title.clone())
                .show_ui(ui, |ui| {
                    for (show, key) in &list {
                        let resp = ui.selectable_value(&mut new_id, key.clone(), show);
                        if resp.changed() {
                            reset = true;
                            click = true;
                        }
                        if resp.gained_focus(){
                            click = true;
                        }
                    }
                });
            one.id = new_id;
            if reset {
                let info = templete.get(&one.id).unwrap();
                for field in &info.field {
                    one.data.insert(field.name.clone(), field.default_val.clone());
                }
            }
            let id = format!("{}_{}_{}_CollapsingHeader", self.name, idx, child_idx);
            let expr = one.get_expr(info);

            show_code_view_ui(ui, expr.as_str(), "lua");
            // ui.label(expr);
            egui::CollapsingHeader::new(info.title.clone())
                .id_source(id).show(ui, |ui|{
                    if self.draw_one_templete(&info.field, &mut one.data, ui, idx){
                        click = true;
                    }
                });
            ret.push(one.get_expr(info));
        }
        return (false, ret.join("\r\n"));
    }

    fn create_one_ui(&self, val: &String, ui: &mut egui::Ui, idx:i32) -> (bool, String) {
        let mut flag = false;
        let mut ret = String::new();
        ui.vertical(|ui|{
            match self.val_type {
                EFieldType::Expr => {
                    let msg = format!("参数:{}", self.suffix);
                    let info = egui::RichText::new(msg);
                    ui.label(info);
                },
                _ => {} // 其他不检查
            }

            match self.editor_type {
                EEditorType::Const => {
                    let mut v = val.clone();
                    let txt1 = egui::TextEdit::singleline(&mut v).interactive(false)
                        .desired_width(f32::INFINITY);
                    ui.add(txt1);
                    ret = v;
                }
                EEditorType::Check => {
                    let mut v = val.to_lowercase() == "true";
                    let one = ui.checkbox(&mut v, "");
                    if one.gained_focus() || one.changed() {
                        flag = true;
                    }
                    ret = if v {"True".to_string()} else {"false".to_string()};
                },
                EEditorType::Text => {
                    let mut v = val.clone();
                    
                    let txt = if self.val_type == EFieldType::Number {
                        egui::TextEdit::singleline(&mut v).desired_width(f32::INFINITY)
                    }
                    else{
                        egui::TextEdit::multiline(&mut v)
                            .desired_width(f32::INFINITY)
                            .desired_rows(1)
                    };

                    if self.val_type == EFieldType::Expr {
                        if crate::syntax_highlight::code_view_ui(ui, &mut v, "lua") {
                            flag = true;
                        }
                    }else{
                        if ui.add(txt).gained_focus(){
                            flag = true;
                        }
                    }

                    ret = v;
                },
                EEditorType::TempleteExpr => {
                    let mut v = val.clone();
                    let mut format = true;

                    ui.vertical(|ui| {
                        let lines = v.lines();
                        let mut json = "[]";
                        if !v.is_empty() {
                            for one in lines {
                                json = one.trim_start_matches("-");
                                break;
                            }
                        }
                        let result: Result<Vec<TempleteData>, serde_json::Error> = serde_json::from_str(json);
                        if result.is_err() {format = false;}

                        if format {
                            let mut data:Vec<TempleteData> = serde_json::from_str(json).unwrap();
                            ui.horizontal(|ui|{
                                if ui.button("+").clicked() {
                                    data.push(TempleteData { id: String::new(), data: HashMap::new() });
                                }
                                if ui.button("-").clicked() {
                                    data.pop();
                                }
                            });
                            let (click, expr) = self.draw_templete(&mut data, ui, idx);
                            if click {flag = true;}
                            let data = serde_json::to_string(&data).unwrap();
                            ret = format!("--{}\r\n{}", data, expr);
                        }
                        else{
                            ui.label("格式错误, 请删除表达式后再使用模板功能");
                            let txt = egui::TextEdit::multiline(&mut v).desired_width(f32::INFINITY);
                            ui.add(txt);
                            ret = v;
                        }
                    });
                },
                EEditorType::BitFlag => {
                    let mut v = val.clone();
                    let num = v.parse::<u32>();
                    let num = match num {
                        Ok(n) => {n},
                        Err(_) => {0},
                    };

                    ui.collapsing(self.title.clone(), |ui|{
                        let mut bit = 1;
                        let mut result = 0;
                        for name in &self.bit_name {
                            let mut select = num & bit != 0;
                            if ui.checkbox(&mut select, name).gained_focus(){
                                flag = true;
                            }
                            if select { result = result + bit; }
                            bit = bit << 1;
                        }
                        v = result.to_string();
                    });
                    let txt1 = egui::TextEdit::singleline(&mut v).interactive(false)
                        .desired_width(f32::INFINITY);
                    ui.add(txt1);
                    ret = v;
                }
                EEditorType::UEFile => {
                    ui.horizontal_centered(|ui|{
                        let mut v = val.clone();
                        if ui.button("📁").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                            .add_filter("uasset", &["uasset"])
                            .pick_file() {
                                let path = uasset2str(path, false);
                                match path {
                                    Ok(s) => { v = s },
                                    Err(e) => {println!("error:{:?}", e)},
                                }
                            }
                        }
                        let txt1 = egui::TextEdit::singleline(&mut v)
                            .desired_width(f32::INFINITY);
                        ui.add(txt1);
                        ret = v;
                    });
                }
                EEditorType::Blueprint => {
                    ui.horizontal_centered(|ui|{
                        let mut v = val.clone();
                        if ui.button("📁").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                            .add_filter("uasset", &["uasset"])
                            .pick_file() {
                                let path = uasset2str(path, true);
                                match path {
                                    Ok(s) => { v = s },
                                    Err(_) => {},
                                }
                            }
                        }
                        let txt1 = egui::TextEdit::singleline(&mut v)
                            .desired_width(f32::INFINITY);
                        ui.add(txt1);
                        ret = v;
                    });
                }
                EEditorType::SubField => {
                    let templete = TEMPLETE_MAP_SUB_FIELD.lock().unwrap();
                    let info = templete.get(&self.sub_field_key);
                    let mut v = val.clone();
                    let data = utils::tablestr2map(&v);

                    let mut msg = String::new();
                    if data.is_err() { msg = format!("格式错误[{:?}]", data); }
                    if info.is_none() { msg = format!("未找到模板[{}]", self.sub_field_key); }

                    if !msg.is_empty() {
                        let txt = egui::TextEdit::multiline(&mut v).desired_width(f32::INFINITY);
                        ui.add(txt);
                        let err_info = egui::RichText::new(msg).color(Color32::RED);
                        ui.label(err_info);
                    }
                    else{
                        let mut data = data.unwrap();
                        let info = info.unwrap();
                        self.draw_one_templete(&info.field, &mut data, ui, idx);
                        let ret = map2tablestr(&data);
                        if ret.is_ok() { v = ret.unwrap(); }
                    }
                    
                    ret = v;
                }
                EEditorType::Enum => {
                    let mut v = val.clone();
                    let mut txt = String::new();
                    let mut found = false;
                    for one in &self.opt {
                        if one.val != v {continue;}
                        txt = format!("[{}]{}", one.val, one.show);
                        found = true;
                        break;
                    }
                    if !found {txt = format!("[{}]未定义选项", v);}
                    let mut label = egui::RichText::new(txt);
                    if !found {label = label.color(Color32::RED);}
                    let size = ui.available_size();
                    let id = format!("{}_{}_combobox", self.name, idx);
                    egui::ComboBox::from_id_source(id)
                    .width(size.x * 0.95)
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for one in &self.opt {
                            let show = format!("[{}]{}", one.val, one.show);
                            ui.selectable_value(&mut v, one.val.clone(), show);
                        }
                    });
                    ret = v;
                }
            }

            let (has_err, msg) = self.check_one_data(val);
            if has_err {
                let err_info = egui::RichText::new(msg).color(Color32::RED);
                ui.label(err_info);
            }
        });
        return (flag, ret);
    }

    fn check_one_data(&self, val:&String) -> (bool, String){
        let mut ret = false;
        let mut msg = String::new();
        // 类型检查

        match self.val_type {
            EFieldType::Number => {
                let v = val.clone();
                let num = v.parse::<f32>();
                if num.is_err() {
                    ret = true;
                    msg = "输入内容非数字".to_string();
                }
            },
            EFieldType::Expr => {
                let v = val.clone();
                let lua = mlua::Lua::new();
                let mut body = v;
                if !self.suffix.starts_with("void") && !body.contains("return") {
                    body = format!("return {}", body);
                }
                let s = format!(r#"return function()
                {}
                end"#, body);
                let result = lua.load(s.as_str()).exec();
                match result {
                    Ok(_) => {},
                    Err(e) => {
                        ret = true;
                        msg = e.to_string();
                    }
                }
            },
            _ => {} // 其他不检查
        }
        return (ret, msg)
    }

    #[allow(dead_code)]
    pub fn check_data(&self, val:&String) -> (bool, String){
        let mut ret = false;
        let mut msg = String::new();

        if self.is_array {
            let mut arr:Vec<&str> = Vec::new();
            if !val.is_empty() {
                arr = val.split(";").collect();
            }
            for one in arr {
                let s = one.to_string();
                (ret, msg) = self.check_one_data(&s);
                if ret {break;}
            }
        }else{
            (ret, msg) = self.check_one_data(val);
        }
        return (ret, msg)
    }

    pub fn create_ui(&self, map: &mut HashMap<String, String>, ui: &mut egui::Ui, selected: bool, search:&String, start:i32) -> bool {
        let mut flag = false;
        let val = map.get(&self.name);
        let v = match val {
            Some(s) => {s.clone()},
            None => {String::new()},
        };

        let mut title = self.title.clone();
        if self.title != self.name {title = format!("{}({})", self.title, self.name);}
        let mut txt = egui::RichText::new(title.clone());
        let search_low = search.to_lowercase();
        if !search_low.is_empty()  && (title.to_lowercase().contains(&search_low) || v.to_lowercase().contains(&search_low)) {
            txt = txt.color(Color32::GREEN)
        }
        let resp = ui.selectable_label(selected, txt)
            .on_hover_text(self.desc.clone());
        if resp.clicked(){
            flag = true;
        }

        if self.is_array {
            let mut arr:Vec<&str> = Vec::new();
            if !v.is_empty() {
                arr = v.split(";").collect();
            }
            let mut new = Vec::new();
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui|{
                    if ui.button("+").clicked() {
                        arr.push("0");
                    }
                    if ui.button("-").clicked() {
                        arr.pop();
                    }
                });
                let mut idx = start;
                for one in arr {
                    idx = idx + 1;
                    let s = one.to_string();
                    let (f, ret) = self.create_one_ui(&s, ui, idx);
                    if f {flag = true};
                    new.push(ret);
                }
                let s = new.join(";");
                map.insert(self.name.clone(), s);
            });
        }else{
            let (f, ret) = self.create_one_ui(&v, ui, 1);
            if f {flag = true;}
            map.insert(self.name.clone(), ret);
        }
        return flag;
    }
}


