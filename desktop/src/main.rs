#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{State, WebviewUrl, WebviewWindowBuilder, ipc::Response};

struct OpenedFiles(Mutex<Vec<PathBuf>>);

#[tauri::command]
fn opened_files(opened: State<OpenedFiles>) -> Vec<PathBuf> {
    opened.0.lock().unwrap().clone()
}

#[tauri::command]
fn read_opened_file(path: PathBuf, opened: State<OpenedFiles>) -> Result<Response, String> {
    let mut pending = opened.0.lock().unwrap();
    let index = pending.iter().position(|opened| *opened == path).ok_or("file was not opened")?;
    pending.remove(index);
    drop(pending);
    fs::read(&path).map(Response::new).map_err(|error| error.to_string())
}

fn main() {
    let launched = std::env::args_os().skip(1).map(PathBuf::from).filter(|path| path.is_file()).collect();
    tauri::Builder::default()
        .manage(OpenedFiles(Mutex::new(launched)))
        .invoke_handler(tauri::generate_handler![opened_files, read_opened_file])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Tintenfisch")
                .inner_size(1280.0, 860.0)
                .min_inner_size(640.0, 480.0)
                .disable_drag_drop_handler()
                .on_download(|_, _| true)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Tintenfisch")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = _event {
                use tauri::{Emitter, Manager};
                let paths = urls.into_iter().filter_map(|url| url.to_file_path().ok());
                _app.state::<OpenedFiles>().0.lock().unwrap().extend(paths);
                let _ = _app.emit("opened-files", ());
            }
        });
}
