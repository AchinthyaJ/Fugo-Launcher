pub mod accounts;
pub mod instance_manager;
pub mod launcher_engine;
pub mod mc;
pub mod models;
pub mod modrinth;
pub mod players;
pub mod settings;

use models::*;
use std::path::PathBuf;
use tauri::{AppHandle, Window};

#[tauri::command]
fn get_instances() -> Vec<Instance> {
    instance_manager::load_instances()
}

#[tauri::command]
fn create_instance(req: CreateInstanceRequest) -> Result<Instance, String> {
    instance_manager::create_instance(req)
}

#[tauri::command]
fn update_instance(instance: Instance) -> Result<Instance, String> {
    instance_manager::update_instance(instance)
}

#[tauri::command]
fn delete_instance(id: String) -> Result<(), String> {
    instance_manager::delete_instance(&id)
}

#[tauri::command]
fn open_instance_folder(id: String) -> Result<(), String> {
    let instances = instance_manager::load_instances();
    if let Some(inst) = instances.iter().find(|i| i.id == id) {
        open::that(&inst.game_dir).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Instance not found".into())
    }
}

#[tauri::command]
fn pick_directory() -> Result<Option<String>, String> {
    instance_manager::pick_directory()
}

#[tauri::command]
fn pick_zip_file() -> Result<Option<String>, String> {
    instance_manager::pick_zip_file()
}

#[tauri::command]
fn import_instance_directory(source_path: String, edition: Option<String>) -> Result<Instance, String> {
    instance_manager::import_instance_directory(&source_path, edition.as_deref())
}

#[tauri::command]
fn import_instance_zip(zip_path: String, edition: Option<String>) -> Result<Instance, String> {
    instance_manager::import_instance_zip(&zip_path, edition.as_deref())
}

#[tauri::command]
fn list_instance_content(instance_id: String, subfolder: String) -> Result<Vec<instance_manager::InstanceContentItem>, String> {
    instance_manager::list_instance_content(&instance_id, &subfolder)
}

#[tauri::command]
fn toggle_instance_content(instance_id: String, subfolder: String, filename: String, enabled: bool) -> Result<(), String> {
    instance_manager::toggle_instance_content(&instance_id, &subfolder, &filename, enabled)
}

#[tauri::command]
fn delete_instance_content(instance_id: String, subfolder: String, filename: String) -> Result<(), String> {
    instance_manager::delete_instance_content(&instance_id, &subfolder, &filename)
}

#[tauri::command]
fn duplicate_instance(instance_id: String) -> Result<Instance, String> {
    instance_manager::duplicate_instance(&instance_id)
}

#[tauri::command]
fn launch_instance(app: AppHandle, instance_id: String) -> Result<String, String> {
    launcher_engine::launch_instance(app, &instance_id)
}

#[tauri::command]
fn kill_instance(instance_id: String) -> Result<(), String> {
    launcher_engine::kill_instance(&instance_id)
}

#[tauri::command]
fn get_running_instances() -> Vec<String> {
    launcher_engine::get_running_instances()
}

#[tauri::command]
fn list_instance_mods(instance_id: String) -> Result<Vec<InstalledMod>, String> {
    instance_manager::list_instance_mods(&instance_id)
}

#[tauri::command]
fn toggle_instance_mod(instance_id: String, filename: String, enabled: bool) -> Result<(), String> {
    instance_manager::toggle_instance_mod(&instance_id, &filename, enabled)
}

#[tauri::command]
fn delete_instance_mod(instance_id: String, filename: String) -> Result<(), String> {
    instance_manager::delete_instance_mod(&instance_id, &filename)
}

#[tauri::command]
async fn search_modrinth(
    query: String,
    loader: Option<String>,
    game_version: Option<String>,
    limit: Option<u32>,
    project_type: Option<String>,
    sort: Option<String>,
) -> Result<ModrinthSearchResult, String> {
    modrinth::search_modrinth(&query, loader.as_deref(), game_version.as_deref(), limit, project_type.as_deref(), sort.as_deref()).await
}

#[tauri::command]
async fn install_modrinth_modpack(project_id: String, version_id: Option<String>) -> Result<Instance, String> {
    modrinth::install_modrinth_modpack(&project_id, version_id.as_deref()).await
}

#[tauri::command]
async fn lookup_player(username: String) -> Result<players::PlayerTextures, String> {
    players::lookup(&username).await
}

#[tauri::command]
async fn list_loader_versions(loader: String, game_version: String) -> Result<Vec<String>, String> {
    let client = mc::http();
    if mc::forge::is_installer_loader(&loader) {
        mc::forge::list_versions(&client, &loader, &game_version).await
    } else {
        mc::loader::list_versions(&client, &loader, &game_version).await
    }
}

#[tauri::command]
async fn download_modrinth_mod(
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<InstalledMod, String> {
    modrinth::download_modrinth_mod(&instance_id, &project_id, version_id.as_deref()).await
}

#[tauri::command]
fn set_active_skin(skin_url: Option<String>, preset: Option<String>) -> Result<(), String> {
    accounts::set_active_skin(skin_url, preset)
}

#[tauri::command]
fn set_active_cape(cape: Option<String>, cape_data: Option<String>) -> Result<(), String> {
    accounts::set_active_cape(cape, cape_data)
}

#[tauri::command]
fn get_accounts() -> Vec<Account> {
    accounts::load_accounts()
}

#[tauri::command]
fn create_offline_account(
    username: String,
    skin: Option<String>,
    cape: Option<String>,
) -> Result<Account, String> {
    accounts::create_offline_account(&username, skin, cape)
}

#[tauri::command]
fn set_active_account(id: String) -> Result<(), String> {
    accounts::set_active_account(&id)
}

#[tauri::command]
fn delete_account(id: String) -> Result<(), String> {
    accounts::delete_account(&id)
}

#[tauri::command]
fn get_settings() -> LauncherSettings {
    settings::load_settings()
}

#[tauri::command]
fn save_settings(settings: LauncherSettings) -> Result<(), String> {
    settings::save_settings(&settings)
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let s = settings::load_settings();
    let bin_path = PathBuf::from(&s.horizon_binary_path);
    let res_path = PathBuf::from(&s.horizon_resources_path);

    SystemInfo {
        os: std::env::consts::OS.to_string(),
        total_memory_mb: 16384,
        has_nvidia_gpu: std::path::Path::new("/proc/driver/nvidia").exists(),
        horizon_ready: bin_path.exists() && res_path.exists(),
        horizon_binary_exists: bin_path.exists(),
        horizon_resources_exist: res_path.exists(),
        java_versions: vec!["Java 21 (OpenJDK)".to_string(), "Java 17 (OpenJDK)".to_string()],
    }
}

#[tauri::command]
fn build_horizon(app: AppHandle) -> Result<(), String> {
    launcher_engine::build_horizon_engine(app)
}

#[tauri::command]
fn window_minimize(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn window_close(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_instances,
            list_loader_versions,
            lookup_player,
            install_modrinth_modpack,
            create_instance,
            update_instance,
            delete_instance,
            open_instance_folder,
            duplicate_instance,
            list_instance_content,
            toggle_instance_content,
            delete_instance_content,
            pick_directory,
            pick_zip_file,
            import_instance_directory,
            import_instance_zip,
            launch_instance,
            kill_instance,
            get_running_instances,
            list_instance_mods,
            toggle_instance_mod,
            delete_instance_mod,
            search_modrinth,
            download_modrinth_mod,
            get_accounts,
            create_offline_account,
            set_active_skin,
            set_active_cape,
            set_active_account,
            delete_account,
            get_settings,
            save_settings,
            get_system_info,
            build_horizon,
            mc::list_game_versions,
            window_minimize,
            window_maximize,
            window_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running fugo launcher");
}
