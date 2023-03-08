use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::data::data_field::FieldInfo;


pub trait DataSaver {
    fn output(
        info: &Vec<FieldInfo>, 
        data: &Vec<HashMap<String, String>>, 
        key: &String,
        table_name: &String,
        writer: PathBuf,
    ) -> Result<()>;
}

pub mod csv;
pub mod scsv;
pub mod json;
pub mod excel;