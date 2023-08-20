// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fmt::format;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::{fs, path::Path, fs::File};
use std::io::Write;
use intptr::IntPtr32;
use libgit2_sys::git_annotated_commit_ref;
use opendal::{services::Webdav, Operator};
use serde_json::Value;
use tauri::{Window, async_runtime};
use tauri::api::file;
use zip_extensions::zip_extract;
use git2::Repository;

#[derive(Clone, serde::Serialize)]
struct Payload {
  message: String,
}

struct SFILE_CREATE_MPQ {
    cbSize: u16,
    dwMpqVersion: u16,
    pvUserData: IntPtr32,
    cbUserData: u16,
    dwStreamFlags: u16,
    dwFileFlags1: u16,
    dwFileFlags2: u16,
    dwAttrFlags: u16,
    dwSectorSize: u16,
    dwRawChunkSize: u16,
    dwMaxFileCount: u16
}

impl SFILE_CREATE_MPQ {
    fn new () -> Self {
        SFILE_CREATE_MPQ { cbSize: 0, dwMpqVersion: 0, pvUserData: IntPtr32::NULL, cbUserData: 0, dwStreamFlags: 0, dwFileFlags1: 0
            , dwFileFlags2: 0, dwAttrFlags: 0, dwSectorSize: 0, dwRawChunkSize: 0, dwMaxFileCount: 0 }
    }
}

async fn package_mpq(wow_dir: String, window: &Window) -> bool {
    println!("Packaging mpq to {}", wow_dir);
    unsafe {
        let path = "StormLibSharp.dll";
        let lib = libloading::Library::new(path).expect("Failed to load stormlib for MPQ creation");
        let func: libloading::Symbol<unsafe extern fn(name: String, create_flag: u8, file_count: u32, handle: SFILE_CREATE_MPQ) 
            -> bool> = lib.get(b"SFileCreateArchive").unwrap();

        let new_mpq: SFILE_CREATE_MPQ = SFILE_CREATE_MPQ::new();
        func("Patch-I.mpq".to_string(), 0, std::u32::MAX, new_mpq)
    }
}

#[tauri::command]
async fn check_file_version_and_download(payload: String, window: Window) -> bool {
    let borrow_win = &window;
    let mut _emitter = window.emit("prog", Payload {message: "Connecting to fileserver...".to_string()}).unwrap();

    _emitter = window.emit("prog", Payload {message: "Unpacking settings payload...".to_string()}).unwrap();
    let p: Value = serde_json::from_str(&payload).unwrap();
    let wow_dir: String = p["wowDir"].to_string().replace("\\\\", "\\").replace("\"", "");

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



    let mut lister = op.list("/").await.unwrap();
    let entries = lister.next_page().await.unwrap().unwrap();
    let mut patch_result = false;
    for i in 0..entries.len() {
        let entry = &entries[i];
        let files: &Vec<Value> = p["files"].as_array().unwrap();
        for j in 0..files.len() {
            _emitter = window.emit("prog", Payload {message: format!("Verifying {}", entry.name())}).unwrap();
            let filename = files[j]["name"].to_string().replace("\"", "");
            let modified = files[j]["date"].to_string();
            if filename.eq(entry.name()) {
                let meta = op.stat(&filename).await.unwrap().last_modified().unwrap().to_string();
                if modified != meta {
                    let mut dl_dir = "\\Interface\\AddOns\\";
                    if filename.contains(".dbc") {
                        dl_dir = "\\launcherpatch\\";
                        patch_result = true;

                        let dbc_path = format!("{}{}", wow_dir, dl_dir);
                        if !Path::new(&dbc_path).exists() {
                            fs::create_dir(&dbc_path).unwrap();
                        }
                    }
                    
                    let target_file: &String = &format!("{}{}{}",wow_dir,dl_dir,filename);
                    _emitter = window.emit("prog", Payload {message: format!("Downloading new {}...", entry.name())}).unwrap();
                    let dl = op.read(&filename).await.unwrap();
                    println!("Making: {}", &target_file);
                    let mut newfile = fs::OpenOptions::new().create(true).write(true).open(target_file).unwrap();
                    let _write_result = newfile.write_all(&dl).unwrap();
                    _emitter = window.emit("prog", Payload {message: format!("Downloaded {}.", entry.name())}).unwrap();

                    if filename.contains(".zip") {
                        let target_dir = format!("{}{}{}",wow_dir,dl_dir,&filename.replace(".zip", ""));
                        if !Path::new(&target_dir).exists() {
                            let _created = fs::create_dir(&target_dir).unwrap();
                        }

                        _emitter = window.emit("prog", Payload {message: format!("Unzipping {}...", filename)}).unwrap();
                        println!("Unzipping {} to {}", target_file, target_dir);
                        let tar_buf = PathBuf::from_str(&target_dir).unwrap();
                        let zip_buf = PathBuf::from_str(&target_file).unwrap();
                        let _ = zip_extract(&zip_buf, &tar_buf);
                        _emitter = window.emit("prog", Payload {message: format!("Unzip complete, deleting zip.")}).unwrap();

                        if Path::new(&target_file).exists() {
                            let _clean_zip = fs::remove_file(target_file).unwrap();
                        }
                    }
                }
            } 
        }
    }

    if patch_result {
        _emitter = window.emit("prog", Payload {message: "Patch changes detected, repackaging MPQ...".to_string()}).unwrap();
        println!("Patch files changed, repackaging!");
        let _package = package_mpq(wow_dir, borrow_win).await;
        println!("Patch packaged! {}", _package);
    }

    // let filename = format!("{}{}","./",file_name);

    // let meta = op.stat(&filename);

    // let date: String = meta.await.unwrap().last_modified().unwrap().to_string();

    // let target_file = &format!("{}{}",target_dir, file_name);
    // let file_exists = Path::new(&target_file.replace(".zip", "")).exists();
    // println!("Stored date: {} Web Date: {}", saved_date, date);
    // if saved_date != date || !file_exists {
    //     println!("New modification found on server with date: {}", date);
    //     let dl = op.read(&filename).await.unwrap();
    //     println!("{}", target_file);

    //     let mut newfile = fs::OpenOptions::new().create(true).write(true).open(target_file).unwrap();
    //     let _write_result = newfile.write_all(&dl).unwrap();

    //     

    //     date
    // } else {
    //     saved_date.to_string()
    // }
    true
}

#[tauri::command(rename_all = "snake_case")]
async fn start_wow(wow_exe: String) -> () {
    Command::new(wow_exe).output().expect("Failed to start wow.exe");
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
            set_realmlist, exists, create_file, create_dir, read_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
