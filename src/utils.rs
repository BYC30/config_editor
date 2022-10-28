use std::path::PathBuf;

use anyhow::Result;
use calamine::{open_workbook_auto, Reader, DataType, Range};

use super::error;

pub fn open_excel(path: &String, tab: &str) -> Result<Range<DataType>> {
    println!("open_excel {:?} sheet {}", path, tab);
    let mut workbook= open_workbook_auto(path)?;
    
    let range = workbook.worksheet_range(tab)
        .ok_or(error::AppError::SheetNotFound(tab.to_string()))??;
    
    return Ok(range);
}

pub fn open_excel2(path: &PathBuf, tab: &str) -> Result<Range<DataType>> {
    println!("open_excel {:?} sheet {}", path, tab);
    let mut workbook= open_workbook_auto(path)?;
    
    let range = workbook.worksheet_range(tab)
        .ok_or(error::AppError::SheetNotFound(tab.to_string()))??;
    
    return Ok(range);
}

pub fn get_cell(range: &Range<DataType>, x: u32, y: u32) -> String {
    let one = range.get_value((x, y));
    if one.is_none() {
        return "".to_string();
    }else{
        return one.unwrap().to_string();
    }
}

pub fn msg(content:String, title:String){
    rfd::MessageDialog::new()
        .set_title(title.as_str())
        .set_description(content.as_str())
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}