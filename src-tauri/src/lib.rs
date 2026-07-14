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

/// Try to read the first .mmd/.mermaid file passed via command-line arguments
/// and emit a `file-opened` event with its contents.
fn try_emit_cli_file(window: &tauri::WebviewWindow) {
    if let Some(file_path) = std::env::args().nth(1) {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let _ = window.emit(
                "file-opened",
                serde_json::json!({
                    "path": file_path,
                    "content": content
                }),
            );
        }
    }
}

/// Listen for file-open events (e.g. double-clicking .mmd in Explorer, or
/// dragging a .mmd file onto the app window). When the app receives a file
/// path via deep link, command line, or drag-drop, we emit the file content
/// to the frontend via the Tauri event system.
fn setup_file_open_handler(app: &tauri::App) {
    let window = app.get_webview_window("main").unwrap();
    let window_clone = window.clone();

    // Handle file paths passed via command-line arguments on startup.
    try_emit_cli_file(&window);

    // Listen for drag-and-drop events on the main window. Tauri 2 exposes
    // drag-drop events via the window event system; we filter for paths
    // pointing to a .mmd or .mermaid file and emit `file-opened`.
    let window_for_drag = window_clone.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop {
            paths, ..
        }) = event
        {
            for path in paths {
                let path_buf = PathBuf::from(path);
                // Check the file extension; only .mmd / .mermaid are supported.
                let is_mermaid = path_buf
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| {
                        let lower = e.to_lowercase();
                        lower == "mmd" || lower == "mermaid"
                    })
                    .unwrap_or(false);
                if !is_mermaid {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&path_buf) {
                    let _ = window_for_drag.emit(
                        "file-opened",
                        serde_json::json!({
                            "path": path_buf.to_string_lossy(),
                            "content": content
                        }),
                    );
                    // Only handle the first valid file in the drop.
                    break;
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            setup_file_open_handler(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_mmd_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
