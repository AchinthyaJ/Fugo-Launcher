//! The Rust Edition: Horizon's latest release (game binary, preparer, lock file), the game files
//! prepared from the player's own Minecraft install, and where those live.
//!
//! Horizon ships no Minecraft files. The launcher installs Minecraft 26.3 from Mojang and runs the
//! release's `horizon-prepare` on it, the same steps the Horizon project documents.

use super::install::{self, Progress};
use super::{data_root, err, http, mc_root, version_json, versions_dir};
use crate::models::{EditionType, LauncherSettings};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};

pub const REPO: &str = "horizon-rust/horizon";

/// The release the Rust Edition follows. Horizon ships only its Nightly now (docs/release-flow.md):
/// a pre-release under this one tag, so `releases/latest` would hand back an old release instead.
pub const CHANNEL: &str = "nightly";

/// How often the background updater checks for a new Nightly.
pub const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);

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

/// The Nightly, downloaded, checked against its SHA256SUMS and unpacked into its own folder.
/// Each build is kept under its archive's digest, since every Nightly carries the same tag.
pub async fn ensure_release(client: &reqwest::Client, log: &Log) -> Result<PathBuf, String> {
    let archive = archive_name().ok_or("Horizon has no release for this platform yet")?;
    let release: Release = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/tags/{CHANNEL}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not check the Horizon Nightly: {e}"))?
        .json()
        .await
        .map_err(err)?;
    let tag = release.tag_name.clone();

    let asset_url = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
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
    let expected = archive_digest(&sums, archive).ok_or_else(|| format!("SHA256SUMS does not list {archive}"))?;
    let dir = releases_dir().join(version_folder(&expected));
    if dir.join(binary_name()).is_file() && dir.join(prepare_name()).is_file() {
        return Ok(dir);
    }

    let url = asset_url(archive).ok_or_else(|| format!("Horizon {tag} has no {archive}"))?;
    log("Downloading the Horizon Nightly...");
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
    let _install = install_lock().lock().await;
    let release = ensure_release(client, log).await?;
    let resources = ensure_resources(client, &release, progress, log).await?;
    Ok((release.join(binary_name()), resources))
}

/// The HTTP client the Rust Edition uses.
pub fn client() -> reqwest::Client {
    http()
}

/// One install at a time: the background update and a launch both fetch and prepare the same files.
fn install_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// The SHA-256 that a SHA256SUMS file lists for `archive`, lowercase, when it is well formed.
pub fn archive_digest(sums: &str, archive: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        let well_formed = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
        (name == archive && well_formed).then(|| hash.to_ascii_lowercase())
    })
}

/// The folder a build is unpacked into: the channel and the first twelve digits of its archive.
pub fn version_folder(digest: &str) -> String {
    format!("{CHANNEL}-{}", &digest[..12])
}

/// The version folders to delete: every one except the `keep` newest by modification time.
pub fn stale_folders(mut folders: Vec<(String, SystemTime)>, keep: usize) -> Vec<String> {
    folders.sort_by(|a, b| b.1.cmp(&a.1));
    folders.into_iter().skip(keep).map(|(name, _)| name).collect()
}

/// Deletes the Nightly folders beyond the two newest. The one before the newest stays, so a launch
/// that has just picked it still finds its files.
fn prune_versions() {
    let Ok(entries) = std::fs::read_dir(releases_dir()) else { return };
    let folders = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("nightly-") && e.path().is_dir())
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((e.file_name().to_string_lossy().into_owned(), modified))
        })
        .collect();
    for name in stale_folders(folders, 2) {
        let _ = std::fs::remove_dir_all(releases_dir().join(name));
    }
}

/// Whether the player has used the Rust Edition: an instance plays it, or a build is installed.
fn rust_edition_used() -> bool {
    crate::instance_manager::load_instances()
        .iter()
        .any(|i| matches!(i.edition, EditionType::Rust))
        || std::fs::read_dir(releases_dir())
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("nightly-"))
}

/// A Horizon the player set in Settings and that is prepared: the launch uses it, so no update runs.
fn custom_build_in_use(settings: &LauncherSettings) -> bool {
    !settings.horizon_binary_path.is_empty()
        && Path::new(&settings.horizon_binary_path).is_file()
        && is_prepared(Path::new(&settings.horizon_resources_path))
}

/// One pass of the background updater: fetches and prepares the Nightly when the player keeps
/// automatic updates on, has used the Rust Edition and runs no Horizon of their own. Reports
/// through the `horizon-update` event.
async fn check_once(app: &AppHandle) {
    let settings = crate::settings::load_settings();
    if !settings.horizon_auto_update || custom_build_in_use(&settings) || !rust_edition_used() {
        return;
    }
    let _ = app.emit("horizon-update", json!({ "stage": "checking" }));
    let client = http();
    let log: Log = Arc::new(|line: &str| eprintln!("horizon update: {line}"));
    let progress = |_: &str, _: usize, _: usize| {};
    match ready(&client, &progress, &log).await {
        Ok((binary, _)) => {
            if crate::launcher_engine::get_running_instances().is_empty() {
                prune_versions();
            }
            let version = binary
                .parent()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let _ = app.emit("horizon-update", json!({ "stage": "ready", "version": version }));
        }
        Err(message) => {
            let _ = app.emit("horizon-update", json!({ "stage": "failed", "message": message }));
        }
    }
}

/// Starts the background updater on its own thread: a check now, then one every [`CHECK_EVERY`].
pub fn spawn_updater(app: AppHandle) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(e) => {
                eprintln!("horizon update: could not start its worker: {e}");
                return;
            }
        };
        loop {
            runtime.block_on(check_once(&app));
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

#[cfg(test)]
mod update_tests {
    use super::*;

    const SUMS: &str = "\
da05362420ce861af1cb641e013678e12cee6a97144224573950be6ef756b4e3  horizon-linux-x86_64.tar.gz
d4d3aba3e26fdb85a7b56c643f3cd2c3a4a198b0bd757e614f1efdf0654c75dc *horizon-server-linux-x86_64.tar.gz
";

    #[test]
    fn the_digest_of_each_archive_is_found_by_its_name() {
        assert_eq!(
            archive_digest(SUMS, "horizon-linux-x86_64.tar.gz").as_deref(),
            Some("da05362420ce861af1cb641e013678e12cee6a97144224573950be6ef756b4e3")
        );
        assert_eq!(
            archive_digest(SUMS, "horizon-server-linux-x86_64.tar.gz").as_deref(),
            Some("d4d3aba3e26fdb85a7b56c643f3cd2c3a4a198b0bd757e614f1efdf0654c75dc")
        );
        assert_eq!(archive_digest(SUMS, "horizon-windows-x64.zip"), None);
    }

    #[test]
    fn a_digest_that_is_not_sha256_is_refused() {
        assert_eq!(archive_digest("deadbeef  horizon-linux-x86_64.tar.gz\n", "horizon-linux-x86_64.tar.gz"), None);
    }

    #[test]
    fn each_nightly_gets_a_folder_of_its_own() {
        let digest = "da05362420ce861af1cb641e013678e12cee6a97144224573950be6ef756b4e3";
        assert_eq!(version_folder(digest), "nightly-da05362420ce");
    }

    #[test]
    fn only_the_two_newest_versions_are_kept() {
        let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        let folders = vec![
            ("nightly-old".to_string(), at(10)),
            ("nightly-new".to_string(), at(30)),
            ("nightly-middle".to_string(), at(20)),
        ];
        assert_eq!(stale_folders(folders, 2), vec!["nightly-old".to_string()]);
    }
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
