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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    /// Whether the picked skin uses slim (Alex-style) arms.
    #[serde(default)]
    pub skin_slim: bool,
    pub is_active: bool,
    /// Microsoft accounts only: the token that renews the sign-in. Kept on this computer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Microsoft accounts only: the Minecraft access token a launch hands to the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// When `access_token` stops working, in seconds since the Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token_expires: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LauncherSettings {
    pub default_ram_mb: u32,
    pub discrete_gpu: bool,
    pub horizon_resources_path: String,
    pub horizon_binary_path: String,
    pub custom_java_path: Option<String>,
    /// JVM flags for every Java run; the instance's own flags come after them.
    #[serde(default = "default_jvm_args")]
    pub jvm_args: String,
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    /// Lighter visuals (no blur, no transitions), as the older launcher's Performance mode.
    #[serde(default)]
    pub performance_mode: bool,
    /// What the launcher does once a game starts: "close", "minimize" or "background".
    #[serde(default = "default_after_launch")]
    pub after_launch: String,
    /// Keeps the Rust Edition's Nightly up to date in the background (`mc::horizon::spawn_updater`).
    #[serde(default = "default_true")]
    pub horizon_auto_update: bool,
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

pub fn default_jvm_args() -> String {
    "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -Dsun.stdout.encoding=UTF-8".to_string()
}

pub fn default_window_width() -> u32 {
    854
}

pub fn default_window_height() -> u32 {
    480
}

pub fn default_after_launch() -> String {
    "close".to_string()
}

pub fn default_true() -> bool {
    true
}
