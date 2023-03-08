use std::{collections::HashMap, path::PathBuf};
use anyhow::Result;
use itertools::Itertools;
use serde_json::json;

use crate::{data::data_field::{FieldInfo, EFieldType}, utils};

use super::DataSaver;

pub struct ExcelSaver {}

impl ExcelSaver {
    pub fn parse_one(field:&FieldInfo, data:&str) -> Result<serde_json::Value> {
        match field.val_type {
            EFieldType::Bool => {return Ok(serde_json::Value::Bool(data.to_lowercase() == "true"))},
            EFieldType::Number => {
                let num = data.parse::<f32>()?;
                return Ok(json!(num));
            },
            EFieldType::Str => { return Ok(json!(data)); },
            EFieldType::Expr => { return Ok(json!(data)); },
            EFieldType::Table => {
                let mut v = "{}";
                if !data.is_empty() {v = data;}
                let map = utils::tablestr2map(&v.to_string())?;
                return Ok(json!(map));
            }
        }
    }

    pub fn parse_only_one(field:&FieldInfo, data:&str) -> Result<String> {
        match field.val_type {
            EFieldType::Table => {
                let mut v = "{}";
                if !data.is_empty() {v = data;}
                let map = utils::tablestr2map(&v.to_string())?;
                return Ok(serde_json::to_string(&json!(map))?);
            }
            _ => {return Ok(data.to_string())}
        }
    }

    pub fn get_one(field:&FieldInfo, data:&String) -> Result<String> {
        if field.is_array {
            let mut ret = json!([]);
            let list = ret.as_array_mut().unwrap();
            let mut arr:Vec<&str> = Vec::new();
            if !data.is_empty() {
                arr = data.split(";").collect();
            }

            for one in arr {
                let tmp = ExcelSaver::parse_one(field, one)?;
                list.push(tmp);
            }

            return Ok(serde_json::to_string(&ret)?);
        }else{
            
            return ExcelSaver::parse_only_one(field, data.as_str());
        }
    }
}


impl DataSaver for ExcelSaver  {
    fn output(
        info: &Vec<FieldInfo>, 
        data: &Vec<HashMap<String, String>>, 
        key: &String,
        table_name: &String,
        path: PathBuf,
    ) -> Result<()>{
        let mut book = utils::read_or_create_excel(&path);
        
        let ret = book.get_sheet_by_name_mut(table_name);
        let sheet = match ret {
            Ok(s) => s,
            Err(e) => {
                let sheet = book.new_sheet(table_name);
                if sheet.is_err() {return Err(anyhow::anyhow!(format!("creat sheet failed: {}", e)));}
                sheet.unwrap()
            }
        };

        // 表头
        let mut col = 0;
        for one in info {
            if !one.export {continue;}
            col = col + 1;
            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &1);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.title);

            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &2);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.origin);


            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &3);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.name);
        }

        // 内容
        let mut row = 3;
        for one in data.iter().sorted_by_key(|a|{utils::map_get_i32(*a, key)}) {
            row = row + 1;
            let mut col = 0;
            for field in info {
                if !field.export {continue;}
                col = col + 1;

                let v = match one.get(&field.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                let one_data = ExcelSaver::get_one(field, &v)?;
                let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &row);
                let cell = sheet.get_cell_mut(cell_name.as_str());
                cell.set_value(&one_data);
            }
        }

        umya_spreadsheet::writer::xlsx::write(&book, path)?;
        Ok(())
    }
}