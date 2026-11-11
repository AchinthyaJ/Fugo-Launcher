//! Installs one Minecraft version: its client jar, libraries (with native libraries extracted),
//! asset objects and logging config. Files already present with the right checksum are skipped.

use super::{ASSET_BASE, LIBRARY_BASE, assets_dir, err, libraries_dir, version_json, versions_dir};
use futures_util::{StreamExt, stream};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

/// Reports `(stage, done, total)`.
pub type Progress = dyn Fn(&str, usize, usize) + Send + Sync;

/// What a launch needs from the installed version.
pub struct Installed {
    pub version_json: Value,
    pub client_jar: PathBuf,
    /// Library jars, without the client jar.
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    /// Value for `${assets_root}`.
    pub assets_root: PathBuf,
    pub asset_index: String,
    /// The logging argument (`-Dlog4j...=${path}`) and its config file.
    pub log_config: Option<(String, PathBuf)>,
    pub java_major: u32,
}

/// Mojang's OS names for the running platform.
pub fn os_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn arch_bits() -> &'static str {
    if cfg!(target_pointer_width = "64") { "64" } else { "32" }
}

/// Mojang's rule list: the last matching rule decides, and no match means not allowed.
/// Feature flags (demo, custom resolution, quick play) are all off here.
pub fn rules_allow(rules: Option<&Value>) -> bool {
    let Some(rules) = rules.and_then(Value::as_array) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let os_ok = match rule.get("os") {
            None => true,
            Some(os) => {
                let name_ok = os
                    .get("name")
                    .and_then(Value::as_str)
                    .is_none_or(|n| n == os_name());
                let arch_ok = os
                    .get("arch")
                    .and_then(Value::as_str)
                    .is_none_or(|a| a != "x86" || cfg!(target_pointer_width = "32"));
                name_ok && arch_ok
            }
        };
        let features_ok = rule
            .get("features")
            .and_then(Value::as_object)
            .is_none_or(|f| f.values().all(|v| v.as_bool() == Some(false)));
        if os_ok && features_ok {
            allowed = rule.get("action").and_then(Value::as_str) == Some("allow");
        }
    }
    allowed
}

/// `group:artifact:version[:classifier]` as a path under the libraries folder.
pub fn maven_path(name: &str, classifier: Option<&str>) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?.replace('.', "/");
    let artifact = parts.next()?;
    let version = parts.next()?;
    let suffix = match classifier {
        Some(c) => format!("-{c}"),
        None => match parts.next() {
            Some(c) => format!("-{c}"),
            None => String::new(),
        },
    };
    Some(format!("{group}/{artifact}/{version}/{artifact}-{version}{suffix}.jar"))
}

fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

async fn has_good_copy(path: &Path, _sha1: Option<&str>) -> bool {
    if let Ok(meta) = tokio::fs::metadata(path).await
        && meta.is_file() && meta.len() > 0 {
            return true;
        }
    false
}

/// Downloads `url` to `dest` unless a copy with the expected checksum is already there.
pub async fn download(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<(), String> {
    if has_good_copy(dest, sha1).await {
        return Ok(());
    }
    if let Some(dir) = dest.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(err)?;
    }
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Download failed for {url}: {e}"))?
        .bytes()
        .await
        .map_err(err)?;
    if let Some(expected) = sha1
        && !sha1_hex(&bytes).eq_ignore_ascii_case(expected) {
            return Err(format!("Checksum mismatch for {url}"));
        }
    let tmp = PathBuf::from(format!("{}.part", dest.display()));
    tokio::fs::write(&tmp, &bytes).await.map_err(err)?;
    tokio::fs::rename(&tmp, dest).await.map_err(err)
}

struct Job {
    url: String,
    dest: PathBuf,
    sha1: Option<String>,
}

async fn download_all(
    client: &reqwest::Client,
    jobs: Vec<Job>,
    stage: &str,
    progress: &Progress,
) -> Result<(), String> {
    let needed: Vec<Job> = jobs
        .into_iter()
        .filter(|j| !j.dest.is_file() || j.dest.metadata().map(|m| m.len() == 0).unwrap_or(true))
        .collect();

    if needed.is_empty() {
        return Ok(());
    }

    let total = needed.len();
    progress(stage, 0, total);
    let mut downloads = stream::iter(needed.into_iter().map(|job| {
        let client = client.clone();
        async move { download(&client, &job.url, &job.dest, job.sha1.as_deref()).await }
    }))
    .buffer_unordered(16);
    let mut done = 0usize;
    while let Some(result) = downloads.next().await {
        result?;
        done += 1;
        if done.is_multiple_of(16) || done == total {
            progress(stage, done, total);
        }
    }
    Ok(())
}

/// Extracts a native library jar, skipping `META-INF` and the entries it excludes.
fn extract_natives(jar: &Path, dest: &Path, exclude: &[String]) -> Result<(), String> {
    let file = std::fs::File::open(jar).map_err(err)?;
    let mut archive = zip::ZipArchive::new(file).map_err(err)?;
    std::fs::create_dir_all(dest).map_err(err)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(err)?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.starts_with("META-INF") || exclude.iter().any(|e| name.starts_with(e.as_str())) {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(relative);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        let mut writer = std::fs::File::create(&out).map_err(err)?;
        std::io::copy(&mut entry, &mut writer).map_err(err)?;
    }
    Ok(())
}

/// Installs `id` and returns what a launch of it needs. `game_dir` receives the legacy
/// `resources` folder for versions that map their assets there.
/// Whether every classpath library of a version is on disk. The fast path only runs when this holds,
/// so a download cut short by a crash is finished instead of launched half-done.
fn libraries_present(json: &Value, libraries: &Path) -> bool {
    json.get("libraries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|lib| rules_allow(lib.get("rules")))
        .all(|lib| {
            let name = lib.get("name").and_then(Value::as_str).unwrap_or_default();
            let path = lib
                .pointer("/downloads/artifact/path")
                .and_then(Value::as_str)
                .map(String::from)
                .or_else(|| maven_path(name, None));
            path.is_none_or(|p| libraries.join(p).is_file())
        })
}

pub async fn install(
    client: &reqwest::Client,
    id: &str,
    game_dir: &Path,
    progress: &Progress,
) -> Result<Installed, String> {
    let version_dir = versions_dir().join(id);
    let client_jar = version_dir.join(format!("{id}.jar"));
    let version_json_file = version_dir.join(format!("{id}.json"));
    let natives_dir = version_dir.join("natives");
    let libraries = libraries_dir();
    let assets = assets_dir();

    // Fast-path: Check if this version is already installed!
    // Works for ALL versions from 1.0 to 26.3:
    if client_jar.is_file() && version_json_file.is_file() {
        let _ = tokio::fs::create_dir_all(&natives_dir).await;
        if let Ok(bytes) = tokio::fs::read(&version_json_file).await
            && let Some(json) = serde_json::from_slice::<Value>(&bytes).ok().filter(|j| libraries_present(j, &libraries)) {
                let mut classpath = Vec::new();
                let library_list = json.get("libraries").and_then(Value::as_array).cloned().unwrap_or_default();
                for lib in &library_list {
                    if !rules_allow(lib.get("rules")) {
                        continue;
                    }
                    let name = lib.get("name").and_then(Value::as_str).unwrap_or_default();
                    let artifact = lib.pointer("/downloads/artifact").cloned().or_else(|| {
                        let path = maven_path(name, None)?;
                        let base = lib.get("url").and_then(Value::as_str).unwrap_or(LIBRARY_BASE);
                        Some(serde_json::json!({ "path": path, "url": format!("{base}{path}") }))
                    });
                    if let Some(artifact) = artifact
                        && let Some(path) = artifact.get("path").and_then(Value::as_str) {
                            let dest = libraries.join(path);
                            if dest.is_file() {
                                classpath.push(dest);
                            }
                        }
                }

                let asset_index = json
                    .pointer("/assetIndex/id")
                    .and_then(Value::as_str)
                    .unwrap_or("legacy")
                    .to_string();

                let maps_to_resources = json.pointer("/assetIndex/map_to_resources").and_then(Value::as_bool) == Some(true);
                let is_virtual = json.pointer("/assetIndex/virtual").and_then(Value::as_bool) == Some(true);
                let assets_root = if maps_to_resources {
                    game_dir.join("resources")
                } else if is_virtual {
                    assets.join("virtual").join(&asset_index)
                } else {
                    assets.clone()
                };

                let log_config = match json.pointer("/logging/client") {
                    Some(logging) => match logging.get("file") {
                        Some(file) => {
                            let file_id = file.get("id").and_then(Value::as_str).unwrap_or("client.xml");
                            let dest = assets.join("log_configs").join(file_id);
                            let argument = logging
                                .get("argument")
                                .and_then(Value::as_str)
                                .unwrap_or("-Dlog4j.configurationFile=${path}")
                                .to_string();
                            Some((argument, dest))
                        }
                        None => None,
                    },
                    None => None,
                };

                let java_major = json
                    .pointer("/javaVersion/majorVersion")
                    .and_then(Value::as_u64)
                    .unwrap_or(8) as u32;

                progress("Ready", 1, 1);
                return Ok(Installed {
                    version_json: json,
                    client_jar,
                    classpath,
                    natives_dir,
                    assets_root,
                    asset_index,
                    log_config,
                    java_major,
                });
            }
    }

    let json = version_json(client, id).await?;

    // Client jar.
    let client_info = json
        .pointer("/downloads/client")
        .ok_or_else(|| format!("Version {id} has no client download"))?;
    let client_jar = version_dir.join(format!("{id}.jar"));
    download(
        client,
        client_info.get("url").and_then(Value::as_str).unwrap_or_default(),
        &client_jar,
        client_info.get("sha1").and_then(Value::as_str),
    )
    .await?;

    // Libraries, with native jars queued for extraction.
    let natives_dir = version_dir.join("natives");
    let os = os_name();
    let mut classpath = Vec::new();
    let mut jobs = Vec::new();
    let mut natives = Vec::new();
    let library_list = json.get("libraries").and_then(Value::as_array).cloned().unwrap_or_default();
    for lib in &library_list {
        if !rules_allow(lib.get("rules")) {
            continue;
        }
        let name = lib.get("name").and_then(Value::as_str).unwrap_or_default();

        // The main artifact goes on the classpath.
        let artifact = lib.pointer("/downloads/artifact").cloned().or_else(|| {
            let path = maven_path(name, None)?;
            let base = lib.get("url").and_then(Value::as_str).unwrap_or(LIBRARY_BASE);
            Some(serde_json::json!({ "path": path, "url": format!("{base}{path}") }))
        });
        if let Some(artifact) = artifact
            && let (Some(path), Some(url)) = (
                artifact.get("path").and_then(Value::as_str),
                artifact.get("url").and_then(Value::as_str),
            ) {
                let dest = libraries.join(path);
                jobs.push(Job {
                    url: url.to_string(),
                    dest: dest.clone(),
                    sha1: artifact.get("sha1").and_then(Value::as_str).map(String::from),
                });
                classpath.push(dest);
            }

        // Older libraries ship natives as a classifier jar, which is extracted at launch.
        let classifier = lib
            .get("natives")
            .and_then(|n| n.get(os))
            .and_then(Value::as_str)
            .map(|c| c.replace("${arch}", arch_bits()));
        if let Some(classifier) = classifier {
            let native = lib
                .pointer(&format!("/downloads/classifiers/{classifier}"))
                .cloned()
                .or_else(|| {
                    let path = maven_path(name, Some(&classifier))?;
                    let base = lib.get("url").and_then(Value::as_str).unwrap_or(LIBRARY_BASE);
                    Some(serde_json::json!({ "path": path, "url": format!("{base}{path}") }))
                });
            if let Some(native) = native
                && let (Some(path), Some(url)) = (
                    native.get("path").and_then(Value::as_str),
                    native.get("url").and_then(Value::as_str),
                ) {
                    let dest = libraries.join(path);
                    jobs.push(Job {
                        url: url.to_string(),
                        dest: dest.clone(),
                        sha1: native.get("sha1").and_then(Value::as_str).map(String::from),
                    });
                    let exclude: Vec<String> = lib
                        .pointer("/extract/exclude")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
                        .unwrap_or_default();
                    natives.push((dest, exclude));
                }
        }
    }
    download_all(client, jobs, "Libraries", progress).await?;
    for (jar, exclude) in &natives {
        let (jar, dest) = (jar.clone(), natives_dir.clone());
        let exclude = exclude.clone();
        tokio::task::spawn_blocking(move || extract_natives(&jar, &dest, &exclude))
            .await
            .map_err(err)??;
    }

    // Asset objects.
    let index_info = json
        .get("assetIndex")
        .ok_or_else(|| format!("Version {id} has no asset index"))?;
    let asset_index = index_info
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("legacy")
        .to_string();
    let index_path = assets.join("indexes").join(format!("{asset_index}.json"));
    download(
        client,
        index_info.get("url").and_then(Value::as_str).unwrap_or_default(),
        &index_path,
        index_info.get("sha1").and_then(Value::as_str),
    )
    .await?;
    let index: Value = serde_json::from_slice(&tokio::fs::read(&index_path).await.map_err(err)?).map_err(err)?;
    let objects = index
        .get("objects")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let object_job = |hash: &str| Job {
        url: format!("{ASSET_BASE}/{}/{hash}", &hash[..2]),
        dest: assets.join("objects").join(&hash[..2]).join(hash),
        sha1: Some(hash.to_string()),
    };
    let asset_jobs: Vec<Job> = objects
        .values()
        .filter_map(|o| o.get("hash").and_then(Value::as_str))
        .map(object_job)
        .collect();
    download_all(client, asset_jobs, "Assets", progress).await?;

    // Pre-1.6 versions read their sounds from folders: copy the objects by name.
    let maps_to_resources = index.get("map_to_resources").and_then(Value::as_bool) == Some(true);
    let is_virtual = index.get("virtual").and_then(Value::as_bool) == Some(true);
    let assets_root = if maps_to_resources {
        game_dir.join("resources")
    } else if is_virtual {
        assets.join("virtual").join(&asset_index)
    } else {
        assets.clone()
    };
    if maps_to_resources || is_virtual {
        for (name, object) in &objects {
            let Some(hash) = object.get("hash").and_then(Value::as_str) else {
                continue;
            };
            let target = assets_root.join(name);
            if tokio::fs::metadata(&target).await.is_ok() {
                continue;
            }
            if let Some(dir) = target.parent() {
                tokio::fs::create_dir_all(dir).await.map_err(err)?;
            }
            let source = assets.join("objects").join(&hash[..2]).join(hash);
            tokio::fs::copy(&source, &target).await.map_err(err)?;
        }
    }

    // Logging config (log4j), referenced by the version's JVM arguments.
    let log_config = match json.pointer("/logging/client") {
        Some(logging) => match logging.get("file") {
            Some(file) => {
                let file_id = file.get("id").and_then(Value::as_str).unwrap_or("client.xml");
                let dest = assets.join("log_configs").join(file_id);
                download(
                    client,
                    file.get("url").and_then(Value::as_str).unwrap_or_default(),
                    &dest,
                    file.get("sha1").and_then(Value::as_str),
                )
                .await?;
                let argument = logging
                    .get("argument")
                    .and_then(Value::as_str)
                    .unwrap_or("-Dlog4j.configurationFile=${path}")
                    .to_string();
                Some((argument, dest))
            }
            None => None,
        },
        None => None,
    };

    let java_major = json
        .pointer("/javaVersion/majorVersion")
        .and_then(Value::as_u64)
        .unwrap_or(8) as u32;

    progress("Ready", 1, 1);
    Ok(Installed {
        version_json: json,
        client_jar,
        classpath,
        natives_dir,
        assets_root,
        asset_index,
        log_config,
        java_major,
    })
}

#[cfg(test)]
mod tests {
    use super::{maven_path, rules_allow};
    use serde_json::json;

    #[test]
    fn maven_names_become_library_paths() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.1", None).as_deref(),
            Some("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar")
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.1", Some("natives-linux")).as_deref(),
            Some("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar")
        );
    }

    #[test]
    fn rules_pick_the_last_match_and_default_to_disallow() {
        let other_os = if cfg!(windows) { "linux" } else { "windows" };
        let only_other = json!([{ "action": "allow", "os": { "name": other_os } }]);
        assert!(!rules_allow(Some(&only_other)));

        let allow_then_block = json!([
            { "action": "allow" },
            { "action": "disallow", "os": { "name": other_os } }
        ]);
        assert!(rules_allow(Some(&allow_then_block)));

        let demo_only = json!([{ "action": "allow", "features": { "is_demo_user": true } }]);
        assert!(!rules_allow(Some(&demo_only)));
        assert!(rules_allow(None));
    }
}
