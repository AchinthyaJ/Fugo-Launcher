//! Builds the Java command that starts a version: JVM arguments, main class and game arguments,
//! from the version file (modern `arguments`, or the older `minecraftArguments` string).

use super::install::{Installed, rules_allow};
use super::{libraries_dir, path_string};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub struct LaunchSpec<'a> {
    pub java: PathBuf,
    pub installed: &'a Installed,
    pub game_dir: PathBuf,
    pub version_id: String,
    pub username: String,
    pub uuid: String,
    /// The Minecraft access token a Microsoft account launches with; `0` for an offline one.
    pub access_token: String,
    /// `msa` for a Microsoft account, `legacy` for an offline one.
    pub user_type: String,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    /// The instance's own JVM flags, after the memory flags.
    pub extra_jvm: Vec<String>,
    /// The game window's width and height.
    pub resolution: (u32, u32),
}

/// Replaces every `${name}` that is in `vars`; unknown placeholders stay as written.
fn fill(text: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = text.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${{{key}}}"), value);
    }
    out
}

/// Arguments from a modern `arguments` list, which may hold plain strings or rule-gated values.
fn collect(list: &[Value], vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in list {
        match item {
            Value::String(s) => out.push(fill(s, vars)),
            Value::Object(_) if rules_allow(item.get("rules")) => match item.get("value") {
                Some(Value::String(s)) => out.push(fill(s, vars)),
                Some(Value::Array(values)) => {
                    out.extend(values.iter().filter_map(Value::as_str).map(|s| fill(s, vars)));
                }
                _ => {}
            },
            _ => {}
        }
    }
    out
}

pub fn build(spec: LaunchSpec<'_>) -> Command {
    let installed = spec.installed;
    let json = &installed.version_json;
    let separator = if cfg!(windows) { ";" } else { ":" };

    let mut classpath: Vec<String> = installed.classpath.iter().map(|p| path_string(p)).collect();
    classpath.push(path_string(&installed.client_jar));
    let classpath = classpath.join(separator);
    let natives = path_string(&installed.natives_dir);
    let game_dir = path_string(&spec.game_dir);
    let assets_root = path_string(&installed.assets_root);
    let version_type = json
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("release")
        .to_string();

    let vars: HashMap<&str, String> = HashMap::from([
        ("natives_directory", natives.clone()),
        ("launcher_name", "fugo-launcher".to_string()),
        ("launcher_version", "3.2.0".to_string()),
        ("classpath", classpath.clone()),
        ("classpath_separator", separator.to_string()),
        ("library_directory", path_string(&libraries_dir())),
        ("version_name", spec.version_id.clone()),
        ("version_type", version_type),
        ("auth_player_name", spec.username.clone()),
        ("auth_uuid", spec.uuid.clone()),
        ("auth_access_token", spec.access_token.clone()),
        ("auth_session", spec.access_token.clone()),
        ("auth_xuid", String::new()),
        ("clientid", String::new()),
        ("user_type", spec.user_type.clone()),
        ("user_properties", "{}".to_string()),
        ("game_directory", game_dir.clone()),
        ("assets_root", assets_root.clone()),
        ("game_assets", assets_root.clone()),
        ("assets_index_name", installed.asset_index.clone()),
        ("resolution_width", spec.resolution.0.to_string()),
        ("resolution_height", spec.resolution.1.to_string()),
        ("quickPlayPath", String::new()),
        ("quickPlaySingleplayer", String::new()),
        ("quickPlayMultiplayer", String::new()),
        ("quickPlayRealms", String::new()),
    ]);

    let mut jvm: Vec<String> = vec![
        format!("-Xmx{}M", spec.memory_max_mb.max(512)),
        format!("-Xms{}M", spec.memory_min_mb.max(256).min(spec.memory_max_mb.max(512))),
    ];
    jvm.extend(spec.extra_jvm.iter().cloned());

    match json.pointer("/arguments/jvm").and_then(Value::as_array) {
        Some(list) => jvm.extend(collect(list, &vars)),
        None => {
            // Versions before 1.13 list no JVM arguments: set the library path and classpath.
            jvm.push(format!("-Djava.library.path={natives}"));
            jvm.push("-cp".to_string());
            jvm.push(classpath.clone());
        }
    }
    if let Some((argument, path)) = &installed.log_config {
        let path = path_string(path);
        jvm.push(fill(argument, &HashMap::from([("path", path)])));
    }

    let main_class = json
        .get("mainClass")
        .and_then(Value::as_str)
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();

    let game: Vec<String> = match json.pointer("/arguments/game").and_then(Value::as_array) {
        Some(list) => collect(list, &vars),
        None => json
            .get("minecraftArguments")
            .and_then(Value::as_str)
            .unwrap_or("")
            .split_whitespace()
            .map(|s| fill(s, &vars))
            .collect(),
    };

    let mut cmd = Command::new(&spec.java);
    cmd.current_dir(&spec.game_dir)
        .args(&jvm)
        .arg(main_class)
        .args(&game);
    cmd
}
