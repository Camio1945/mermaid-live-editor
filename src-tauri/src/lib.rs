use std::fs;
use std::path::PathBuf;
use tauri::Emitter;
use tauri::Manager;

/// Read the contents of a .mmd file from the given path.
#[tauri::command]
fn read_mmd_file(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }
    fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Listen for file-open events (e.g. double-clicking .mmd in Explorer).
/// When the app receives a file path via deep link or command line,
/// we emit the file content to the frontend via the Tauri event system.
fn setup_file_open_handler(app: &tauri::App) {
    let window = app.get_webview_window("main").unwrap();
    let window_clone = window.clone();

    // Handle file paths passed via command-line arguments on startup
    // or when the app is already running (single-instance)
    #[cfg(target_os = "windows")]
    {
        // On Windows, check if there's a file path argument
        if let Some(file_path) = std::env::args().nth(1) {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let _ = window_clone.emit("file-opened", serde_json::json!({
                    "path": file_path,
                    "content": content
                }));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(file_path) = std::env::args().nth(1) {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let _ = window_clone.emit("file-opened", serde_json::json!({
                    "path": file_path,
                    "content": content
                }));
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_file_open_handler(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_mmd_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
