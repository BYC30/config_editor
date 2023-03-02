use std::{collections::{HashMap, HashSet}, path::PathBuf, fs};
use anyhow::{Result, bail};
use calamine::{open_workbook_auto, Reader};
use itertools::Itertools;
use serde_json::json;
use walkdir::WalkDir;
use xlsxwriter::{FormatAlignment, FormatColor, FormatBorder};

use crate::{utils, error, data_field::FieldInfo, app::app::TempleteInfo, saver::{self, DataSaver}};

#[derive(Debug)]
pub struct DataTable {
    pub table_name: String,
    pub show_name: String,
    pub show_field: String,
    pub master_field: String,
    pub group_key: String,
    pub export_sort: String,
    pub output_type: Vec<String>,
    pub output_path: Vec<String>,

    pub info: Vec<FieldInfo>,
    pub data: Vec<HashMap<String, String>>,
    pub key_name: String,
    pub templete: Vec<TempleteInfo>,
    pub post_save_exec: String,
    pub reload_editor: bool,
    pub data_hash: String,

    // UI 相关
    pub cur: i32,
    pub cur_row: i32,
    pub search: String,
    pub detail_search: String,
    pub show_all: bool,
    pub templete_idx: i32,

    pub error: String,
}

impl DataTable {
    pub fn new(table_name: String, show_name:String, show_field:String, master_field:String, group_field:String, export_sort:String, output_type:Vec<String>, output_path:Vec<String>, info:Vec<FieldInfo>, templete:Vec<TempleteInfo>, post_save_exec:String) -> DataTable{
        let key_name = String::new();
        
        let ret = DataTable{
            table_name,
            show_name,
            show_field,
            master_field,
            group_key: group_field,
            export_sort,
            output_type,
            output_path,
            post_save_exec,
            reload_editor: false,
            data_hash: String::new(),

            info,
            data: Vec::new(),
            key_name,
            templete,
            
            cur: 0,
            cur_row: 0,
            templete_idx: 0,
            search:String::new(),
            detail_search:String::new(),
            show_all: false,
            error: String::new(),
        };
        
        return ret;
    }
}

impl DataTable {
    fn calc_data_hash(&self) -> String {
        let json = serde_json::to_string(&self.data).unwrap(); 
        let hash = format!("{:x}", md5::compute(&json));
        if self.table_name == "skill" {
            println!("calc data[{}] hash[{}] old[{}]", json, hash, self.data_hash);
        }
        return hash;
    }

    fn _load_data(&mut self) -> Result<()> {
        for one in &self.info {
            if !one.is_key {continue;}
            self.key_name = one.name.clone();
            if self.export_sort.is_empty() {self.export_sort = self.key_name.clone();}
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

    pub fn output(&self, path: PathBuf, out_type: &String) -> Result<()> {
        let content = match out_type.as_str() {
            "csv" => {saver::csv::CsvSaver::output(&self.info, &self.data, &self.key_name)?},
            "scsv" => {saver::scsv::ScsvSaver::output(&self.info, &self.data, &self.key_name)?},
            "json" => {saver::json::JsonSaver::output(&self.info, &self.data, &self.key_name)?},
            _ => {bail!(error::AppError::ExportTypeError(out_type.clone()));}
        };
        let mut full_path = path.clone();
        if full_path.is_dir() {
            match out_type.as_str() {
                "csv" | "scsv" => {full_path.push(format!("{}.csv", self.table_name));},
                "json" => {full_path.push(format!("{}.json", self.table_name));},
                _ => {bail!(error::AppError::ExportTypeError(out_type.clone()));}
            };
        }
        let mut dir = full_path.clone();
        dir.pop();
        if !dir.exists() { std::fs::create_dir_all(dir.clone())?; }
        fs::write(full_path, content)?;
        Ok(())
    }

    pub fn get_save_json(&self) -> Result<PathBuf> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("save_data");
        path.push(self.table_name.clone());
        return Ok(path);
    }


    pub fn save_json(&mut self, force: bool) -> Result<(bool, String)> {
        let hash = self.calc_data_hash();
        if !force && hash == self.data_hash {return Ok((false, "未改变, 跳过".to_string()));}

        let mut path = std::env::current_exe()?;
        path.pop();

        let mut idx = 0;
        for output_type in &self.output_type {
            if self.output_path.len() > idx {
                let mut p = path.clone();
                let path = self.output_path.get(idx).unwrap();
                p.push(path.clone());
                self.output(p, output_type)?;
            }
            idx = idx + 1;
        }
        let p = self.get_save_json()?;
        self._save_json(p)?;

        let mut msg = String::new();
        if !self.post_save_exec.is_empty() {
            let result = utils::exec_bat(&self.post_save_exec);
            msg = match result {
                Ok(_) => {"后处理脚本:执行成功".to_string()},
                Err(e) => {format!("后处理脚本:{:?}", e)},
            };
        }

        self.data_hash = hash;
        let mut ret_msg = "成功".to_string();
        if !msg.is_empty() {
            ret_msg = format!("{} - {}", ret_msg, msg);
        }
        return Ok((true, ret_msg));
    }

    fn _save_json(&self, path: PathBuf) -> Result<()> {
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
                for (_name, idx, _key_num, _dup) in two {
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
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {continue;}
            let s = std::fs::read_to_string(p)?;
            let data: Vec<HashMap<String, String>> = serde_json::from_str(&s)?;
            for mut one in data {
                for field in &self.info {
                    if one.contains_key(&field.name) {continue;}
                    one.insert(field.name.clone(), field.default_val.clone());
                }

                self.data.push(one);
            }
        }
        self.data_hash = self.calc_data_hash(); 
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
            let mut v = one.default_val.clone();
            if group_key == one.name {v = max_group.to_string();}
            if one.is_key { v = max.to_string();}
            if master_field == one.name {v = master_val.clone();}

            let v = v.replace("%key%", max.to_string().as_str());
            let v = v.replace("%group%", max_group.to_string().as_str());
            let v = v.replace("%master%", master_val.as_str());
            row.insert(one.name.clone(), v);
        }

        println!("创建数据: {:?}", row);
        return row;
    }

    pub fn create_row(&mut self, master_val: &String) {
        let row = self._create_row(master_val);
        self.data.push(row);
        self.cur_row = self.data.len() as i32 - 1;
    }

    pub fn delete_cur_row(&mut self, next_row:i32) {
        if self.cur_row < 0 || self.cur_row >= self.data.len() as i32 {return;}
        self.data.remove(self.cur_row as usize);
        self.cur_row = next_row;
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
            // for field in &self.info {
            //     let val = utils::map_get_string(one, &field.name, &String::new());
            //     let (err, _) = field.check_data(&val);
            //     if err {
            //         dup = true;
            //         name = format!("{} - {}", name, field.name);
            //         break;
            //     }
            // }
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

    pub fn update_cur_row(&mut self, master_val:&String) {
        let list = self.get_show_name_list(&self.master_field, master_val, false, &String::new());
        for (_, one) in list {
            for (_, two) in one {
                for (_, idx, _, _) in two {
                    self.cur_row = idx;
                    return;
                }
            }
        }
        self.cur_row = -1;
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
        for row in self.data.iter().sorted_by_key(|a|{utils::map_get_i32(a, &self.export_sort)}) {
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
