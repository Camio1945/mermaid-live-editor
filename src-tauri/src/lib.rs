use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

/// Stores the CLI file path/content so the frontend can retrieve it
/// via IPC after it has registered its event listeners.
struct CliFileState {
    payload: Option<serde_json::Value>,
}

/// Read the contents of a .mmd file from the given path.
#[tauri::command]
fn read_mmd_file(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }
    fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Result item for list_mmd_files: a sibling .mmd file in the same folder.
#[derive(serde::Serialize)]
struct MmdFileEntry {
    name: String,
    path: String,
    stem: String,
}

/// List all .mmd/.mermaid files in the same directory as the given file path.
#[tauri::command]
fn list_mmd_files(path: String) -> Result<Vec<MmdFileEntry>, String> {
    let file_path = PathBuf::from(&path);
    let dir = file_path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory: {}", path))?;

    let mut entries: Vec<MmdFileEntry> = Vec::new();
    let read_dir = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        if !has_mermaid_extension(&entry_path) {
            continue;
        }
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let stem = entry_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        entries.push(MmdFileEntry {
            path: entry_path.to_string_lossy().to_string(),
            name,
            stem,
        });
    }

    // Sort by name for a stable, predictable order
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// IPC command: returns the CLI file payload if one was passed on startup,
/// then clears it so it's only consumed once.
#[tauri::command]
fn get_cli_file(state: tauri::State<'_, Mutex<CliFileState>>) -> Option<serde_json::Value> {
    let mut guard = state.lock().unwrap();
    guard.payload.take()
}

/// Reveal the given file in the operating system's file manager,
/// selecting (highlighting) the file.
#[tauri::command]
fn reveal_in_explorer(path: String) -> Result<(), String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }
    open_in_file_manager(&file_path)
}

/// Platform-specific: open the parent directory in the file manager
/// and select (highlight) the given file.
fn open_in_file_manager(file_path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg("/select,")
            .arg(file_path.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(file_path.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }

    #[cfg(all(
        target_os = "linux",
        not(target_os = "macos"),
        not(target_os = "windows")
    ))]
    {
        if let Some(parent) = file_path.parent() {
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to open file manager: {}", e))?;
        }
    }

    Ok(())
}

/// Read the first .mmd/.mermaid file passed via command-line arguments and
/// store it in managed state so the frontend can retrieve it via `get_cli_file`.
fn capture_cli_file(state: &Mutex<CliFileState>) {
    if let Some(file_path) = std::env::args().nth(1) {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let mut guard = state.lock().unwrap();
            guard.payload = Some(serde_json::json!({
                "path": file_path,
                "content": content
            }));
        }
    }
}

/// Emit the file-opened event for a CLI-provided file path.
fn emit_cli_file(window: &tauri::WebviewWindow) {
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

/// Check whether a path has a .mmd or .mermaid extension.
fn has_mermaid_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_lowercase();
            lower == "mmd" || lower == "mermaid"
        })
        .unwrap_or(false)
}

/// Process drag-and-drop paths: emit file-opened for the first valid mermaid file.
fn handle_drag_drop(window: &tauri::WebviewWindow, paths: &[std::path::PathBuf]) {
    for path in paths {
        if !has_mermaid_extension(path) {
            continue;
        }
        if let Ok(content) = fs::read_to_string(path) {
            let _ = window.emit(
                "file-opened",
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "content": content
                }),
            );
            break; // Only handle the first valid file
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

    // Store CLI file in managed state so the frontend can retrieve it after
    // its event listeners are registered (avoids race condition).
    let state = app.state::<Mutex<CliFileState>>();
    capture_cli_file(&state);

    // Also try to emit immediately for cases where the frontend listener
    // might already be registered (e.g. hot-reload during dev).
    emit_cli_file(&window);

    // Listen for drag-and-drop events on the main window.
    let window_for_drag = window_clone.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
            let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            handle_drag_drop(&window_for_drag, &path_bufs);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(CliFileState { payload: None }))
        .setup(|app| {
            setup_file_open_handler(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_mmd_file,
            get_cli_file,
            list_mmd_files,
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
