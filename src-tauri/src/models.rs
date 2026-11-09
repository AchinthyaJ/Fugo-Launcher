use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EditionType {
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "java")]
    Java,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub edition: EditionType,
    pub game_version: String,
    pub loader: String, // "horizon-rust", "vanilla", "fabric", "forge", "neoforge", "quilt"
    pub game_dir: String,
    pub banner: String, // asset url or filename
    pub icon: String,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub java_args: String,
    pub created_at: String,
    pub last_played: Option<String>,
    /// The loader build the player picked (Fabric or Quilt); None uses the newest stable one.
    #[serde(default)]
    pub loader_version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub edition: EditionType,
    pub game_version: String,
    pub loader: String,
    pub banner: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub is_offline: bool,
    pub skin_preset: Option<String>,
    pub cape_preset: Option<String>,
    /// Skin picked from a player lookup or an imported file: a texture URL or a data URL.
    #[serde(default)]
    pub skin_url: Option<String>,
    /// Cape picked from a player lookup or a preset, as a PNG data URL.
    #[serde(default)]
    pub cape_data: Option<String>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LauncherSettings {
    pub default_ram_mb: u32,
    pub discrete_gpu: bool,
    pub horizon_resources_path: String,
    pub horizon_binary_path: String,
    pub custom_java_path: Option<String>,
    pub close_on_launch: bool,
    pub theme: String, // "obsidian", "glass", "aurora"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthHit>,
    pub total_hits: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModrinthHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: String,
    pub server_side: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub author: String,
    pub slug: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstalledMod {
    pub filename: String,
    pub name: String,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogMessage {
    pub instance_id: String,
    pub text: String,
    pub level: String, // "info", "warn", "error"
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub total_memory_mb: u64,
    pub has_nvidia_gpu: bool,
    pub horizon_ready: bool,
    pub horizon_binary_exists: bool,
    pub horizon_resources_exist: bool,
    pub java_versions: Vec<String>,
}
