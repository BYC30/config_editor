use std::collections::HashMap;
use anyhow::Result;
use itertools::Itertools;
use crate::{data_field::{FieldInfo, EFieldType}, utils};

use super::DataSaver;


pub struct ScsvSaver {}

impl DataSaver for ScsvSaver  {
    fn output(info:&Vec<FieldInfo>, data:&Vec<HashMap<String, String>>, key:&String) -> Result<String> {
        let out_type = "scsv";
        let mut content = String::new();
        // 表头
        let mut header:Vec<Vec<String>> = vec![Vec::new(), Vec::new()];
        for one in info {
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
        for row in data.iter().sorted_by_key(|a|{utils::map_get_i32(*a, key)}) {
            let mut one_line = Vec::new();
            for one in info {
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

        Ok(content)
    }
}