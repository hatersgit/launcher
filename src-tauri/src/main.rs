// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::{fs, path::Path, fs::File};
use std::io::Write;
use opendal::{services::Webdav, Operator};
use serde_json::Value;
use sysinfo::{System, SystemExt};
use tauri::Window;
use zip_extensions::zip_extract;

#[derive(Clone, serde::Serialize)]
struct Payload {
  message: String,
}

#[tauri::command]
async fn check_wow_running() -> bool {
    let s = System::new_all();
    if s.processes_by_name("wow.exe").count() > 0 {
        return true
    }
    false
}

#[tauri::command]
async fn check_file_version_and_download(payload: String, window: Window) -> String {
    let mut _emitter = window.emit("prog", Payload {message: "Connecting to fileserver...".to_string()}).unwrap();

    _emitter = window.emit("prog", Payload {message: "Unpacking settings payload...".to_string()}).unwrap();
    let p: Value = serde_json::from_str(&payload).unwrap();
    let wow_dir: &String = &p["wowDir"].to_string().replace("\\\\", "\\").replace("\"", "");

    _emitter = window.emit("prog", Payload {message: "Complete.".to_string()}).unwrap();

    println!("Received payload: {}", payload);

    let mut _emitter = window.emit("prog", Payload {message: "Connecting to fileserver...".to_string()}).unwrap();
    let mut builder = Webdav::default();

    builder
        .endpoint("http://65.109.163.164/")
        .root("webdav")
        .username("dluser")
        .password("stalwart.rider");

    let op = Operator::new(builder).unwrap().finish();
    _emitter = window.emit("prog", Payload {message: "Connected to fileserver".to_string()}).unwrap();

    let mut output_settings = format!("{{\"wowDir\":\"{}\",\"files\":[", wow_dir.replace("\\", "\\\\"));

    let mut lister = op.list("/").await.unwrap();
    let entries = lister.next_page().await.unwrap().unwrap();
    for i in 0..entries.len() {
        let entry = &entries[i];
        let files: &Vec<Value> = p["files"].as_array().unwrap();
        for j in 0..files.len() {
            _emitter = window.emit("prog", Payload {message: format!("Verifying {}", entry.name())}).unwrap();
            let filename = files[j]["name"].to_string().replace("\"", "");
            let modified = files[j]["date"].to_string().replace("\"", "");
            if filename.eq(entry.name()) {
                let meta = op.stat(&filename).await.unwrap().last_modified().unwrap().to_string();
                if modified != meta {
                    let mut dl_dir = "\\Interface\\AddOns\\";
                    if filename.contains(".mpq") {
                        dl_dir = "\\Data\\";
                    }

                    let target_file: &String = &format!("{}{}{}",wow_dir,dl_dir,filename);
                    _emitter = window.emit("prog", Payload {message: format!("Downloading new {}...", entry.name())}).unwrap();
                    let dl = op.read(&filename).await.unwrap();
                    println!("Making: {}", &target_file);
                    let mut newfile = fs::OpenOptions::new().create(true).write(true).open(target_file).unwrap();
                    let _write_result = newfile.write_all(&dl).unwrap();
                    _emitter = window.emit("prog", Payload {message: format!("Downloaded {}.", entry.name())}).unwrap();

                    if filename.contains(".zip") {
                        let target_dir = format!("{}{}",wow_dir,dl_dir);
                        if !Path::new(&target_dir).exists() {
                            let _created = fs::create_dir(&target_dir).unwrap();
                        }

                        _emitter = window.emit("prog", Payload {message: format!("Unzipping {} to {}", filename, target_dir)}).unwrap();
                        println!("Unzipping {} to {}", target_file, target_dir);
                        let tar_buf = PathBuf::from_str(&target_dir).unwrap();
                        let zip_buf = PathBuf::from_str(&target_file).unwrap();
                        let _ = zip_extract(&zip_buf, &tar_buf);
                        _emitter = window.emit("prog", Payload {message: format!("Unzip complete, deleting zip.")}).unwrap();

                        if Path::new(&target_file).exists() {
                            let _clean_zip = fs::remove_file(target_file).unwrap();
                        }
                    }

                    output_settings = format!("{}{{\"name\":\"{}\",\"date\":\"{}\"}},",output_settings, filename, meta);
                } else {
                    output_settings = format!("{}{{\"name\":\"{}\",\"date\":\"{}\"}},",output_settings, filename, modified);
                }
            }
        }
    }
    _emitter = window.emit("prog", Payload {message: "Patching complete.".to_string()}).unwrap();
    output_settings = format!("{}]}}", output_settings.strip_suffix(|_: char| true).unwrap());

    output_settings

}

#[tauri::command(rename_all = "snake_case")]
async fn start_wow(wow_exe: String) -> () {
    let process = Command::new(wow_exe).spawn().expect("Failed to start wow.exe");
}

#[tauri::command(rename_all = "snake_case")]
fn set_realmlist(realm_path: String, realm_info: String) -> bool {
    let format = &format!("Writing {} to {}", realm_info, realm_path);
    println!("{}",format);
    let _written = fs::write(realm_path, realm_info).unwrap();
    true
}

#[tauri::command(rename_all = "snake_case")]
fn exists(dir: String) -> bool {
    std::path::Path::new(&dir).exists()
}

#[tauri::command]
fn create_file(path: String, content: String) -> bool {
    println!("Creating {}", path);
    let mut file = File::create(&path).expect("Error writing file!");
    let written = file.write(content.as_bytes()).unwrap();
    written > 0
}

#[tauri::command(rename_all = "snake_case")]
fn create_dir(path: String) -> bool {
    let mut file = Path::new(&path).exists();
    if !file {
        let _created = fs::create_dir_all(&path).expect("Unable to create dir.");
        file = Path::new(&path).exists()
    }
    file
}

#[tauri::command(rename_all = "snake_case")]
fn read_settings(path: String) -> String {
    fs::read_to_string(&path).expect("Unable to read to json.")
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![check_file_version_and_download, start_wow, 
            set_realmlist, exists, create_file, create_dir, read_settings, check_wow_running])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
