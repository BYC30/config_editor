use std::{collections::HashMap, process::Command};

use anyhow::Result;
use calamine::{DataType, Range};

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

pub fn tablestr2map(table:&String) -> Result<HashMap<String, String>>{
    let lua = mlua::Lua::new();
    let table: mlua::Table = lua.load(table).eval()?;
    let json = serde_json::to_string(&table)?;
    let map: HashMap<String, String> = serde_json::from_str(&json)?;
    return Ok(map);
}

pub fn map2tablestr(map:&HashMap<String, String>) -> Result<String> {
    let mut ret = Vec::new();
    for (k, v) in map {
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


pub fn hide_console_window() {
    use std::ptr;
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE};

    let window = unsafe {GetConsoleWindow()};
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    if window != ptr::null_mut() {
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }
}

pub fn show_console_window() {
    use std::ptr;
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_SHOW};

    let window = unsafe {GetConsoleWindow()};
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    if window != ptr::null_mut() {
        unsafe {
            ShowWindow(window, SW_SHOW);
        }
    }
}


pub fn exec_bat(path:&String) -> Result<()> {
    let mut current_exe = std::env::current_exe()?;
    current_exe.pop();
    current_exe.push(path.clone());

    let full_path = dunce::canonicalize(current_exe)?;
    let mut bat_dir_path = full_path.clone();
    bat_dir_path.pop();
    Command::new("cmd")
        .current_dir(bat_dir_path)
        .args(&["/C", full_path.to_str().unwrap()])
        .spawn()?;

    return Ok(());
}

pub fn ordered_map<S>(value: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let ordered: std::collections::BTreeMap<_, _> = value.iter().collect();
    serde::Serialize::serialize(&ordered, serializer)
}