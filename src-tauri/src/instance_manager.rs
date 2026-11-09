use crate::models::{CreateInstanceRequest, EditionType, Instance, InstalledMod};
use crate::settings::get_app_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Path to Fugo / Death Client instances folder: ~/.minecraft/death-client/instances
pub fn get_instances_dir() -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
        .join("death-client")
        .join("instances");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_death_client_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
        .join("death-client")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeathProfile {
    pub name: String,
    pub game_version: String,
    #[serde(default = "default_loader")]
    pub loader: String,
    pub loader_version: Option<String>,
    #[serde(default)]
    pub version_id: String,
    pub source_project_slug: Option<String>,
    pub created_utc: Option<String>,
    pub updated_utc: Option<String>,
    pub instance_directory: Option<String>,
    #[serde(default)]
    pub game_directory_override: String,
    #[serde(default)]
    pub jvm_arguments: String,
    #[serde(default)]
    pub java_path: String,
    pub max_ram_mb: Option<u32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    #[serde(default)]
    pub environment_variables: String,
    #[serde(default)]
    pub installed_mod_ids: HashSet<String>,
    #[serde(default)]
    pub is_installed: bool,
    #[serde(default)]
    pub last_launch_crashed: bool,
    #[serde(default)]
    pub launch_count_since_last_install: u32,
    pub mods_directory: Option<String>,
    pub loader_display: Option<String>,
}

fn default_loader() -> String {
    "vanilla".to_string()
}

pub fn detect_is_rust(dir: &std::path::Path, v: Option<&serde_json::Value>, name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let dir_name_lower = dir
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 1. Check folder/instance name
    if name_lower.contains("rust")
        || name_lower.contains("horizon")
        || dir_name_lower.contains("rust")
        || dir_name_lower.contains("horizon")
    {
        return true;
    }

    // 2. Check manifest json (death-profile.json or similar)
    if let Some(json) = v {
        let loader = json.get("loader").and_then(|l| l.as_str()).unwrap_or("").to_lowercase();
        if loader.contains("rust") || loader.contains("horizon") {
            return true;
        }
        let loader_display = json.get("loaderDisplay").and_then(|l| l.as_str()).unwrap_or("").to_lowercase();
        if loader_display.contains("rust") || loader_display.contains("horizon") {
            return true;
        }
        let version_id = json.get("versionId").and_then(|l| l.as_str()).unwrap_or("").to_lowercase();
        if version_id.contains("rust") || version_id.contains("horizon") {
            return true;
        }
        let slug = json.get("sourceProjectSlug").and_then(|l| l.as_str()).unwrap_or("").to_lowercase();
        if slug.contains("rust") || slug.contains("horizon") {
            return true;
        }
        if json.get("edition").and_then(|e| e.as_str()) == Some("rust") {
            return true;
        }
    }

    // 3. Check Horizon-specific files in the instance directory
    if dir.join(".horizon-cache").exists()
        || dir.join("horizon-graphics.txt").exists()
        || dir.join("horizon-extras.txt").exists()
        || dir.join("horizon-mods").exists()
        || dir.join("horizon-native-mods").exists()
        || dir.join("horizon-mods.txt").exists()
        || dir.join("horizon_official_server.txt").exists()
        || dir.join("horizon_partner_servers.txt").exists()
        || dir.join("horizon.toml").exists()
        || dir.join(".horizon").exists()
    {
        return true;
    }

    // 4. Check for .hmod files in mods/ or horizon-mods/
    for folder in [dir.join("mods"), dir.join("horizon-mods")] {
        if folder.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&folder) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext.to_string_lossy().eq_ignore_ascii_case("hmod") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // 5. Check if any file/folder in root starts with "horizon"
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.starts_with("horizon") {
                return true;
            }
        }
    }

    false
}

/// The mod loader a Prism or MultiMC pack declares in its `mmc-pack.json`, if it has one.
fn detect_loader_from_pack(source: &std::path::Path) -> Option<&'static str> {
    let text = fs::read_to_string(source.join("mmc-pack.json")).ok()?;
    let pack: serde_json::Value = serde_json::from_str(&text).ok()?;
    pack.get("components")?.as_array()?.iter().find_map(|c| {
        match c.get("uid")?.as_str()? {
            "net.fabricmc.fabric-loader" => Some("fabric"),
            "org.quiltmc.quilt-loader" => Some("quilt"),
            "net.minecraftforge" => Some("forge"),
            "net.neoforged" => Some("neoforge"),
            _ => None,
        }
    })
}

pub fn write_death_profile(inst: &Instance) -> Result<(), String> {
    let dir = PathBuf::from(&inst.game_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let (loader, version_id, loader_display) = match inst.edition {
        EditionType::Rust => (
            "horizon-rust".to_string(),
            format!("horizon-rust-{}", inst.game_version),
            format!("Horizon Rust · {}", inst.game_version),
        ),
        EditionType::Java => {
            if inst.loader == "vanilla" {
                (inst.loader.clone(), inst.game_version.clone(), inst.game_version.clone())
            } else if inst.loader == "fabric" {
                (inst.loader.clone(), format!("fabric-loader-0.19.5-{}", inst.game_version), format!("fabric · {}", inst.game_version))
            } else {
                (inst.loader.clone(), inst.game_version.clone(), format!("{} · {}", inst.loader, inst.game_version))
            }
        }
    };

    let profile = DeathProfile {
        name: inst.name.clone(),
        game_version: inst.game_version.clone(),
        loader,
        loader_version: inst.loader_version.clone(),
        version_id,
        source_project_slug: None,
        created_utc: Some(chrono::Utc::now().to_rfc3339()),
        updated_utc: Some(chrono::Utc::now().to_rfc3339()),
        instance_directory: Some(dir.to_string_lossy().to_string()),
        game_directory_override: String::new(),
        jvm_arguments: inst.java_args.clone(),
        java_path: String::new(),
        max_ram_mb: Some(inst.memory_max_mb),
        window_width: None,
        window_height: None,
        environment_variables: String::new(),
        installed_mod_ids: HashSet::new(),
        is_installed: true,
        last_launch_crashed: false,
        launch_count_since_last_install: 0,
        mods_directory: Some(dir.join("mods").to_string_lossy().to_string()),
        loader_display: Some(loader_display),
    };

    let path = dir.join("death-profile.json");
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_instances() -> Vec<Instance> {
    let instances_dir = get_instances_dir();

    // Read any existing cached instances.json
    let mut cached_map: std::collections::HashMap<String, Instance> = std::collections::HashMap::new();
    for file in [
        get_death_client_dir().join("instances.json"),
        get_app_dir().join("instances.json"),
    ] {
        if let Ok(data) = fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<Instance>>(&data) {
                for inst in list {
                    cached_map.entry(inst.id.clone()).or_insert(inst);
                }
            }
        }
    }

    let mut discovered: Vec<Instance> = Vec::new();
    if let Ok(entries) = fs::read_dir(&instances_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let folder_name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            if folder_name.starts_with('.') {
                continue;
            }

            let profile_path = path.join("death-profile.json");
            let cached = cached_map.remove(&folder_name);

            // Determine cover / banner
            let banner = if path.join("cover.png").is_file() {
                path.join("cover.png").to_string_lossy().to_string()
            } else if path.join("cover.jpg").is_file() {
                path.join("cover.jpg").to_string_lossy().to_string()
            } else if let Some(ref c) = cached {
                c.banner.clone()
            } else {
                match folder_name.as_str() {
                    "horizon-26-3-rust" => "mountains_misty.png".to_string(),
                    "vanilla-1-20-4" => "instance_default_cover_1.png".to_string(),
                    "fabulously-optimized" => "instance_default_cover_2.png".to_string(),
                    "fugo-client" => "instance_default_cover_3.png".to_string(),
                    _ => "instance_default_cover_1.png".to_string(),
                }
            };

            if profile_path.is_file() {
                if let Ok(data) = fs::read_to_string(&profile_path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        let raw_name = v.get("name").and_then(|n| n.as_str()).unwrap_or(&folder_name).trim();
                        let name = if raw_name.is_empty() { folder_name.clone() } else { raw_name.to_string() };
                        let mut loader = v.get("loader").and_then(|l| l.as_str()).unwrap_or("vanilla").to_string();
                        let mut game_version = v.get("gameVersion").and_then(|g| g.as_str()).unwrap_or("26.3").to_string();
                        let is_rust = detect_is_rust(&path, Some(&v), &name);
                        let edition = if is_rust {
                            loader = "horizon-rust".to_string();
                            game_version = "26.3".to_string();
                            EditionType::Rust
                        } else {
                            EditionType::Java
                        };
                        let memory_max_mb = v.get("maxRamMb").and_then(|m| m.as_u64()).map(|m| m as u32).unwrap_or(4096);
                        let jvm = v.get("jvmArguments").and_then(|j| j.as_str()).unwrap_or("");
                        let java_args = if !jvm.trim().is_empty() {
                            jvm.to_string()
                        } else if let Some(ref c) = cached {
                            c.java_args.clone()
                        } else {
                            "-Xmx4G -XX:+UseG1GC".to_string()
                        };
                        let last_played = cached.as_ref().and_then(|c| c.last_played.clone());
                        let created_at = v.get("createdUtc")
                            .and_then(|c| c.as_str())
                            .and_then(|d| d.split('T').next())
                            .map(String::from)
                            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

                        let inst = Instance {
                            id: folder_name,
                            name,
                            edition,
                            game_version,
                            loader,
                            game_dir: path.to_string_lossy().to_string(),
                            banner,
                            icon: "minecraft_icon.png".to_string(),
                            memory_min_mb: 2048,
                            memory_max_mb,
                            java_args,
                            created_at,
                            last_played,
                            loader_version: v
                                .get("loaderVersion")
                                .and_then(|x| x.as_str())
                                .filter(|s| !s.trim().is_empty())
                                .map(String::from),
                        };
                        if is_rust {
                            let _ = write_death_profile(&inst);
                        }
                        discovered.push(inst);
                        continue;
                    }
                }
            }

            // Folder exists without a valid death-profile.json
            let inst = if let Some(mut c) = cached {
                c.game_dir = path.to_string_lossy().to_string();
                if detect_is_rust(&path, None, &c.name) {
                    c.edition = EditionType::Rust;
                    c.loader = "horizon-rust".to_string();
                    c.game_version = "26.3".to_string();
                }
                c
            } else {
                let is_rust = detect_is_rust(&path, None, &folder_name);
                let edition = if is_rust { EditionType::Rust } else { EditionType::Java };
                let game_version = if is_rust {
                    "26.3".to_string()
                } else if folder_name.contains("1.20.4") {
                    "1.20.4".to_string()
                } else {
                    "26.3".to_string()
                };
                let loader = if is_rust { "horizon-rust".to_string() } else { "vanilla".to_string() };
                Instance {
                    id: folder_name.clone(),
                    name: folder_name.clone(),
                    edition,
                    game_version,
                    loader,
                    game_dir: path.to_string_lossy().to_string(),
                    banner,
                    icon: "minecraft_icon.png".to_string(),
                    memory_min_mb: 2048,
                    memory_max_mb: 4096,
                    java_args: "-Xmx4G -XX:+UseG1GC".to_string(),
                    created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    last_played: None,
                    loader_version: None,
                }
            };

            let _ = write_death_profile(&inst);
            discovered.push(inst);
        }
    }

    // If no instances found at all, initialize default Horizon and Vanilla
    if discovered.is_empty() {
        let rust_dir = instances_dir.join("horizon-26-3-rust");
        let _ = fs::create_dir_all(&rust_dir);
        let default_rust = Instance {
            id: "horizon-26-3-rust".to_string(),
            name: "Horizon 26.3 (Rust Edition)".to_string(),
            edition: EditionType::Rust,
            game_version: "26.3".to_string(),
            loader: "horizon-rust".to_string(),
            game_dir: rust_dir.to_string_lossy().to_string(),
            banner: "mountains_misty.png".to_string(),
            icon: "minecraft_icon.png".to_string(),
            memory_min_mb: 2048,
            memory_max_mb: 4096,
            java_args: "-XX:+UnlockExperimentalVMOptions -XX:+UseG1GC".to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
            last_played: None,
            loader_version: None,
        };
        let _ = write_death_profile(&default_rust);
        discovered.push(default_rust);

        let java_dir = instances_dir.join("vanilla-1-20-4");
        let _ = fs::create_dir_all(&java_dir);
        let default_java = Instance {
            id: "vanilla-1-20-4".to_string(),
            name: "Vanilla 1.20.4 (Java)".to_string(),
            edition: EditionType::Java,
            game_version: "1.20.4".to_string(),
            loader: "vanilla".to_string(),
            game_dir: java_dir.to_string_lossy().to_string(),
            banner: "instance_default_cover_1.png".to_string(),
            icon: "minecraft_icon.png".to_string(),
            memory_min_mb: 2048,
            memory_max_mb: 4096,
            java_args: "-Xmx4G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC".to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
            last_played: None,
            loader_version: None,
        };
        let _ = write_death_profile(&default_java);
        discovered.push(default_java);
    }

    // Sort: Rust first, then alphabetically
    discovered.sort_by(|a, b| {
        match (a.edition.clone(), b.edition.clone()) {
            (EditionType::Rust, EditionType::Java) => std::cmp::Ordering::Less,
            (EditionType::Java, EditionType::Rust) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    let _ = save_instances(&discovered);
    discovered
}

pub fn save_instances(instances: &[Instance]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(instances).map_err(|e| e.to_string())?;
    let _ = fs::write(get_death_client_dir().join("instances.json"), &json);
    let _ = fs::write(get_app_dir().join("instances.json"), &json);
    Ok(())
}

pub fn create_instance(req: CreateInstanceRequest) -> Result<Instance, String> {
    let mut instances = load_instances();
    let slug = req
        .name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-");
    let slug = slug.trim_matches('-').to_string();
    let base_slug = if slug.is_empty() { "profile".to_string() } else { slug };

    let instances_dir = get_instances_dir();
    let mut candidate = instances_dir.join(&base_slug);
    let mut id = base_slug.clone();
    let mut counter = 2;
    while candidate.exists() {
        id = format!("{base_slug}-{counter}");
        candidate = instances_dir.join(&id);
        counter += 1;
    }

    let game_dir = candidate;
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(game_dir.join("mods")).map_err(|e| e.to_string())?;

    let banner = req
        .banner
        .unwrap_or_else(|| "instance_default_cover_1.png".to_string());

    let new_inst = Instance {
        id: id.clone(),
        name: req.name,
        edition: req.edition,
        game_version: req.game_version,
        loader: req.loader,
        game_dir: game_dir.to_string_lossy().to_string(),
        banner,
        icon: "minecraft_icon.png".to_string(),
        memory_min_mb: 2048,
        memory_max_mb: 4096,
        java_args: "-Xmx4G -XX:+UseG1GC".to_string(),
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        last_played: None,
        loader_version: None,
    };

    let _ = write_death_profile(&new_inst);
    instances.push(new_inst.clone());
    save_instances(&instances)?;
    Ok(new_inst)
}

pub fn delete_instance(id: &str) -> Result<(), String> {
    let mut instances = load_instances();
    if let Some(inst) = instances.iter().find(|i| i.id == id) {
        let p = PathBuf::from(&inst.game_dir);
        if p.exists() {
            let _ = fs::remove_dir_all(p);
        }
    }
    let direct_p = get_instances_dir().join(id);
    if direct_p.exists() {
        let _ = fs::remove_dir_all(direct_p);
    }
    instances.retain(|i| i.id != id);
    save_instances(&instances)
}

pub fn update_instance(updated: Instance) -> Result<Instance, String> {
    let mut instances = load_instances();
    if let Some(pos) = instances.iter().position(|i| i.id == updated.id) {
        instances[pos] = updated.clone();
        let _ = write_death_profile(&updated);
        save_instances(&instances)?;
        Ok(updated)
    } else {
        Err(format!("Instance not found: {}", updated.id))
    }
}

pub fn update_last_played(id: &str) {
    let mut instances = load_instances();
    if let Some(inst) = instances.iter_mut().find(|i| i.id == id) {
        inst.last_played = Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
        let _ = save_instances(&instances);
    }
}

pub fn list_instance_mods(instance_id: &str) -> Result<Vec<InstalledMod>, String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let mods_dir = PathBuf::from(&inst.game_dir).join("mods");
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let entries = fs::read_dir(mods_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "jar" || ext_str == "disabled" || ext_str == "hmod" {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let enabled = !filename.ends_with(".disabled");
                let name = filename
                    .trim_end_matches(".disabled")
                    .trim_end_matches(".jar")
                    .trim_end_matches(".hmod")
                    .to_string();
                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                result.push(InstalledMod {
                    filename,
                    name,
                    enabled,
                    size_bytes,
                });
            }
        }
    }
    Ok(result)
}

pub fn toggle_instance_mod(instance_id: &str, filename: &str, enabled: bool) -> Result<(), String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let mods_dir = PathBuf::from(&inst.game_dir).join("mods");
    let current_path = mods_dir.join(filename);
    if !current_path.exists() {
        return Err(format!("Mod file {filename} does not exist"));
    }

    let target_filename = if enabled {
        filename.trim_end_matches(".disabled").to_string()
    } else if !filename.ends_with(".disabled") {
        format!("{filename}.disabled")
    } else {
        filename.to_string()
    };

    let target_path = mods_dir.join(target_filename);
    fs::rename(current_path, target_path).map_err(|e| e.to_string())
}

pub fn delete_instance_mod(instance_id: &str, filename: &str) -> Result<(), String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let file_path = PathBuf::from(&inst.game_dir).join("mods").join(filename);
    if file_path.exists() {
        fs::remove_file(file_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else if file_type.is_file() {
            let _ = fs::copy(&entry_path, &target_path);
        }
    }
    Ok(())
}

pub fn pick_directory() -> Result<Option<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("zenity")
            .args(["--file-selection", "--directory", "--title=Select Instance Directory to Import"])
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !s.is_empty() {
                    return Ok(Some(s));
                }
            }
        }
    }
    Ok(None)
}

pub fn pick_zip_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("zenity")
            .args(["--file-selection", "--file-filter=*.zip", "--title=Select Instance Backup (.zip) to Import"])
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !s.is_empty() {
                    return Ok(Some(s));
                }
            }
        }
    }
    Ok(None)
}

pub fn import_instance_directory(source_path_str: &str, edition_override: Option<&str>) -> Result<Instance, String> {
    let source = PathBuf::from(source_path_str.trim());
    if !source.exists() || !source.is_dir() {
        return Err(format!("Selected path is not a directory: {source_path_str}"));
    }

    let folder_name = source
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported-instance".to_string());

    let instances_dir = get_instances_dir();

    // Check if source has death-profile.json
    let death_profile_path = source.join("death-profile.json");
    let mut instance_name = folder_name.clone();
    let mut game_version = "1.20.4".to_string();
    let mut loader = "vanilla".to_string();
    let mut memory_max_mb = 4096;
    let mut java_args = "-Xmx4G -XX:+UseG1GC".to_string();
    let mut parsed_json_value: Option<serde_json::Value> = None;

    if death_profile_path.is_file() {
        if let Ok(data) = fs::read_to_string(&death_profile_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
                    if !n.trim().is_empty() {
                        instance_name = n.to_string();
                    }
                }
                if let Some(gv) = v.get("gameVersion").and_then(|g| g.as_str()) {
                    if !gv.trim().is_empty() {
                        game_version = gv.to_string();
                    }
                }
                if let Some(l) = v.get("loader").and_then(|l| l.as_str()) {
                    if !l.trim().is_empty() {
                        loader = l.to_string();
                    }
                }
                if let Some(ram) = v.get("maxRamMb").and_then(|r| r.as_u64()) {
                    memory_max_mb = ram as u32;
                }
                if let Some(jvm) = v.get("jvmArguments").and_then(|j| j.as_str()) {
                    if !jvm.trim().is_empty() {
                        java_args = jvm.to_string();
                    }
                }
                parsed_json_value = Some(v);
            }
        }
    } else {
        // No launcher profile: read the loader from the pack's own metadata (Prism and MultiMC
        // list each component by its uid). A mods folder alone says nothing about the loader.
        if let Some(detected) = detect_loader_from_pack(&source) {
            loader = detected.to_string();
        }
    }

    let is_rust = match edition_override {
        Some("rust") => true,
        Some("java") => false,
        _ => detect_is_rust(&source, parsed_json_value.as_ref(), &instance_name),
    };
    let edition = if is_rust { EditionType::Rust } else { EditionType::Java };
    let loader = if is_rust { "horizon-rust".to_string() } else { loader };
    let game_version = if is_rust { "26.3".to_string() } else { game_version };

    // Target directory in ~/.minecraft/death-client/instances/
    let is_already_in_instances_root = source.starts_with(&instances_dir);
    let target_dir = if is_already_in_instances_root {
        source.clone()
    } else {
        let slug = instance_name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-");
        let slug = slug.trim_matches('-').to_string();
        let base_slug = if slug.is_empty() { "imported".to_string() } else { slug };

        let mut candidate = instances_dir.join(&base_slug);
        let mut counter = 2;
        while candidate.exists() {
            candidate = instances_dir.join(format!("{base_slug}-{counter}"));
            counter += 1;
        }

        // Copy source directory into target_dir
        copy_dir_all(&source, &candidate)?;
        candidate
    };

    let id = target_dir
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let banner = if target_dir.join("cover.png").is_file() {
        target_dir.join("cover.png").to_string_lossy().to_string()
    } else if target_dir.join("cover.jpg").is_file() {
        target_dir.join("cover.jpg").to_string_lossy().to_string()
    } else {
        "instance_default_cover_1.png".to_string()
    };

    let new_inst = Instance {
        id: id.clone(),
        name: instance_name,
        edition,
        game_version,
        loader,
        game_dir: target_dir.to_string_lossy().to_string(),
        banner,
        icon: "minecraft_icon.png".to_string(),
        memory_min_mb: 2048,
        memory_max_mb,
        java_args,
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        last_played: None,
        loader_version: None,
    };

    let _ = write_death_profile(&new_inst);
    let mut instances = load_instances();
    instances.retain(|i| i.id != id);
    instances.push(new_inst.clone());
    let _ = save_instances(&instances);
    Ok(new_inst)
}

pub fn import_instance_zip(zip_path_str: &str, edition_override: Option<&str>) -> Result<Instance, String> {
    let zip_file_path = PathBuf::from(zip_path_str.trim());
    if !zip_file_path.exists() || !zip_file_path.is_file() {
        return Err(format!("Selected zip file does not exist: {zip_path_str}"));
    }

    let file_stem = zip_file_path
        .file_stem()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported-zip".to_string());

    let slug = file_stem
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-");
    let slug = slug.trim_matches('-').to_string();
    let base_slug = if slug.is_empty() { "imported-zip".to_string() } else { slug };

    let instances_dir = get_instances_dir();
    let mut candidate = instances_dir.join(&base_slug);
    let mut counter = 2;
    while candidate.exists() {
        candidate = instances_dir.join(format!("{base_slug}-{counter}"));
        counter += 1;
    }

    let file = fs::File::open(&zip_file_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    fs::create_dir_all(&candidate).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match entry.enclosed_name() {
            Some(path) => candidate.join(path),
            None => continue,
        };

        if entry.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    import_instance_directory(&candidate.to_string_lossy(), edition_override)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceContentItem {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

pub fn list_instance_content(instance_id: &str, subfolder: &str) -> Result<Vec<InstanceContentItem>, String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let folder = PathBuf::from(&inst.game_dir).join(subfolder);
    if !folder.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if filename.starts_with('.') {
                continue;
            }
            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let enabled = !filename.ends_with(".disabled");
            let clean_name = filename
                .trim_end_matches(".disabled")
                .trim_end_matches(".zip")
                .trim_end_matches(".tar.gz")
                .to_string();

            let mut description = None;
            if subfolder == "resourcepacks" {
                if path.is_dir() {
                    let mcmeta_path = path.join("pack.mcmeta");
                    if mcmeta_path.exists() {
                        if let Ok(content) = fs::read_to_string(mcmeta_path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(desc) = json.get("pack").and_then(|p| p.get("description")) {
                                    if let Some(text) = desc.as_str() {
                                        description = Some(text.to_string());
                                    } else if let Some(text) = desc.get("text").and_then(|t| t.as_str()) {
                                        description = Some(text.to_string());
                                    }
                                }
                            }
                        }
                    }
                } else if path.is_file() {
                    if let Ok(file) = fs::File::open(&path) {
                        if let Ok(mut archive) = zip::ZipArchive::new(file) {
                            if let Ok(mut mcmeta_entry) = archive.by_name("pack.mcmeta") {
                                use std::io::Read;
                                let mut content = String::new();
                                if mcmeta_entry.read_to_string(&mut content).is_ok() {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                        if let Some(desc) = json.get("pack").and_then(|p| p.get("description")) {
                                            if let Some(text) = desc.as_str() {
                                                description = Some(text.to_string());
                                            } else if let Some(text) = desc.get("text").and_then(|t| t.as_str()) {
                                                description = Some(text.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            result.push(InstanceContentItem {
                name: if clean_name.is_empty() { filename.clone() } else { clean_name },
                filename,
                size_bytes,
                enabled,
                description,
            });
        }
    }
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

pub fn toggle_instance_content(instance_id: &str, subfolder: &str, filename: &str, enabled: bool) -> Result<(), String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let dir = PathBuf::from(&inst.game_dir).join(subfolder);
    let current_path = dir.join(filename);
    if !current_path.exists() {
        return Err(format!("Content file {filename} does not exist"));
    }

    let target_filename = if enabled {
        filename.trim_end_matches(".disabled").to_string()
    } else if !filename.ends_with(".disabled") {
        format!("{filename}.disabled")
    } else {
        filename.to_string()
    };

    let target_path = dir.join(target_filename);
    fs::rename(current_path, target_path).map_err(|e| e.to_string())
}

pub fn delete_instance_content(instance_id: &str, subfolder: &str, filename: &str) -> Result<(), String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let file_path = PathBuf::from(&inst.game_dir).join(subfolder).join(filename);
    if file_path.exists() {
        if file_path.is_dir() {
            fs::remove_dir_all(file_path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(file_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn duplicate_instance(instance_id: &str) -> Result<Instance, String> {
    let instances = load_instances();
    let inst = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))?;

    let new_name = format!("{} (Copy)", inst.name);
    let slug = new_name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-");
    let slug = slug.trim_matches('-').to_string();

    let instances_dir = get_instances_dir();
    let mut candidate = instances_dir.join(&slug);
    let mut counter = 2;
    while candidate.exists() {
        candidate = instances_dir.join(format!("{slug}-{counter}"));
        counter += 1;
    }

    let src = PathBuf::from(&inst.game_dir);
    if src.exists() {
        copy_dir_all(&src, &candidate)?;
    } else {
        fs::create_dir_all(&candidate).map_err(|e| e.to_string())?;
    }

    let id = candidate
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let new_inst = Instance {
        id: id.clone(),
        name: new_name,
        edition: inst.edition,
        game_version: inst.game_version,
        loader: inst.loader,
        game_dir: candidate.to_string_lossy().to_string(),
        banner: inst.banner,
        icon: inst.icon,
        memory_min_mb: inst.memory_min_mb,
        memory_max_mb: inst.memory_max_mb,
        java_args: inst.java_args,
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        last_played: None,
        loader_version: None,
    };

    let _ = write_death_profile(&new_inst);
    let mut all = load_instances();
    all.push(new_inst.clone());
    let _ = save_instances(&all);
    Ok(new_inst)
}
