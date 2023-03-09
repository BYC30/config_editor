use std::{collections::HashMap, path::PathBuf, process::Command};

use anyhow::{bail, Result};
use calamine::{DataType, Range};
use itertools::Itertools;

use serde_json::json;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use walkdir::WalkDir;

use crate::{
    data::data_field::EFieldType,
    error,
    marco::{check_if, check_some},
};

pub fn get_cell(range: &Range<DataType>, x: u32, y: u32) -> String {
    let one = range.get_value((x, y));
    if one.is_none() {
        return "".to_string();
    } else {
        return one.unwrap().to_string();
    }
}

pub fn msg(content: String, title: String) {
    rfd::MessageDialog::new()
        .set_title(title.as_str())
        .set_description(content.as_str())
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

pub fn toast(toast: &mut egui_notify::Toasts, icon: &str, msg: impl Into<String>) {
    match icon {
        "SHORT" => {
            toast
                .info(msg)
                .set_duration(Some(std::time::Duration::from_secs(2)));
        }
        "INFO" => {
            toast
                .info(msg)
                .set_duration(Some(std::time::Duration::from_secs(5)));
        }
        "ERRO" => {
            toast.error(msg).set_closable(true).set_duration(None);
        }
        "SUCC" => {
            toast
                .success(msg)
                .set_duration(Some(std::time::Duration::from_secs(5)));
        }
        _ => {
            toast
                .basic(msg)
                .set_duration(Some(std::time::Duration::from_secs(5)));
        }
    }
}

pub fn map_get_i32(map: &HashMap<String, String>, key: &String) -> i32 {
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

pub fn map_get_string(map: &HashMap<String, String>, key: &str, default: &str) -> String {
    let mut ret = default.to_string();
    let v = map.get(key);
    if v.is_some() {
        ret = v.unwrap().clone();
    }
    return ret;
}

pub fn map_contains_str(map: &HashMap<String, String>, search: &String) -> bool {
    for (_k, v) in map {
        if v.contains(search) {
            return true;
        }
    }
    return false;
}

pub fn tablestr2map(table: &String) -> Result<HashMap<String, String>> {
    let lua = mlua::Lua::new();
    let table: mlua::Table = lua.load(table).eval()?;
    let json = serde_json::to_string(&table)?;
    let map: HashMap<String, String> = serde_json::from_str(&json)?;
    return Ok(map);
}

pub fn map2tablestr(map: &HashMap<String, String>) -> Result<String> {
    let mut ret = Vec::new();
    for (k, v) in map.iter().sorted() {
        ret.push(format!("{}=\"{}\"", k, v.as_str()));
    }
    let s = format!("{{{}}}", ret.join(", "));
    return Ok(s);
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

// 显示隐藏控制台窗口
pub fn show_console_window(show: bool) {
    use std::ptr;
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE, SW_SHOW};

    let window = unsafe { GetConsoleWindow() };
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    if window != ptr::null_mut() {
        unsafe {
            if show {
                ShowWindow(window, SW_SHOW);
            } else {
                ShowWindow(window, SW_HIDE);
            }
        }
    }
}

pub fn exec_bat(path: &String) -> Result<()> {
    let mut current_exe = std::env::current_exe()?;
    current_exe.pop();
    current_exe.push(path);

    let full_path = dunce::canonicalize(current_exe)?;
    let mut bat_dir_path = full_path.clone();
    bat_dir_path.pop();

    let bat_path = full_path
        .to_str()
        .ok_or(anyhow::anyhow!("Cannot convert path to string"))?;

    Command::new("cmd")
        .current_dir(bat_dir_path)
        .args(&["/C", bat_path])
        .spawn()?;

    Ok(())
}

pub fn ordered_map<S>(value: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let ordered: std::collections::BTreeMap<_, _> = value.iter().collect();
    serde::Serialize::serialize(&ordered, serializer)
}

fn get_cell_value(sheet: &Worksheet, col: u32, row: u32) -> String {
    let name = umya_spreadsheet::helper::coordinate::coordinate_from_index(&col, &row);
    let cell = sheet.get_cell(&name);
    let cell = match cell {
        Some(c) => c,
        None => return String::new(),
    };

    let value = cell.get_value();
    return value.to_string();
}

pub fn parse_data_type(field_type: &String) -> Result<(bool, bool, EFieldType, String)> {
    let mut tmp = field_type.clone();
    let mut prefix = String::new();
    let arr: Vec<&str> = tmp.split("<").collect();
    if arr.len() == 2 {
        prefix = arr[0].to_string();
        tmp = arr[1].to_string();
    } else {
        tmp = arr[0].to_string();
    }
    let mut suffix = String::new();
    let arr: Vec<&str> = tmp.split(">").collect();
    let field = arr[0];
    let data_type = match field {
        "B" => EFieldType::Bool,
        "N" => EFieldType::Number,
        "S" => EFieldType::Str,
        "E" => EFieldType::Expr,
        "M" => EFieldType::Table,
        _ => {
            bail!(error::AppError::FieldTypeNotSupport(field.to_string()));
        }
    };
    if arr.len() == 2 {
        suffix = arr[1].to_string();
    }

    let mut is_key = false;
    if prefix == "K" {
        is_key = true;
    }
    let mut is_array = false;
    if prefix == "A" {
        is_array = true;
    }
    return Ok((is_key, is_array, data_type, suffix));
}

fn parse_one_data(data: &String, data_type: &EFieldType) -> Result<serde_json::Value> {
    let ret = match data_type {
        EFieldType::Bool => {
            json!(data.to_lowercase() == "true")
        }
        EFieldType::Number => {
            let mut ret = json!(0);
            if data.len() > 0 {
                let v = data.parse::<i32>()?;
                ret = json!(v);
            }
            ret
        }
        EFieldType::Str | EFieldType::Expr => {
            json!(data)
        }
        EFieldType::Table => {
            if data.is_empty() {
                json!({})
            } else {
                let lua = mlua::Lua::new();
                let table: mlua::Table = lua.load(data).eval()?;
                let json = serde_json::to_string(&table)?;
                serde_json::from_str(&json)?
            }
        }
    };
    return Ok(ret);
}

pub fn load_one_cell(data: &String, data_type: &String) -> Result<serde_json::Value> {
    let (_, is_array, data_type, _) = parse_data_type(&data_type)?;
    if is_array {
        let mut ret = json!([]);
        if data.is_empty() {
            return Ok(ret);
        }
        let list = ret.as_array_mut().unwrap();
        let arr: Vec<&str> = data.split(";").collect();
        for v in arr {
            let v = v.trim();
            let v = parse_one_data(&v.to_string(), &data_type)?;
            list.push(v);
        }
        return Ok(ret);
    } else {
        return parse_one_data(&data, &data_type);
    }
}

pub fn load_excel_sheet<T>(path: &PathBuf, sheet_name: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let book = umya_spreadsheet::reader::xlsx::read(path.clone())?;
    let ret = book.get_sheet_by_name(sheet_name);
    let sheet = match ret {
        Ok(s) => s,
        Err(e) => {
            return Err(anyhow::anyhow!(format!("get_sheet_by_name failed: {}", e)));
        }
    };

    let (col, row) = sheet.get_highest_column_and_row();

    let mut ret = json!([]);
    let list = ret.as_array_mut().unwrap();
    for r in 4..row + 1 {
        let mut one = json!({});
        let map = one.as_object_mut().unwrap();
        for c in 1..col + 1 {
            let data_type = get_cell_value(&sheet, c, 2);
            let key = get_cell_value(&sheet, c, 3);
            let value = get_cell_value(&sheet, c, r);

            check_if!(key.is_empty(), continue);

            let val = load_one_cell(&value, &data_type)?;
            // let val = match load_one_cell(&value, &data_type) {
            //     Ok(v) => {v},
            //     Err(_) => {serde_json::from_str(&value)?},
            // };
            map.insert(key, val);
        }
        list.push(one);
    }
    let ret: T = serde_json::from_value(ret)?;
    return Ok(ret);
}

pub fn load_dir_excel_cfg<T>(p: &str, table_name: &str) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    let mut path = std::env::current_exe()?;
    path.pop();
    path.push(p);
    let mut ret = Vec::new();
    if !path.exists() {
        return Ok(ret);
    }

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let p = entry.path();

        check_if!(p.is_dir(), continue);
        let ext = check_some!(p.extension(), continue);
        check_if!(ext != "xlsx", continue);
        let name = check_some!(p.file_name(), continue);
        let name = check_some!(name.to_str(), continue);
        check_if!(name.starts_with("~$"), continue);

        let mut data: Vec<T> = load_excel_sheet(&p.to_path_buf(), table_name)?;
        ret.append(&mut data);
    }
    return Ok(ret);
}

pub fn load_excel2map(path: &PathBuf, sheet_name: &str) -> Result<Vec<HashMap<String, String>>> {
    let book = umya_spreadsheet::reader::xlsx::read(path.clone())?;
    let ret = book.get_sheet_by_name(sheet_name);
    let sheet = match ret {
        Ok(s) => s,
        Err(e) => {
            return Err(anyhow::anyhow!(format!("get_sheet_by_name failed: {}", e)));
        }
    };

    let (col, row) = sheet.get_highest_column_and_row();
    let mut list = Vec::new();
    for r in 4..row + 1 {
        let mut map = HashMap::new();
        for c in 1..col + 1 {
            // let data_type = get_cell_value(&sheet, c, 2);
            let key = get_cell_value(&sheet, c, 3);
            let value = get_cell_value(&sheet, c, r);
            if key.is_empty() {
                continue;
            }
            map.insert(key, value);
        }
        list.push(map);
    }
    return Ok(list);
}

pub fn read_or_create_excel(path: &PathBuf) -> Spreadsheet {
    let book = umya_spreadsheet::reader::xlsx::read(path.clone());
    let book = match book {
        Ok(b) => b,
        Err(_e) => umya_spreadsheet::new_file(),
    };

    return book;
}
