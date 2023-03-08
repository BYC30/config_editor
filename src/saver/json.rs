use std::{collections::HashMap, path::PathBuf, fs};
use anyhow::Result;
use itertools::Itertools;
use serde_json::json;

use crate::{data::data_field::{FieldInfo, EFieldType}, utils};

use super::DataSaver;


pub struct JsonSaver {}

impl JsonSaver {
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

    pub fn get_one(field:&FieldInfo, data:&String) -> Result<serde_json::Value> {
        if field.is_array {
            let mut ret = json!([]);
            let list = ret.as_array_mut().unwrap();
            let mut arr:Vec<&str> = Vec::new();
            if !data.is_empty() {
                arr = data.split(";").collect();
            }

            for one in arr {
                let tmp = JsonSaver::parse_one(field, one)?;
                list.push(tmp);
            }

            return Ok(ret);
        }else{
            return JsonSaver::parse_one(field, data.as_str());
        }
    }
}

impl DataSaver for JsonSaver  {
    fn output(
        info: &Vec<FieldInfo>, 
        data: &Vec<HashMap<String, String>>, 
        key: &String,
        _table_name: &String,
        path: PathBuf,
        all: bool,
    ) -> Result<()>{
        let mut total = json!([]);
        let list = total.as_array_mut().unwrap();
        // 表头

        // 内容
        for row in data.iter().sorted_by_key(|a|{utils::map_get_i32(*a, key)}) {
            let mut one = json!({});
            let map = one.as_object_mut().unwrap();

            for field in info {
                if !field.export && !all {continue;}
                let v = match row.get(&field.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                
                let one_data = JsonSaver::get_one(field, &v)?;
                map.insert(field.name.clone(), one_data);
            }
            list.push(one);
        }

        let str = serde_json::to_string_pretty(&total)?;
        fs::write(path, str)?;
        Ok(())
    }
}