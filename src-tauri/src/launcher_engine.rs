use crate::accounts::get_active_account;
use crate::instance_manager::{load_instances, update_last_played};
use crate::models::{EditionType, LogMessage};
use crate::settings::{get_default_horizon_paths, load_settings};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

// Global Mutex<HashMap<String, u32>> to store running instance PIDs

static RUNNING_PROCESSES: std::sync::OnceLock<Arc<Mutex<HashMap<String, u32>>>> =
    std::sync::OnceLock::new();

fn get_running_map() -> Arc<Mutex<HashMap<String, u32>>> {
    RUNNING_PROCESSES
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub fn is_instance_running(instance_id: &str) -> bool {
    let map = get_running_map();
    let lock = map.lock().unwrap();
    lock.contains_key(instance_id)
}

pub fn get_running_instances() -> Vec<String> {
    let map = get_running_map();
    let lock = map.lock().unwrap();
    lock.keys().cloned().collect()
}

pub fn kill_instance(instance_id: &str) -> Result<(), String> {
    let map = get_running_map();
    let mut lock = map.lock().unwrap();
    if let Some(pid) = lock.remove(instance_id) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-15", &pid.to_string()])
                .output();
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
        Ok(())
    } else {
        Err(format!("Instance {instance_id} is not running"))
    }
}

pub fn launch_instance(app: AppHandle, instance_id: &str) -> Result<String, String> {
    if is_instance_running(instance_id) {
        return Err("This instance is already running.".to_string());
    }

    let instances = load_instances();
    let instance = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let account = get_active_account().ok_or_else(|| "No active account found".to_string())?;
    let settings = load_settings();

    update_last_played(instance_id);

    match instance.edition {
        EditionType::Rust => launch_rust_edition(app, instance, account, settings),
        EditionType::Java => launch_java_edition(app, instance, account, settings),
    }
}

pub(crate) fn emit_log(app: &AppHandle, instance_id: &str, text: String, level: &str) {
    let log = LogMessage {
        instance_id: instance_id.to_string(),
        text,
        level: level.to_string(),
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
    };
    let _ = app.emit("instance-log", &log);
}

fn launch_rust_edition(
    app: AppHandle,
    instance: crate::models::Instance,
    account: crate::models::Account,
    settings: crate::models::LauncherSettings,
) -> Result<String, String> {
    crate::mc::launch::start_rust(app, instance, account, settings);
    Ok("Starting Horizon Rust Edition".to_string())
}

fn launch_java_edition(
    app: AppHandle,
    instance: crate::models::Instance,
    account: crate::models::Account,
    settings: crate::models::LauncherSettings,
) -> Result<String, String> {
    let version = instance.game_version.clone();
    crate::mc::launch::start(app, instance, account, settings);
    Ok(format!("Starting Minecraft Java Edition {version}"))
}

/// Records a started game's process id for the running list.
pub(crate) fn register_running(instance_id: &str, pid: u32) {
    get_running_map()
        .lock()
        .unwrap()
        .insert(instance_id.to_string(), pid);
}

pub(crate) fn spawn_log_streamers(app: AppHandle, instance_id: String, mut child: Child) {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_out = app.clone();
    let id_out = instance_id.clone();
    if let Some(out) = stdout {
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                emit_log(&app_out, &id_out, line, "info");
            }
        });
    }

    let app_err = app.clone();
    let id_err = instance_id.clone();
    if let Some(err) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                emit_log(&app_err, &id_err, line, "error");
            }
        });
    }

    let app_exit = app.clone();
    let id_exit = instance_id.clone();
    thread::spawn(move || {
        let status = child.wait();
        let map = get_running_map();
        map.lock().unwrap().remove(&id_exit);
        match status {
            Ok(code) => {
                emit_log(
                    &app_exit,
                    &id_exit,
                    format!("Process exited with status: {code}"),
                    "warn",
                );
            }
            Err(e) => {
                emit_log(
                    &app_exit,
                    &id_exit,
                    format!("Process wait error: {e}"),
                    "error",
                );
            }
        }
        let _ = app_exit.emit("instance-stopped", &id_exit);
    });
}

pub fn build_horizon_engine(app: AppHandle) -> Result<(), String> {
    let (res_path, _) = get_default_horizon_paths();
    let root = res_path
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists())
        .ok_or_else(|| "Could not locate Horizon repository root".to_string())?
        .to_path_buf();

    thread::spawn(move || {
        let _ = app.emit("build-status", "Starting cargo build --profile fast -p horizon-client...");

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root)
            .args(["build", "--profile", "fast", "-p", "horizon-client"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let _ = app.emit("build-status", &line);
                }
            }
            let _ = child.wait();
            let _ = app.emit("build-status", "BUILD_COMPLETE");
        } else {
            let _ = app.emit("build-status", "BUILD_FAILED");
        }
    });

    Ok(())
}
