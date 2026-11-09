//! Starts a Java-edition instance: installs its version if needed, finds Java, builds the
//! command and runs it. The work runs off the UI thread; progress and errors are logged to the
//! instance's log.

use super::args::{LaunchSpec, build};
use super::horizon::{self, Log};
use super::{expand_home, forge, http, install, java, loader};
use crate::launcher_engine::{emit_log, register_running, spawn_log_streamers};
use crate::models::{Account, Instance, LauncherSettings};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter};

/// The UUID Minecraft gives an offline player: MD5 of `OfflinePlayer:<name>`, as version 3.
pub fn offline_uuid(username: &str) -> String {
    let mut bytes = md5::compute(format!("OfflinePlayer:{username}")).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

/// Starts the instance on its own thread and returns at once.
pub fn start(app: AppHandle, instance: Instance, account: Account, settings: LauncherSettings) {
    thread::spawn(move || {
        let id = instance.id.clone();
        let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(e) => {
                emit_log(&app, &id, format!("Could not start the launcher's worker: {e}"), "error");
                let _ = app.emit("instance-stopped", &id);
                return;
            }
        };
        if let Err(message) = runtime.block_on(run(&app, &instance, &account, &settings)) {
            emit_log(&app, &id, format!("Launch failed: {message}"), "error");
            let _ = app.emit("instance-stopped", &id);
        }
    });
}

async fn run(
    app: &AppHandle,
    instance: &Instance,
    account: &Account,
    settings: &LauncherSettings,
) -> Result<(), String> {
    let id = instance.id.clone();
    let game_dir = expand_home(&instance.game_dir);
    tokio::fs::create_dir_all(&game_dir).await.map_err(super::err)?;
    let client = http();

    // Check if death-profile.json specifies versionId, javaPath, or jvmArguments
    let profile_path = game_dir.join("death-profile.json");
    let mut version = instance.game_version.clone();
    let mut custom_java_from_profile = None;
    let mut extra_jvm_from_profile = None;
    if let Ok(data) = tokio::fs::read_to_string(&profile_path).await
        && let Ok(profile) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(vid) = profile.get("versionId").and_then(|v| v.as_str())
                && !vid.is_empty() && super::versions_dir().join(vid).exists() {
                    version = vid.to_string();
                }
            if let Some(jp) = profile.get("javaPath").and_then(|v| v.as_str())
                && !jp.is_empty() && std::path::Path::new(jp).exists() {
                    custom_java_from_profile = Some(jp.to_string());
                }
            if let Some(jvm) = profile.get("jvmArguments").and_then(|v| v.as_str())
                && !jvm.trim().is_empty() {
                    extra_jvm_from_profile = Some(jvm.to_string());
                }
        }

    emit_log(app, &id, format!("Preparing Minecraft {version}..."), "info");
    let progress_app = app.clone();
    let progress_id = id.clone();
    let progress = move |stage: &str, done: usize, total: usize| {
        let _ = progress_app.emit(
            "install-progress",
            json!({ "instanceId": progress_id, "stage": stage, "done": done, "total": total }),
        );
    };
    match instance.loader.as_str() {
        loader_name if forge::is_installer_loader(loader_name) => {
            version = forge::ensure_installed(&client, loader_name, &instance.game_version, instance.loader_version.as_deref(), &progress).await?;
        }
        loader_name if loader::is_profile_loader(loader_name) => {
            version = loader::ensure_profile(&client, loader_name, &instance.game_version, instance.loader_version.as_deref(), &progress).await?;
        }
        _ => {}
    }
    emit_log(app, &id, format!("Preparing {version}..."), "info");

    let installed = install::install(&client, &version, &game_dir, &progress).await?;
    super::appearance::deploy(&game_dir, account.skin_url.as_deref(), account.cape_data.as_deref())?;

    let java_log = {
        let app = app.clone();
        let id = id.clone();
        move |stage: &str, _done: usize, _total: usize| emit_log(&app, &id, stage.to_string(), "info")
    };
    let custom_java = custom_java_from_profile
        .as_deref()
        .or(settings.custom_java_path.as_deref());
    let java_bin = java::ensure_java(&client, installed.java_major, custom_java, &java_log).await?;

    let jvm_str = extra_jvm_from_profile.unwrap_or_else(|| instance.java_args.clone());
    let mut extra_jvm: Vec<String> = jvm_str.split_whitespace().map(String::from).collect();
    // Skins and capes go to the skin service before the game starts; authlib-injector points the game at it.
    // Preset skins have no image in the launcher, so only a picked skin or a cape is uploaded.
    if let Some(skin) = account.skin_url.as_deref() {
        let model = if account.skin_slim { "slim" } else { "classic" };
        if let Err(e) = super::appearance::upload_skin(&client, &account.username, skin, model).await {
            emit_log(app, &id, format!("Could not upload skin: {e}"), "warn");
        }
    }
    if let Some(cape) = account.cape_data.as_deref()
        && let Err(e) = super::appearance::upload_cape(&client, &account.username, cape).await
    {
        emit_log(app, &id, format!("Could not upload cape: {e}"), "warn");
    }
    match super::appearance::authlib_injector(&client).await {
        Ok(jar) => extra_jvm.push(format!("-javaagent:{}={}", jar.display(), super::appearance::SKIN_SERVER)),
        Err(e) => emit_log(app, &id, format!("Skins off: {e}"), "warn"),
    }
    let mut cmd = build(LaunchSpec {
        java: java_bin,
        installed: &installed,
        game_dir: game_dir.clone(),
        version_id: version.clone(),
        username: account.username.clone(),
        uuid: offline_uuid(&account.username),
        memory_min_mb: instance.memory_min_mb,
        memory_max_mb: instance.memory_max_mb,
        extra_jvm,
    });
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    emit_log(app, &id, format!("Starting Minecraft {version} for '{}'", account.username), "info");
    let child = cmd.spawn().map_err(|e| format!("Could not start Java: {e}"))?;
    register_running(&id, child.id());
    spawn_log_streamers(app.clone(), id, child);
    Ok(())
}

/// Starts the Rust Edition on its own thread: the latest release, its prepared files, then the game.
pub fn start_rust(app: AppHandle, instance: Instance, account: Account, settings: LauncherSettings) {
    thread::spawn(move || {
        let id = instance.id.clone();
        let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(e) => {
                emit_log(&app, &id, format!("Could not start the launcher's worker: {e}"), "error");
                let _ = app.emit("instance-stopped", &id);
                return;
            }
        };
        if let Err(message) = runtime.block_on(run_rust(&app, &instance, &account, &settings)) {
            emit_log(&app, &id, format!("Launch failed: {message}"), "error");
            let _ = app.emit("instance-stopped", &id);
        }
    });
}

fn log_sink(app: &AppHandle, id: &str) -> Log {
    let app = app.clone();
    let id = id.to_string();
    Arc::new(move |message: &str| emit_log(&app, &id, message.to_string(), "info"))
}

async fn run_rust(
    app: &AppHandle,
    instance: &Instance,
    account: &Account,
    settings: &LauncherSettings,
) -> Result<(), String> {
    let id = instance.id.clone();
    let game_dir = expand_home(&instance.game_dir);
    tokio::fs::create_dir_all(&game_dir).await.map_err(super::err)?;
    let client = http();
    let log = log_sink(app, &id);
    let progress_app = app.clone();
    let progress_id = id.clone();
    let progress = move |stage: &str, done: usize, total: usize| {
        let _ = progress_app.emit(
            "install-progress",
            json!({ "instanceId": progress_id, "stage": stage, "done": done, "total": total }),
        );
    };

    // A binary and resources the player set in Settings are used as they are; otherwise the
    // latest release is installed and its files prepared.
    let custom_binary = PathBuf::from(&settings.horizon_binary_path);
    let custom_resources = PathBuf::from(&settings.horizon_resources_path);
    let (binary, resources) = if !settings.horizon_binary_path.is_empty()
        && custom_binary.is_file()
        && horizon::is_prepared(&custom_resources)
    {
        (custom_binary, custom_resources)
    } else {
        horizon::ready(&client, &progress, &log).await?
    };
    let binary = std::path::absolute(&binary).map_err(super::err)?;
    let resources = std::path::absolute(&resources).map_err(super::err)?;

    log(&format!("Starting Horizon Rust Edition for '{}'", account.username));
    let mut cmd = std::process::Command::new(&binary);
    cmd.current_dir(&game_dir)
        .arg("--resources")
        .arg(&resources)
        .arg("--game-dir")
        .arg(&game_dir)
        .arg("--name")
        .arg(&account.username)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if settings.discrete_gpu {
        cmd.env("__NV_PRIME_RENDER_OFFLOAD", "1")
            .env("__VK_LAYER_NV_optimus", "NVIDIA_only")
            .env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
            .env("WGPU_ADAPTER_NAME", "NVIDIA")
            .env("WGPU_BACKEND", "vulkan");
    }
    let child = cmd.spawn().map_err(|e| format!("Could not start the Horizon game: {e}"))?;
    register_running(&id, child.id());
    spawn_log_streamers(app.clone(), id, child);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::offline_uuid;

    #[test]
    fn offline_uuid_matches_minecraft() {
        // The UUID Minecraft gives the offline player "Notch".
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }
}

#[cfg(test)]
mod install_check {
    use super::super::{args::{LaunchSpec, build}, expand_home, http, install, java};
    use std::path::PathBuf;

    /// Installs a real version into the launcher's folders and prints the command that starts it.
    /// Ignored by default: it downloads hundreds of megabytes. Run with `--ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn installs_and_builds_a_real_version() {
        let version = std::env::var("FUGO_TEST_VERSION").unwrap_or_else(|_| "26.3".into());
        let game_dir: PathBuf = expand_home("~/.local/share/fugo-launcher/test-instance");
        tokio::fs::create_dir_all(&game_dir).await.unwrap();
        let client = http();
        let progress = |stage: &str, done: usize, total: usize| println!("{stage}: {done}/{total}");
        let installed = install::install(&client, &version, &game_dir, &progress).await.unwrap();
        println!("java major needed: {}", installed.java_major);
        let java_bin = java::ensure_java(&client, installed.java_major, None, &progress).await.unwrap();
        println!("java: {}", java_bin.display());
        let cmd = build(LaunchSpec {
            java: java_bin,
            installed: &installed,
            game_dir: game_dir.clone(),
            version_id: version.clone(),
            username: "TestPlayer".into(),
            uuid: super::offline_uuid("TestPlayer"),
            memory_min_mb: 512,
            memory_max_mb: 2048,
            extra_jvm: vec![],
        });
        println!("command: {cmd:?}");
    }
}
