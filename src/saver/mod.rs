use std::collections::HashMap;

use anyhow::Result;

use crate::data::data_field::FieldInfo;


pub trait DataSaver {
    fn output(info:&Vec<FieldInfo>, data:&Vec<HashMap<String, String>>, key:&String) -> Result<String>;
}

pub mod csv;
pub mod scsv;
pub mod json;