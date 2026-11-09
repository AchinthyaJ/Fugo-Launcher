//! The Rust Edition: Horizon's latest release (game binary, preparer, lock file), the game files
//! prepared from the player's own Minecraft install, and where those live.
//!
//! Horizon ships no Minecraft files. The launcher installs Minecraft 26.3 from Mojang and runs the
//! release's `horizon-prepare` on it, the same steps the Horizon project documents.

use super::install::{self, Progress};
use super::{data_root, err, http, mc_root, version_json, versions_dir};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

pub const REPO: &str = "horizon-rust/horizon";

/// A line-by-line log sink, shareable with blocking threads.
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

fn archive_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("horizon-linux-x86_64.tar.gz"),
        ("windows", "x86_64") => Some("horizon-windows-x64.zip"),
        ("macos", "aarch64") => Some("horizon-macos-arm64.zip"),
        _ => None,
    }
}

pub fn binary_name() -> &'static str {
    if cfg!(windows) { "horizon-client.exe" } else { "horizon-client" }
}

fn prepare_name() -> &'static str {
    if cfg!(windows) { "horizon-prepare.exe" } else { "horizon-prepare" }
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Lock {
    minecraft: LockMinecraft,
}

#[derive(Deserialize)]
struct LockMinecraft {
    version: String,
    java_major: u32,
    manifest_sha1: String,
}

/// Where a release is unpacked: `<data>/horizon/<tag>/`.
pub fn releases_dir() -> PathBuf {
    data_root().join("horizon")
}

/// Prepared game files inside a release folder.
pub fn resources_in(release: &Path) -> PathBuf {
    release.join(".local").join("horizon-client").join("26.3")
}

pub fn is_prepared(resources: &Path) -> bool {
    resources.join("horizon-resources.json").is_file()
}

/// Drops the archive's top folder and refuses paths that climb out of the destination.
fn stripped(path: &Path) -> Option<PathBuf> {
    let rest: PathBuf = path.components().skip(1).collect();
    if rest.as_os_str().is_empty() || rest.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir)) {
        return None;
    }
    Some(rest)
}

fn extract_release(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(err)?;
    if archive.to_string_lossy().ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).map_err(err)?).map_err(err)?;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(err)?;
            let Some(path) = entry.enclosed_name() else { continue };
            let Some(rest) = stripped(&path) else { continue };
            let out = dest.join(rest);
            if entry.is_dir() {
                std::fs::create_dir_all(&out).map_err(err)?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(err)?;
            }
            let mut writer = std::fs::File::create(&out).map_err(err)?;
            std::io::copy(&mut entry, &mut writer).map_err(err)?;
        }
    } else {
        let decoder = flate2::read::GzDecoder::new(std::fs::File::open(archive).map_err(err)?);
        for entry in tar::Archive::new(decoder).entries().map_err(err)? {
            let mut entry = entry.map_err(err)?;
            let path = entry.path().map_err(err)?.into_owned();
            let Some(rest) = stripped(&path) else { continue };
            entry.unpack(dest.join(rest)).map_err(err)?;
        }
    }
    Ok(())
}

/// Latest Horizon release, downloaded, checked against SHA256SUMS and unpacked. Cached per tag.
pub async fn ensure_release(client: &reqwest::Client, log: &Log) -> Result<PathBuf, String> {
    let archive = archive_name().ok_or("Horizon has no release for this platform yet")?;
    let release: Release = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not check the latest Horizon release: {e}"))?
        .json()
        .await
        .map_err(err)?;
    let tag = release.tag_name.clone();
    let dir = releases_dir().join(&tag);
    if dir.join(binary_name()).is_file() && dir.join(prepare_name()).is_file() {
        return Ok(dir);
    }

    let asset_url = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
    let url = asset_url(archive).ok_or_else(|| format!("Horizon {tag} has no {archive}"))?;
    let sums_url = asset_url("SHA256SUMS").ok_or_else(|| format!("Horizon {tag} has no SHA256SUMS"))?;
    let sums = client
        .get(sums_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(err)?
        .text()
        .await
        .map_err(err)?;
    let expected = sums
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == archive).then(|| hash.to_string())
        })
        .ok_or_else(|| format!("SHA256SUMS does not list {archive}"))?;

    log(&format!("Downloading Horizon {tag}..."));
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(err)?
        .bytes()
        .await
        .map_err(err)?;
    if !format!("{:x}", Sha256::digest(&bytes)).eq_ignore_ascii_case(&expected) {
        return Err(format!("The Horizon {tag} download failed its checksum"));
    }

    let root = releases_dir();
    tokio::fs::create_dir_all(&root).await.map_err(err)?;
    let archive_path = root.join(format!("{tag}.archive"));
    let partial = root.join(format!("{tag}.partial"));
    tokio::fs::write(&archive_path, &bytes).await.map_err(err)?;
    let _ = tokio::fs::remove_dir_all(&partial).await;
    let (from, to) = (archive_path.clone(), partial.clone());
    tokio::task::spawn_blocking(move || extract_release(&from, &to))
        .await
        .map_err(err)??;
    let _ = tokio::fs::remove_file(&archive_path).await;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::rename(&partial, &dir).await.map_err(err)?;
    Ok(dir)
}

/// The Minecraft server jar the preparer needs, downloaded from Mojang and checked.
async fn server_jar(client: &reqwest::Client, version: &str) -> Result<PathBuf, String> {
    let json = version_json(client, version).await?;
    let server = json
        .pointer("/downloads/server")
        .ok_or_else(|| format!("Minecraft {version} has no server download"))?;
    let dest = versions_dir().join(version).join("server.jar");
    install::download(
        client,
        server.get("url").and_then(|u| u.as_str()).unwrap_or_default(),
        &dest,
        server.get("sha1").and_then(|s| s.as_str()),
    )
    .await?;
    Ok(dest)
}

/// Runs the release's preparer and streams its output to `log`.
async fn run_prepare(release: &Path, java_home: &Path, server: &Path, log: &Log) -> Result<(), String> {
    let mut cmd = std::process::Command::new(release.join(prepare_name()));
    cmd.current_dir(release)
        .env("JAVA_HOME", java_home)
        .args(["--prepare", "--root"])
        .arg(release)
        .arg("--minecraft-dir")
        .arg(mc_root())
        .arg("--server-jar")
        .arg(server)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let log = log.clone();
    let succeeded = tokio::task::spawn_blocking(move || -> Result<bool, String> {
        let mut child = cmd.spawn().map_err(|e| format!("Could not start horizon-prepare: {e}"))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let err_log = log.clone();
        let errors = std::thread::spawn(move || {
            if let Some(stream) = stderr {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    err_log(&line);
                }
            }
        });
        if let Some(stream) = stdout {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                log(&line);
            }
        }
        let _ = errors.join();
        Ok(child.wait().map_err(err)?.success())
    })
    .await
    .map_err(err)??;
    if succeeded {
        Ok(())
    } else {
        Err("horizon-prepare did not finish; see the log above".into())
    }
}

/// Prepared game files for a release. Installs Minecraft from Mojang and prepares them once.
pub async fn ensure_resources(
    client: &reqwest::Client,
    release: &Path,
    progress: &Progress,
    log: &Log,
) -> Result<PathBuf, String> {
    let resources = resources_in(release);
    if is_prepared(&resources) {
        return Ok(resources);
    }
    let lock: Lock = serde_json::from_slice(
        &tokio::fs::read(release.join("research-lock.json")).await.map_err(err)?,
    )
    .map_err(err)?;
    let version = lock.minecraft.version.clone();

    // The preparer accepts only the version file this release pins. Mojang's current one can
    // differ (its asset index), so the pinned file is fetched by its hash and put in place first.
    let pinned_url = format!(
        "https://piston-meta.mojang.com/v1/packages/{}/{version}.json",
        lock.minecraft.manifest_sha1
    );
    let version_file = versions_dir().join(&version).join(format!("{version}.json"));
    install::download(client, &pinned_url, &version_file, Some(&lock.minecraft.manifest_sha1)).await?;

    log(&format!("Installing Minecraft {version} from Mojang..."));
    install::install(client, &version, &release.join("game"), progress).await?;
    let server = server_jar(client, &version).await?;

    log(&format!("Finding Java {} for preparing...", lock.minecraft.java_major));
    let java = super::java::ensure_java(client, lock.minecraft.java_major, None, progress).await?;
    let java_home = java
        .parent()
        .and_then(Path::parent)
        .ok_or("Java was found in an unexpected place")?
        .to_path_buf();

    log("Preparing the game files from your Minecraft (a minute or two)...");
    progress("Preparing game files", 0, 1);
    run_prepare(release, &java_home, &server, log).await?;
    if !is_prepared(&resources) {
        return Err("Preparing finished, but the game files are missing".into());
    }
    progress("Preparing game files", 1, 1);
    Ok(resources)
}

/// The release, prepared, ready for a launch.
pub async fn ready(client: &reqwest::Client, progress: &Progress, log: &Log) -> Result<(PathBuf, PathBuf), String> {
    let release = ensure_release(client, log).await?;
    let resources = ensure_resources(client, &release, progress, log).await?;
    Ok((release.join(binary_name()), resources))
}

/// The HTTP client the Rust Edition uses.
pub fn client() -> reqwest::Client {
    http()
}

#[cfg(test)]
mod pipeline_check {
    use super::*;

    /// Downloads the latest release and prepares its game files from Mojang's Minecraft.
    /// Ignored by default: it downloads the release, Minecraft and a JDK. Run with `--ignored --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn prepares_the_latest_release() {
        let client = http();
        let log: Log = Arc::new(|m: &str| println!("log: {m}"));
        let progress = |stage: &str, done: usize, total: usize| println!("progress: {stage} {done}/{total}");
        let (binary, resources) = ready(&client, &progress, &log).await.unwrap();
        println!("binary: {}", binary.display());
        println!("resources: {}", resources.display());
        assert!(binary.is_file());
        assert!(is_prepared(&resources));
    }
}
