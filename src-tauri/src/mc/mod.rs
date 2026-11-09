//! Vanilla Minecraft for every release and snapshot: the version list, installing a version's
//! files, a Java runtime for it, and starting the game.

pub mod appearance;
pub mod args;
pub mod horizon;
pub mod install;
pub mod java;
pub mod launch;
pub mod forge;
pub mod loader;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const ASSET_BASE: &str = "https://resources.download.minecraft.net";
pub const LIBRARY_BASE: &str = "https://libraries.minecraft.net/";

pub fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub fn data_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fugo-launcher")
}

/// Shared Minecraft files (versions, libraries, assets), reused by every instance.
pub fn mc_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
}

pub fn versions_dir() -> PathBuf {
    mc_root().join("versions")
}

pub fn libraries_dir() -> PathBuf {
    mc_root().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    mc_root().join("assets")
}

pub fn runtimes_dir() -> PathBuf {
    data_root().join("runtimes")
}

/// Instance folders may be written as `~/...`.
pub fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(rest),
        None => PathBuf::from(path),
    }
}

pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("fugo-launcher")
        .build()
        .expect("HTTP client")
}

#[derive(Serialize, Clone, Debug)]
pub struct GameVersion {
    pub id: String,
    /// "release", "snapshot", "old_beta" or "old_alpha".
    pub kind: String,
    pub release_time: String,
}

#[derive(Deserialize)]
struct ManifestEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<ManifestEntry>,
}

/// Mojang's version list. Falls back to the last saved copy when offline.
async fn manifest(client: &reqwest::Client) -> Result<Manifest, String> {
    let cache = mc_root().join("version_manifest_v2.json");
    let fetched = async {
        client
            .get(MANIFEST_URL)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await
    }
    .await;
    match fetched {
        Ok(bytes) => {
            if let Some(dir) = cache.parent() {
                let _ = tokio::fs::create_dir_all(dir).await;
            }
            let _ = tokio::fs::write(&cache, &bytes).await;
            serde_json::from_slice(&bytes).map_err(err)
        }
        Err(network) => match tokio::fs::read(&cache).await {
            Ok(saved) => serde_json::from_slice(&saved).map_err(err),
            Err(_) => Err(format!("Could not load the Minecraft version list: {network}")),
        },
    }
}

/// The version file (`<id>.json`) for one version, saved once fetched.
pub async fn version_json(client: &reqwest::Client, id: &str) -> Result<Value, String> {
    let path = versions_dir().join(id).join(format!("{id}.json"));
    if let Ok(saved) = tokio::fs::read(&path).await {
        if let Ok(json) = serde_json::from_slice(&saved) {
            return Ok(json);
        }
    }
    let list = manifest(client).await?;
    let entry = list
        .versions
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("Minecraft version {id} is not in Mojang's version list"))?;
    let bytes = client
        .get(&entry.url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(err)?
        .bytes()
        .await
        .map_err(err)?;
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(err)?;
    }
    tokio::fs::write(&path, &bytes).await.map_err(err)?;
    serde_json::from_slice(&bytes).map_err(err)
}

#[tauri::command]
pub async fn list_game_versions() -> Result<Vec<GameVersion>, String> {
    let client = http();
    let list = manifest(&client).await?;
    Ok(list
        .versions
        .into_iter()
        .map(|v| GameVersion {
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect())
}

/// Path helper for a file under a folder, used by install and launch.
pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
