#[path = "strict-dad.rs"]
mod strict_dad;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_apps() -> Result<Vec<strict_dad::BlockableApp>, String> {
    strict_dad::get_apps()
}

#[tauri::command]
fn load_sessions() -> Result<Vec<strict_dad::Session>, String> {
    strict_dad::load_sessions()
}

#[tauri::command]
fn save_sessions(sessions: Vec<strict_dad::Session>) -> Result<(), String> {
    strict_dad::save_sessions(sessions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_apps,
            load_sessions,
            save_sessions
        ])
        .setup(|_| {
            strict_dad::start_blocking_monitor();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
