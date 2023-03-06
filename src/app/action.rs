
use std::{collections::{HashMap, HashSet}, path::PathBuf};

use anyhow::{Result, bail};
use calamine::Reader;

use crate::{data::data_table::DataTable, error, utils};

pub type DataAction = Box<dyn Action<Target=HashMap<String, DataTable>, Output=String>>;

pub trait Action {
    type Target;
    type Output;

    fn apply(&mut self, target: &mut Self::Target){
        self.redo(target);
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output;

    fn redo(&mut self, target: &mut Self::Target) -> Self::Output;
}

pub struct ActionList<T, R> {
    actions: Vec<Box<dyn Action<Target = T, Output = R>>>,
    current: usize,
}

impl<T, R> ActionList<T, R> {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            current: 0,
        }
    }

    pub fn apply(&mut self, mut action: Box<dyn Action<Target = T, Output = R>>, target: &mut T) {
        if self.current < self.actions.len() {
            self.actions.truncate(self.current);
        }
        action.apply(target);
        self.actions.push(action);
        self.current += 1;
    }

    pub fn undo(&mut self, target: &mut T)  -> Option<R> {
        if self.current <= 0 {return None;}
        self.current -= 1;
        Some(self.actions[self.current].undo(target))
    }

    pub fn redo(&mut self, target: &mut T)  -> Option<R> {
        if self.current >= self.actions.len() {return None;}
        let ret = self.actions[self.current].redo(target);
        self.current += 1;
        Some(ret)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Location {
    pub cur_view: usize,
    pub cur_view_group: String,
}

#[derive(Debug, Clone)]
pub struct MoveLocationAction {
    pub old_location: Location,
    pub new_location: Location,
}

impl Action for MoveLocationAction {
    type Target = Location;
    type Output = ();

    fn redo(&mut self, target: &mut Location) {
        *target = self.new_location.clone();
    }

    fn undo(&mut self, target: &mut Location)  {
        *target = self.old_location.clone();
    }
}


pub struct AddAction {
    pub table_name: String,
    pub data: HashMap<String, String>,
    pub old_idx: i32,
    pub cur_master_val: String,
}

impl AddAction {
    pub fn new(target: &HashMap<String, DataTable>, table_name: &str, data: HashMap<String, String>) -> Option<DataAction> {
        let table = target.get(table_name);
        if table.is_none() {return None;}
        let table = table.unwrap();
        let cur_master_val = data.get(&table.key_name);
        if cur_master_val.is_none() {return None;}
        let cur_master_val = cur_master_val.unwrap().clone();

        Some(Box::new(Self {
            table_name: table_name.to_string(),
            data,
            old_idx: 0,
            cur_master_val,
        }))
    }
}

impl Action for AddAction {
    type Target = HashMap<String, DataTable>;
    type Output = String;

    fn redo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.push(self.data.clone());
        self.old_idx = table.cur_row;
        table.cur_row = table.data.len() as i32 - 1;
        return format!("重做:添加第{}行", table.data.len());
    }

    fn undo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.pop();
        table.cur_row = self.old_idx;
        return format!("撤销:添加第{}行", table.data.len() + 1);
    }
}

pub struct DelAction {
    pub table_name: String,
    pub row_idx: usize,
    pub next_idx: usize,
    pub data: HashMap<String, String>,
}

impl Action for DelAction {
    type Target = HashMap<String, DataTable>;
    type Output = String;

    fn redo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.remove(self.row_idx);
        table.cur_row = self.next_idx as i32;
        return format!("重做:删除第{}行", self.row_idx + 1);
    }

    fn undo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.insert(self.row_idx, self.data.clone());
        table.cur_row = self.row_idx as i32;
        return format!("撤销:删除第{}行", self.row_idx + 1);
    }
}

impl DelAction {
    pub fn new(target: &HashMap<String, DataTable>, table_name: &str, row_idx: usize, next_idx: usize) -> Option<DataAction> {
        let table = target.get(table_name);
        if table.is_none() {return None;}
        let table = table.unwrap();
        let row = table.data.get(row_idx);
        if row.is_none() {return None;}
        let row = row.unwrap();
        let data = row.clone();

        Some(Box::new(Self {
            table_name: table_name.to_string(),
            row_idx,
            next_idx,
            data,
        }))
    }
}

pub struct UpdateAction {
    pub table_name: String,
    pub row_idx: usize,
    pub key: String,
    pub old: String,
    pub new: String,
}

impl UpdateAction {
    pub fn new(target: &HashMap<String, DataTable>, table_name: &str, row_idx: usize, key: &str, new: &str) -> Option<DataAction> {
        let table = target.get(table_name);
        if table.is_none() {return None;}
        let table = table.unwrap();
        let row = table.data.get(row_idx);
        if row.is_none() {return None;}
        let row = row.unwrap();
        let old = row.get(key);
        if old.is_none() {return None;}
        let old = old.unwrap().clone();

        Some(Box::new(Self {
            table_name: table_name.to_string(),
            row_idx,
            key: key.to_string(),
            old,
            new: new.to_string(),
        }))
    }
}

impl Action for UpdateAction {
    type Target = HashMap<String, DataTable>;
    type Output = String;

    fn redo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        let row = table.data.get_mut(self.row_idx);
        if row.is_none() {return String::new();}
        let row = row.unwrap();
        row.insert(self.key.clone(), self.new.clone());
        return format!("重做: 修改{}: {} -> {}", self.key, self.old, self.new);
    }

    fn undo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        let row = table.data.get_mut(self.row_idx);
        if row.is_none() {return String::new();}
        let row = row.unwrap();
        row.insert(self.key.clone(), self.old.clone());
        return format!("撤销: 修改{}: {} -> {}", self.key, self.new, self.old);
    }
}

pub struct ImportAction {
    pub table_name: String,
    pub data: Vec<HashMap<String, String>>,
    pub old: Vec<HashMap<String, String>>,
}

impl ImportAction {

    fn _new(target: &HashMap<String, DataTable>, path:PathBuf, tab:String) -> Result<Self> {
        let table = target.get(&tab);
        if table.is_none() {bail!(error::AppError::SheetNotFound(tab))};
        let table = table.unwrap();
        let mut workbook= calamine::open_workbook_auto(path)?;
        let range = workbook.worksheet_range(&tab)
            .ok_or(error::AppError::SheetNotFound(tab))??;

        let mut field_set = HashSet::new();
        for one in &table.info {
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
        if !flag {bail!(error::AppError::ImportExcelKeyNotFound(table.key_name.clone()))};

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
                let field_info = DataTable::get_field_by_name(&table.info, &title);
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
        let name = table.table_name.clone();
        let ret = Self {
            table_name: name.to_string(),
            data,
            old: table.data.clone(),
        };
        Ok(ret)
    }

    pub fn new(target: &HashMap<String, DataTable>, path:PathBuf, tab:String) -> Option<DataAction> {
        let ret = Self::_new(target, path, tab);
        if ret.is_err() {return None;}
        Some(Box::new(ret.unwrap()))
    }
}

impl Action for ImportAction {
    type Target = HashMap<String, DataTable>;
    type Output = String;

    fn redo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data = self.data.clone();
        return format!("撤销: 导入表{}", &self.table_name);
    }

    fn undo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data = self.old.clone();
        return format!("撤销: 导入表{}", &self.table_name);
    }
}

pub struct CopyAction {
    pub table_name: String,
    pub data: HashMap<String, String>,
    pub old_idx: i32,

    pub child: HashMap<String, Vec<HashMap<String, String>>>,
    pub cur_master_val: String,
}

impl CopyAction {
    pub fn new(target: &HashMap<String, DataTable>, table_name: &str, data: HashMap<String, String>, child: Vec<String>, copy_master_val:String) -> Option<DataAction> {
        let table = target.get(table_name);
        if table.is_none() {return None;}
        let table = table.unwrap();
        let cur_master_val = data.get(&table.key_name);
        if cur_master_val.is_none() {return None;}
        let cur_master_val = cur_master_val.unwrap().clone();

        let mut child_data = HashMap::new();
        for one in child{
            let data_table = target.get(&one);
            if data_table.is_none() {continue;}
            let data_table = data_table.unwrap();
            let copy_list = data_table.get_show_name_list(&data_table.master_field, &copy_master_val, false, &"".to_string());
            let mut data = Vec::new();
            for (_, one) in copy_list {
                for (_, two) in one {
                    for (_, idx, _, _) in two {
                        let row = data_table.copy_row(idx.clone() as usize, &cur_master_val, data.len() as i32);
                        if row.is_some() {data.push(row.unwrap());}
                    }
                }
            }
            child_data.insert(one, data);
        }
        

        Some(Box::new(Self {
            table_name: table_name.to_string(),
            data,
            old_idx: 0,
            child: child_data,
            cur_master_val,
        }))
    }
}

impl Action for CopyAction {
    type Target = HashMap<String, DataTable>;
    type Output = String;

    fn redo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        // 添加复制的行
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.push(self.data.clone());
        self.old_idx = table.cur_row;
        table.cur_row = table.data.len() as i32 - 1;

        // 添加子表复制的行
        for (k, v) in &self.child{
            let table = target.get_mut(k);
            if table.is_none() {continue;}
            let table = table.unwrap();
            for one in v {
                table.data.push(one.clone());
            }
            table.cur_row = table.data.len() as i32 - 1;
        }
        return format!("重做:复制表{}的行{}", self.table_name, self.cur_master_val);
    }

    fn undo(&mut self, target: &mut HashMap<String, DataTable>) -> String {
        // 删除复制的行
        let table = target.get_mut(&self.table_name);
        if table.is_none() {return String::new();}
        let table = table.unwrap();
        table.data.pop();
        table.cur_row = self.old_idx;

        // 删除子表复制的行
        for (k, v) in &self.child{
            let table = target.get_mut(k);
            if table.is_none() {continue;}
            let table = table.unwrap();
            for _ in 0..v.len() {
                table.data.pop();
            }
        }
        return format!("撤销:复制表{}的行{}", self.table_name, self.cur_master_val);
    }
}