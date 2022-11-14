use std::collections::HashMap;
use anyhow::{Result, bail};
use eframe::{egui, epaint::Color32};

use crate::error;

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
    pub default: String,
    pub link_table: String,
    pub export: bool,
    pub header: Vec<String>,
    
    pub is_key: bool,
    pub is_array: bool,
    pub suffix: String,
    pub origin: String,
}


impl FieldInfo {
    pub fn parse(name:String, title:String, desc:String, group:String, field_type:String, editor_type:String, opt_str:String,default:String,link_table:String,export:bool, header:Vec<String>) -> Result<FieldInfo> {
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
            _ => {bail!(error::AppError::EditorTypeNotSupport(editor_type))}
        };
        let mut opt: Vec<EnumOption> = Vec::new();
        if editor_type == EEditorType::Enum {
            let v: Vec<Vec<String>> = serde_json::from_str(opt_str.as_str())?;
            for one in v {
                if one.len() >= 2 {
                    let val = one.get(0).unwrap().clone();
                    let show = one.get(1).unwrap().clone();
                    opt.push(EnumOption{show, val})
                }
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
            is_key,
            is_array,
            suffix,
            origin: field_type.clone(),
            default,
            link_table,
            export,
            header,
        });
    }
}


impl FieldInfo {
    fn create_one_ui(&self, val: &String, ui: &mut egui::Ui) -> (bool, String) {
        let mut flag = false;
        let mut ret = String::new();
        ui.vertical(|ui|{
            match self.editor_type {
                EEditorType::Const => {
                    let mut v = val.clone();
                    let txt1 = egui::TextEdit::multiline(&mut v).interactive(false)
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
                    
                    let txt1 = egui::TextEdit::multiline(&mut v)
                    .desired_width(f32::INFINITY);
                    if ui.add(txt1).gained_focus(){
                        flag = true;
                    }
                    ret = v;
                },
                EEditorType::Enum => {
                    let mut v = val.clone();
                    let mut label = egui::RichText::new("未定义选项").color(Color32::RED);
                    for one in &self.opt {
                        if one.val != v {continue;}
                        label = egui::RichText::new(one.show.clone());
                    }

                    let id = format!("{}_combobox", self.name);
                    egui::ComboBox::from_id_source(id)
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for one in &self.opt {
                            ui.selectable_value(&mut v, one.val.clone(), &one.show.clone());
                        }
                    });
                    ret = v;
                }
            }

            // 类型检查

            match self.val_type {
                EFieldType::Number => {
                    let mut v = val.clone();
                    let num = v.parse::<f32>();
                    if num.is_err() {
                        let err_info = egui::RichText::new("输入内容非数字").color(Color32::RED);
                        ui.label(err_info);
                    }
                },
                _ => {} // 其他不检查
            }
        });
        return (flag, ret);
    }

    pub fn create_ui(&self, map: &mut HashMap<String, String>, ui: &mut egui::Ui, selected: bool) -> bool {
        let mut flag = false;
        let title = format!("{}\r\n{}", self.title, self.name);
        if ui.selectable_label(selected, &title).clicked(){
            flag = true;
        }

        let val = map.get(&self.name);
        let v = match val {
            Some(s) => {s.clone()},
            None => {String::new()},
        };

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


