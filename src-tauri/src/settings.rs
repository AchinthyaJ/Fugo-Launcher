use crate::models::LauncherSettings;
use std::fs;
use std::path::PathBuf;

pub fn get_app_dir() -> PathBuf {
    if let Some(dir) = dirs::data_dir() {
        let app_dir = dir.join("fugo-launcher");
        let _ = fs::create_dir_all(&app_dir);
        app_dir
    } else {
        PathBuf::from(".fugo-launcher")
    }
}

pub fn get_default_horizon_paths() -> (PathBuf, PathBuf) {
    // Current workspace root is typically parent of launcher/
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo_root = if current_dir.ends_with("launcher") || current_dir.ends_with("src-tauri") {
        current_dir
            .ancestors()
            .find(|p| p.join("crates/horizon-client").exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| current_dir.clone())
    } else {
        current_dir.clone()
    };

    let resources = repo_root.join(".local/horizon-client/26.3");
    let binary = if repo_root.join("target/fast/horizon-client").exists() {
        repo_root.join("target/fast/horizon-client")
    } else if repo_root.join("target/release/horizon-client").exists() {
        repo_root.join("target/release/horizon-client")
    } else {
        repo_root.join("target/fast/horizon-client")
    };

    (resources, binary)
}

pub fn load_settings() -> LauncherSettings {
    let settings_file = get_app_dir().join("settings.json");
    if let Ok(data) = fs::read_to_string(&settings_file) {
        if let Ok(settings) = serde_json::from_str::<LauncherSettings>(&data) {
            return settings;
        }
    }

    let (res_path, bin_path) = get_default_horizon_paths();
    let default = LauncherSettings {
        default_ram_mb: 4096,
        discrete_gpu: true,
        horizon_resources_path: res_path.to_string_lossy().to_string(),
        horizon_binary_path: bin_path.to_string_lossy().to_string(),
        custom_java_path: None,
        close_on_launch: false,
        theme: "obsidian".to_string(),
    };
    let _ = save_settings(&default);
    default
}

pub fn save_settings(settings: &LauncherSettings) -> Result<(), String> {
    let settings_file = get_app_dir().join("settings.json");
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(settings_file, json).map_err(|e| e.to_string())
}
