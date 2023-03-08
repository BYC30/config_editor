use std::{collections::HashMap, path::PathBuf};
use anyhow::Result;
use itertools::Itertools;

use crate::{data::data_field::FieldInfo, utils};

use super::DataSaver;

pub struct ExcelSaver {}

impl DataSaver for ExcelSaver  {
    fn output(
        info: &Vec<FieldInfo>, 
        data: &Vec<HashMap<String, String>>, 
        key: &String,
        table_name: &String,
        path: PathBuf,
        all: bool,
    ) -> Result<()>{
        let mut book = utils::read_or_create_excel(&path);

        // 删除旧表
        let _ = book.remove_sheet_by_name(table_name);
        let sheet = book.new_sheet(table_name);
        if sheet.is_err() {return Err(anyhow::anyhow!(format!("creat sheet[{}] failed: {:?}", table_name, sheet)));}
        let sheet = sheet.unwrap();

        // 表头
        let mut max_col = 0;
        for one in info {
            if !one.export && !all {continue;}
            max_col = max_col + 1;
            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&max_col, &1);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.title);

            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&max_col, &2);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.origin);


            let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&max_col, &3);
            let cell = sheet.get_cell_mut(&cell_name);
            cell.set_value(&one.name);
        }

        // 内容
        let mut row = 3;
        for one in data.iter().sorted_by_key(|a|{utils::map_get_i32(*a, key)}) {
            row = row + 1;
            let mut col = 0;
            for field in info {
                if !field.export && !all {continue;}
                col = col + 1;

                let v = match one.get(&field.name){
                    Some(s) => {s.clone()},
                    None => {String::new()},
                };
                let cell_name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &row);
                let cell = sheet.get_cell_mut(cell_name.as_str());
                cell.set_value(&v);
            }
        }
        umya_spreadsheet::writer::xlsx::write(&book, path)?;
        Ok(())
    }
}