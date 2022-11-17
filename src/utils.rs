use std::{path::PathBuf, collections::HashMap};

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

pub fn map_get_i32(map:&HashMap<String, String>, key:&String) -> i32 {
    let mut ret = 0;
    let v = map.get(key);
    if v.is_some() {
        let v = v.unwrap();
        let v = v.parse::<i32>();
        if v.is_ok() {
            ret = v.unwrap();
        }
    }
    return ret;
}

pub fn map_get_string(map:&HashMap<String, String>, key:&str, default:&str) -> String {
    let mut ret = default.to_string();
    let v = map.get(key);
    if v.is_some() {
        ret = v.unwrap().clone();
    }
    return ret;
}

pub fn map_contains_str(map:&HashMap<String, String>, search:&String) -> bool {
    for (_k, v) in map {
        if v.contains(search) {return true;}
    }
    return false;
}

pub fn translate_key(key: &str) -> Option<eframe::egui::Key> {
    use eframe::egui::Key;

    match key {
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "ArrowUp" => Some(Key::ArrowUp),

        "Esc" | "Escape" => Some(Key::Escape),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        "Enter" => Some(Key::Enter),
        "Space" | " " => Some(Key::Space),

        "Help" | "Insert" => Some(Key::Insert),
        "Delete" => Some(Key::Delete),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),

        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),

        "a" | "A" => Some(Key::A),
        "b" | "B" => Some(Key::B),
        "c" | "C" => Some(Key::C),
        "d" | "D" => Some(Key::D),
        "e" | "E" => Some(Key::E),
        "f" | "F" => Some(Key::F),
        "g" | "G" => Some(Key::G),
        "h" | "H" => Some(Key::H),
        "i" | "I" => Some(Key::I),
        "j" | "J" => Some(Key::J),
        "k" | "K" => Some(Key::K),
        "l" | "L" => Some(Key::L),
        "m" | "M" => Some(Key::M),
        "n" | "N" => Some(Key::N),
        "o" | "O" => Some(Key::O),
        "p" | "P" => Some(Key::P),
        "q" | "Q" => Some(Key::Q),
        "r" | "R" => Some(Key::R),
        "s" | "S" => Some(Key::S),
        "t" | "T" => Some(Key::T),
        "u" | "U" => Some(Key::U),
        "v" | "V" => Some(Key::V),
        "w" | "W" => Some(Key::W),
        "x" | "X" => Some(Key::X),
        "y" | "Y" => Some(Key::Y),
        "z" | "Z" => Some(Key::Z),

        _ => None,
    }
}

