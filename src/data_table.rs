use std::{collections::{HashMap, HashSet}, path::PathBuf, fs};
use anyhow::{Result, bail};
use calamine::{open_workbook_auto, Reader};
use eframe::{egui::{self, collapsing_header::HeaderResponse}, epaint::Color32};
use itertools::Itertools;
use serde_json::json;
use walkdir::WalkDir;
use xlsxwriter::{FormatAlignment, FormatColor, FormatBorder};

use crate::{utils, error, data_field::{FieldInfo, EFieldType}, app::TempleteInfo};

#[derive(Debug)]
pub struct DataTable {
    pub table_name: String,
    pub tab: String, 
    pub show_name: String,
    pub show_field: String,
    pub master_table: String,
    pub master_field: String,
    pub group_key: String,
    pub output_type: Vec<String>,
    pub output_path: Vec<String>,

    pub info: Vec<FieldInfo>,
    pub data: Vec<HashMap<String, String>>,
    pub key_name: String,
    pub templete: Vec<TempleteInfo>,

    // UI 相关
    pub cur: i32,
    pub cur_row: i32,
    pub search: String,
    pub show_all: bool,
    pub templete_idx: i32,

    pub error: String,
}

impl DataTable {
    pub fn new(table_name: String, tab: String, show_name:String, show_field:String, master_table:String, master_field:String, group_field:String, output_type:Vec<String>, output_path:Vec<String>, info:Vec<FieldInfo>, templete:Vec<TempleteInfo>) -> DataTable{
        let key_name = String::new();
        
        let ret = DataTable{
            table_name,
            tab,
            show_name,
            show_field,
            master_table,
            master_field,
            group_key: group_field,
            output_type,
            output_path,
            
            info,
            data: Vec::new(),
            key_name,
            templete,
            
            cur: 0,
            cur_row: 0,
            templete_idx: 0,
            search:String::new(),
            show_all: false,
            error: String::new(),
        };
        
        return ret;
    }
}

impl DataTable {
    fn _load_data(&mut self) -> Result<()> {
        for one in &self.info {
            if !one.is_key {continue;}
            self.key_name = one.name.clone();
        }
        if self.key_name.is_empty() {bail!(error::AppError::TableKeyNotFound(self.table_name.clone()));}
        let path = self.get_save_json()?;
        self.load_json(&path)?;
        return Ok(());
    }
    pub fn load_data(&mut self) {
        let ret = self._load_data();
        match ret {
            Ok(_) => {},
            Err(e) => self.error = e.to_string(),
        }
    }

    pub fn _save_csv(&self, path: PathBuf, out_type: &String) -> Result<()> {
        let mut content = String::new();
        // 表头
        let mut header:Vec<Vec<String>> = vec![Vec::new(), Vec::new()];
        for one in &self.info {
            if !one.export {continue;}

            if one.header.len() > 0 {
                let mut idx = 0;
                for h in &one.header {
                    if header.len() < idx + 1 {
                        header.push(Vec::new());
                    }
                    let line = header.get_mut(idx).unwrap();
                    let mut s = h.clone();
                    let f1 = h.contains(",") || h.contains("\r") || h.contains("\n")
                        || h.contains("\'") || h.contains("\"");
                    if f1 {s = format!("\"{}\"", s);}
                    line.push(s);
                    idx = idx + 1;
                }

                let mut field_name = one.name.clone();
                let f2 = out_type == "scsv";
                if f2 { field_name = format!("\"{}\"", field_name); }
                let name_field = one.header.len();
                if header.len() <= name_field {
                    header.push(Vec::new());
                }
                let name_line = header.get_mut(name_field).unwrap();
                name_line.push(field_name);
            }else{
                let mut field_type = one.origin.clone();
                let mut field_name = one.name.clone();
                let f1 = field_type.contains(",") || field_type.contains("\r") || field_type.contains("\n")
                || field_type.contains("\'") || field_type.contains("\"");
                let f2 = out_type == "scsv";
                if f1 || f2 { field_type = format!("\"{}\"", field_type);}
                if f2 { field_name = format!("\"{}\"", field_name); }
                let type_line = header.get_mut(0).unwrap();
                type_line.push(field_type);
                let name_line = header.get_mut(1).unwrap();
                name_line.push(field_name);
            }
        }
        for one in header {
            content.push_str(one.join(",").as_str());
            content.push_str("\r\n");
        }

        // 内容
        for row in self.data.iter().sorted_by_key(|a|{utils::map_get_i32(*a, &self.key_name)}) {
            let mut one_line = Vec::new();
            for one in &self.info {
                if !one.export {continue;}
                let v = match row.get(&one.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                
                let mut tmp;
                let mut flag = false;
                if out_type == "scsv" {
                    tmp = v.trim()
                    .replace("'", "\\'")
                    .replace("\"", "\"\"");
                    flag = one.is_array || one.val_type == EFieldType::Str || one.val_type == EFieldType::Table;
                    flag = flag && !tmp.is_empty();
                }else {
                    tmp = v.trim()
                        .replace("'", "\\'")
                        .replace("\"", "\\\"");
                }
                if tmp.contains(",") || tmp.contains("\r") || tmp.contains("\n")
                    || tmp.contains("\'") || tmp.contains("\"") || flag {
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

    pub fn get_save_json(&self) -> Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("save_data");
        path.push(self.table_name.clone());
        return Ok(path);
    }


    pub fn _save_json(&self, path: PathBuf) -> Result<()> {
        println!("_save_json {:?}", path);
        if !path.exists() {
            std::fs::create_dir_all(path.clone())?;
        }

        // 清空旧数据
        for entry in WalkDir::new(&path) {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {continue;}
            fs::remove_file(p)?;
        }

        let list = self.get_show_name_list(&String::new(), &String::new(), true, &String::new());
        for (group, one) in list.iter().sorted_by_key(|a|{a.0}) {
            for (sub_group, two) in one.iter().sorted_by_key(|a|{a.0}) {
                let mut js = json!([]);
                let arr = js.as_array_mut().unwrap();
                for (_name, idx, _key_num, dup) in two {
                    let mut obj = json!({});
                    let obj_map = obj.as_object_mut().unwrap();
                    let row = self.data.get(*idx as usize).unwrap();
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
                p.push(format!("{}_{}.json", group, sub_group));
                println!("save[{:?}] to file", p);
                
                fs::write(p, serde_json::to_string_pretty(arr)?)?;
            }
        }

        Ok(())
    }

    fn load_json(&mut self, path:&PathBuf) -> Result<()> {
        if !path.exists() {return Ok(());}
        println!("load json from path[{:?}]", path);
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {continue;}
            let a1 = entry.file_name();
            let a2 = entry.file_type();
            println!("read[{:?}] to string a1[{:?}] a2[{:?}]", p, a1, a2);
            let s = std::fs::read_to_string(p)?;
            let data: Vec<HashMap<String, String>> = serde_json::from_str(&s)?;
            for one in data {
                self.data.push(one);
            }
        }
        Ok(())
    }

    pub fn get_cur_key(&self) -> String {
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

    pub fn get_field_val(&self, key:&String) -> String {
        let mut ret = String::new();
        let row = self.data.get(self.cur_row as usize);
        if row.is_none() {return ret;}
        let row = row.unwrap();
        let v = row.get(key);
        if v.is_none() {return ret;}
        let v = v.unwrap();
        ret = v.clone();
        return ret;

    }

    fn _create_row(&mut self, master_val: &String) -> HashMap<String, String> {
        let mut row = HashMap::new();
        let mut max = 1;
        let mut max_group = 1;
        let group_key = self.group_key.clone();
        let master_field = self.master_field.clone();
        println!("table[{}] create row group[{}] master[{}]", self.table_name, group_key, master_field);
        for one in &self.data {
            let key_val = utils::map_get_i32(&one, &self.key_name);
            if key_val >= max {max = key_val + 1;}
            if !group_key.is_empty() && !master_field.is_empty() {
                let master = one.get(&master_field);
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
            if group_key == one.name {v = max_group.to_string();}
            if one.is_key { v = max.to_string();}
            if master_field == one.name {v = master_val.clone();}
            println!("create_row field[{}] default[{}] is_key[{}] master_key[{}] finial[{}]", one.name, one.default, one.is_key, master_field, v);
            row.insert(one.name.clone(), v);
        }
        return row;
    }

    pub fn create_row(&mut self, master_val: &String) {
        let row = self._create_row(master_val);
        self.data.push(row);
        self.cur_row = self.data.len() as i32 - 1;
    }

    pub fn delete_cur_row(&mut self) {
        self.data.remove(self.cur_row as usize);
    }

    pub fn copy_row(&mut self, idx: usize, master_val: &String) {
        println!("copy_row {}", idx);
        let len = self.data.len();
        let cur_row = idx;
        if cur_row >= len {return;}
        let mut new_row = self._create_row(&master_val);
        let cur_row = self.data.get(cur_row as usize);
        if cur_row.is_none() {return;}
        let cur_row = cur_row.unwrap();

        let group_key = self.group_key.clone();
        let master_key = self.master_field.clone();
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

    pub fn copy_cur_row(&mut self, master_val:&String) {
        self.copy_row(self.cur_row as usize, master_val);
    }

    pub fn get_show_name_list(&self, master_key:&String, id:&String, show_all: bool, search: &String) -> HashMap<String, HashMap<String, Vec<(String, i32, i32, bool)>>> {
        let mut total: HashMap<String, HashMap<String, Vec<(String, i32, i32, bool)>>> = HashMap::new();

        let mut idx = 0;
        let mut key_cnt: HashMap<String, i32> = HashMap::new();
        for one in &self.data {
            let key = utils::map_get_string(&one, &self.key_name, "");
            let mut cnt = 0;
            if key_cnt.contains_key(&key) {
                cnt = *key_cnt.get(&key).unwrap();
            }
            cnt = cnt + 1;
            key_cnt.insert(key, cnt);
        }
        for one in &self.data {
            let name = self.get_one_show_name(one);
            let key = utils::map_get_string(&one, &self.key_name, "");
            idx = idx + 1;
            if name.is_none() {continue;}
            let name = name.unwrap();
            if !master_key.is_empty() && !show_all {
                let rel_id = one.get(master_key);
                if rel_id.is_none() {continue;}
                let rel_id = rel_id.unwrap();
                if rel_id != id {continue;}
            }
            let group = utils::map_get_string(one, "__Group__", "默认分组");
            let sub_group = utils::map_get_string(one, "__SubGroup__", "默认子分组");
            let key_num = utils::map_get_i32(one, &self.key_name);
            if !search.is_empty() && !utils::map_contains_str(one, &search) {continue;}

            if !total.contains_key(&group) {
                total.insert(group.clone(), HashMap::new());
            }
            let layer1 = total.get_mut(&group).unwrap();
            if !layer1.contains_key(&sub_group) {
                layer1.insert(sub_group.clone(), Vec::new());
            }
            let layer2 = layer1.get_mut(&sub_group).unwrap();
            let cnt = *key_cnt.get(&key).unwrap();
            let dup = cnt > 1;
            layer2.push((name, idx - 1, key_num, dup));
        }
        for (_, one) in &mut total {
            for (_, two) in one {
                two.sort_by(|a, b| {a.2.cmp(&b.2)})
            }
        }
        return total;
    }

    fn get_one_show_name(&self, map:&HashMap<String, String>) -> Option<String> {
        let v = map.get(&self.key_name);
        if v.is_none() {return None;}
        let v = v.unwrap();
        let name = match map.get(&self.show_field) {
            None => String::new(),
            Some(s) => s.clone(),
        };

        let name = format!("[{}]{}", v, name);
        return Some(name);
    }

    fn get_field_by_name(info:&Vec<FieldInfo>, name:&String) -> Option<FieldInfo> {
        for one in info {
            if one.name == *name {
                return Some(one.clone());
            }
        }
        return None;
    }

    pub fn import_excel(&mut self, path:PathBuf, tab:String) -> Result<()> {
        let mut workbook= open_workbook_auto(path)?;
        let range = workbook.worksheet_range(&tab)
            .ok_or(error::AppError::SheetNotFound(tab))??;

        let mut field_set = HashSet::new();
        for one in &self.info {
            field_set.insert(one.name.clone());
        }

        let mut row = 0;
        let mut title_row = 0;
        let mut flag = false;
        for one in range.rows() {
            let cell = one[0].to_string();
            if field_set.contains(&cell) {
                flag = true;
                break;
            }
            row = row + 1;
            title_row = title_row + 1;
        }
        if !flag {bail!(error::AppError::ImportExcelKeyNotFound(self.key_name.clone()))};

        let mut data = Vec::new();
        let max_size = range.get_size().0 as u32;
        let max_col = range.get_size().1 as u32;
        loop {
            row = row + 1;
            if row > max_size {break;}
            let mut map = HashMap::new();
            
            let mut key = String::new();

            for col in 0..max_col {
                let title = utils::get_cell(&range, title_row, col);
                let field_info = DataTable::get_field_by_name(&self.info, &title);
                if field_info.is_none() {continue;}
                let field_info = field_info.unwrap();
                let v = utils::get_cell(&range, row, col);
                if field_info.is_key {
                    key = v.clone();
                }
                map.insert(field_info.name.clone(), v);
            }

            if key.is_empty() {continue;}
            data.push(map);
        }
        self.data = data;
        Ok(())
    }

    pub fn export_excel(&self, path:PathBuf, tab:String) -> Result<()> {
        let default_path = "./导出.xlsx";
        let p = match path.to_str() {
            Some(s)=>{s},
            None => {default_path},
        };
        let wb = xlsxwriter::Workbook::new(p);
        let mut sheet = wb.add_worksheet(Some(&tab))?;
        sheet.freeze_panes(4, 1);
        sheet.set_column(0, 1, 25.0, None)?;
        let format_title = wb.add_format().set_bg_color(FormatColor::Custom(0x0070C0))
            .set_text_wrap()
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlignment::CenterAcross)
            .set_align(FormatAlignment::VerticalCenter);

        let mut col = 0;
        for one in &self.info {
            sheet.write_string(0, col, &one.title, Some(&format_title))?;
            sheet.write_string(1, col, &one.desc, Some(&format_title))?;
            sheet.write_string(2, col, &one.origin, Some(&format_title))?;
            sheet.write_string(3, col, &one.name, Some(&format_title))?;
            col = col + 1;
        }

        let mut row_idx = 3;
        for row in &self.data {
            row_idx = row_idx + 1;
            let mut col = 0;
            for one in &self.info {
                col = col + 1;
                let v = row.get(&one.name);
                if v.is_none() {continue;}
                sheet.write_string(row_idx, col - 1, v.unwrap(), None)?;
            }
        }

        wb.close()?;
        Ok(())        
    }
}