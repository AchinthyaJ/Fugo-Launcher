//! Forge and NeoForge. The installer jar carries a version JSON and an install profile: libraries
//! and processors that patch the vanilla client. This runs the installer's processors the way the
//! official installer does, then leaves a version that launches like any other.

use super::{install, java, libraries_dir, loader, mc_root, version_json, versions_dir};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORGE_META: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/";
const NEO_META: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const NEO_MAVEN: &str = "https://maven.neoforged.net/releases/";
/// Written into a version folder once its processors have finished.
const DONE_MARKER: &str = ".horizon-complete";

pub fn is_installer_loader(loader: &str) -> bool {
    matches!(loader, "forge" | "neoforge")
}

fn maven_base(loader: &str) -> &'static str {
    if loader == "neoforge" { NEO_MAVEN } else { FORGE_MAVEN }
}

/// Builds that fit a Minecraft version, newest first. Forge's builds are "{mc}-{build}";
/// NeoForge's are numbered by Minecraft's minor and patch version ("1.21.1" gives "21.1.x").
pub async fn list_versions(client: &reqwest::Client, loader: &str, mc_version: &str) -> Result<Vec<String>, String> {
    let (meta, _) = meta_for(loader)?;
    let xml = client
        .get(meta)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(super::err)?
        .text()
        .await
        .map_err(super::err)?;
    let all = maven_versions(&xml);
    let mut fit: Vec<String> = if loader == "neoforge" {
        let parts: Vec<&str> = mc_version.split('.').collect();
        let prefix = match parts.as_slice() {
            ["1", minor] => format!("{minor}.0."),
            ["1", minor, patch, ..] => format!("{minor}.{patch}."),
            _ => return Err(format!("NeoForge has no builds for Minecraft {mc_version}")),
        };
        all.into_iter().filter(|v| v.starts_with(&prefix)).collect()
    } else {
        let prefix = format!("{mc_version}-");
        all.into_iter()
            .filter_map(|v| v.strip_prefix(&prefix).map(String::from))
            .collect()
    };
    fit.reverse();
    if fit.is_empty() {
        return Err(format!("No {loader} build is published for Minecraft {mc_version}"));
    }
    Ok(fit)
}

fn meta_for(loader: &str) -> Result<(&'static str, &'static str), String> {
    match loader {
        "forge" => Ok((FORGE_META, FORGE_MAVEN)),
        "neoforge" => Ok((NEO_META, NEO_MAVEN)),
        other => Err(format!("{other} is not an installer loader")),
    }
}

/// Every `<version>` in a maven-metadata.xml, in the order Maven lists them (oldest first).
fn maven_versions(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .map(|v| v.trim().to_string())
        .collect()
}

/// Full installer URL for a loader build. `version` is the build as `list_versions` gives it.
fn installer_url(loader: &str, mc_version: &str, version: &str) -> String {
    if loader == "neoforge" {
        format!("{NEO_MAVEN}net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar")
    } else {
        format!(
            "{FORGE_MAVEN}net/minecraftforge/forge/{mc_version}-{version}/forge-{mc_version}-{version}-installer.jar"
        )
    }
}

/// Installs a Forge or NeoForge build for `mc_version` and returns its version id.
pub async fn ensure_installed(
    client: &reqwest::Client,
    loader: &str,
    mc_version: &str,
    requested: Option<&str>,
    progress: &install::Progress,
) -> Result<String, String> {
    let build = match requested.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => v.to_string(),
        None => list_versions(client, loader, mc_version)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| format!("No {loader} build for Minecraft {mc_version}"))?,
    };

    let installer = mc_root()
        .join(".horizon-installers")
        .join(format!("{loader}-{mc_version}-{build}-installer.jar"));
    progress("Downloading installer", 0, 1);
    install::download(client, &installer_url(loader, mc_version, &build), &installer, None).await?;

    let (version_doc, profile) = read_installer(&installer)?;
    let id = version_doc
        .get("id")
        .and_then(Value::as_str)
        .ok_or("Installer version.json has no id")?
        .to_string();
    let dir = versions_dir().join(&id);
    if dir.join(DONE_MARKER).is_file() {
        return Ok(id);
    }

    // The vanilla client jar, then the merged version JSON, then its libraries.
    let parent = version_json(client, mc_version).await?;
    let client_info = parent
        .pointer("/downloads/client")
        .ok_or_else(|| format!("Version {mc_version} has no client download"))?;
    // Processors read the vanilla jar from its own version folder; the Forge version gets a copy.
    let vanilla_dir = versions_dir().join(mc_version);
    let vanilla_jar = vanilla_dir.join(format!("{mc_version}.jar"));
    install::download(
        client,
        client_info.get("url").and_then(Value::as_str).unwrap_or_default(),
        &vanilla_jar,
        client_info.get("sha1").and_then(Value::as_str),
    )
    .await?;
    tokio::fs::create_dir_all(&dir).await.map_err(super::err)?;
    tokio::fs::copy(&vanilla_jar, dir.join(format!("{id}.jar")))
        .await
        .map_err(super::err)?;
    let merged = loader::merge(&parent, &version_doc);
    let text = serde_json::to_string_pretty(&merged).map_err(super::err)?;
    tokio::fs::write(dir.join(format!("{id}.json")), text).await.map_err(super::err)?;

    // Libraries the installer ships itself (an empty URL) come out of its `maven/` folder.
    for lib in version_doc.get("libraries").and_then(Value::as_array).into_iter().flatten() {
        let Some(artifact) = lib.pointer("/downloads/artifact") else { continue };
        if artifact.get("url").and_then(Value::as_str) == Some("")
            && let Some(path) = artifact.get("path").and_then(Value::as_str) {
                // A processor may be what makes it (the patched client), so a miss here is not an error yet.
                let _ = ensure_library(client, &installer, path, None, loader).await;
            }
    }

    // The processors make the patched client jar that the version lists as a library, so they
    // run before the library download.
    let java_major = parent
        .pointer("/javaVersion/majorVersion")
        .and_then(Value::as_u64)
        .unwrap_or(8) as u32;
    run_processors(client, &installer, &profile, loader, mc_version, java_major, progress).await?;

    install::install(client, &id, &mc_root(), progress).await?;
    tokio::fs::write(dir.join(DONE_MARKER), b"").await.map_err(super::err)?;
    Ok(id)
}

/// The installer's version.json and install_profile.json.
fn read_installer(installer: &Path) -> Result<(Value, Value), String> {
    let file = std::fs::File::open(installer).map_err(super::err)?;
    let mut zip = zip::ZipArchive::new(file).map_err(super::err)?;
    let mut read = |name: &str| -> Result<Value, String> {
        let mut text = String::new();
        zip.by_name(name)
            .map_err(|_| format!("Installer has no {name}"))?
            .read_to_string(&mut text)
            .map_err(super::err)?;
        serde_json::from_str(&text).map_err(|e| format!("Bad {name}: {e}"))
    };
    let version = read("version.json")?;
    let profile = read("install_profile.json")?;
    Ok((version, profile))
}

/// Maven coordinates to a path under the libraries folder:
/// `group:artifact:version[:classifier][@ext]`.
pub fn coords_path(coords: &str) -> Option<String> {
    let (body, ext) = match coords.split_once('@') {
        Some((b, e)) => (b, e),
        None => (coords, "jar"),
    };
    let mut parts = body.split(':');
    let group = parts.next()?.replace('.', "/");
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next().map(|c| format!("-{c}")).unwrap_or_default();
    Some(format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}"))
}

/// Pulls a file out of the installer jar, e.g. `/data/client.lzma`, into the work folder.
fn extract_from_installer(installer: &Path, entry: &str, work: &Path) -> Result<PathBuf, String> {
    let name = entry.trim_start_matches('/');
    let dest = work.join(name);
    if dest.is_file() {
        return Ok(dest);
    }
    let file = std::fs::File::open(installer).map_err(super::err)?;
    let mut zip = zip::ZipArchive::new(file).map_err(super::err)?;
    let mut entry = zip.by_name(name).map_err(|_| format!("Installer has no {name}"))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(super::err)?;
    }
    let mut out = std::fs::File::create(&dest).map_err(super::err)?;
    std::io::copy(&mut entry, &mut out).map_err(super::err)?;
    Ok(dest)
}

/// Downloads a library into the libraries folder: from the installer's own `maven/` folder when
/// it has one, else from the artifact URL, else from the loader's Maven.
async fn ensure_library(
    client: &reqwest::Client,
    installer: &Path,
    path: &str,
    url: Option<&str>,
    loader: &str,
) -> Result<(), String> {
    let dest = libraries_dir().join(path);
    if dest.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(super::err)?;
    }
    let bundled = format!("maven/{path}");
    let from_installer = std::fs::File::open(installer)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .and_then(|mut z| {
            let mut entry = z.by_name(&bundled).ok()?;
            let mut out = std::fs::File::create(&dest).ok()?;
            std::io::copy(&mut entry, &mut out).ok()
        });
    if from_installer.is_some() {
        return Ok(());
    }
    let url = match url.filter(|u| !u.is_empty()) {
        Some(u) => u.to_string(),
        None => format!("{}{path}", maven_base(loader)),
    };
    install::download(client, &url, &dest, None).await
}

/// Runs the install profile's client processors, in order.
async fn run_processors(
    client: &reqwest::Client,
    installer: &Path,
    profile: &Value,
    loader: &str,
    mc_version: &str,
    java_major: u32,
    progress: &install::Progress,
) -> Result<(), String> {
    let work = installer.with_extension("work");
    std::fs::create_dir_all(&work).map_err(super::err)?;
    let libraries = libraries_dir();

    // Libraries the profile lists, then the jars and classpaths its processors use.
    let mut lib_jobs: Vec<(String, Option<String>)> = Vec::new();
    for lib in profile.get("libraries").and_then(Value::as_array).into_iter().flatten() {
        let name = lib.get("name").and_then(Value::as_str).unwrap_or_default();
        let artifact = lib.pointer("/downloads/artifact");
        let path = artifact
            .and_then(|a| a.get("path").and_then(Value::as_str).map(String::from))
            .or_else(|| coords_path(name));
        let url = artifact.and_then(|a| a.get("url").and_then(Value::as_str)).map(String::from);
        if let Some(path) = path {
            lib_jobs.push((path, url));
        }
    }
    let processors: Vec<&Value> = profile
        .get("processors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| {
            p.get("sides")
                .and_then(Value::as_array)
                .is_none_or(|s| s.iter().any(|x| x.as_str() == Some("client")))
        })
        .collect();
    for p in &processors {
        let coords = p.get("jar").and_then(Value::as_str).unwrap_or_default();
        lib_jobs.extend(coords_path(coords).map(|path| (path, None)));
        for c in p.get("classpath").and_then(Value::as_array).into_iter().flatten() {
            lib_jobs.extend(c.as_str().and_then(coords_path).map(|path| (path, None)));
        }
    }
    lib_jobs.sort();
    lib_jobs.dedup();
    progress("Downloading processor libraries", 0, lib_jobs.len());
    for (path, url) in &lib_jobs {
        // A processor output can be listed here too; a missing one is fine until it is made.
        let _ = ensure_library(client, installer, path, url.as_deref(), loader).await;
    }

    // Placeholder values for the client side.
    let minecraft_jar = versions_dir().join(mc_version).join(format!("{mc_version}.jar"));
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("ROOT".into(), mc_root().to_string_lossy().into());
    vars.insert("LIBRARY_DIR".into(), libraries.to_string_lossy().into());
    vars.insert("MINECRAFT_VERSION".into(), mc_version.into());
    vars.insert("MINECRAFT_JAR".into(), minecraft_jar.to_string_lossy().into());
    vars.insert("INSTALLER".into(), installer.to_string_lossy().into());
    vars.insert("SIDE".into(), "client".into());
    for (key, value) in profile.get("data").and_then(Value::as_object).into_iter().flatten() {
        let raw = value.get("client").and_then(Value::as_str).unwrap_or_default();
        let resolved = if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            libraries.join(coords_path(inner).ok_or_else(|| format!("Bad coordinates {inner}"))?).to_string_lossy().into()
        } else if raw.starts_with('/') {
            extract_from_installer(installer, raw, &work)?.to_string_lossy().into()
        } else {
            raw.trim_matches('\'').to_string()
        };
        vars.insert(key.clone(), resolved);
    }

    let java_bin = java::ensure_java(client, java_major, None, &noop_progress).await?;
    let total = processors.len();
    for (index, p) in processors.iter().enumerate() {
        progress("Processing", index, total);
        let jar_coords = p.get("jar").and_then(Value::as_str).unwrap_or_default();
        let jar = libraries.join(coords_path(jar_coords).ok_or("Processor has no jar")?);

        let mut classpath = vec![jar.clone()];
        for c in p.get("classpath").and_then(Value::as_array).into_iter().flatten() {
            if let Some(path) = c.as_str().and_then(coords_path) {
                classpath.push(libraries.join(path));
            }
        }
        let args: Vec<String> = p
            .get("args")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|a| resolve_arg(a, &vars, &libraries))
            .collect::<Result<_, _>>()?;

        // Skip a processor whose outputs already exist.
        let outputs_ready = p
            .get("outputs")
            .and_then(Value::as_object)
            .is_some_and(|o| {
                o.keys().all(|k| {
                    let path = resolve_arg(k, &vars, &libraries).unwrap_or_default();
                    Path::new(&path).is_file()
                })
            });
        if outputs_ready {
            continue;
        }

        let main = main_class(&jar)?;
        let cp = classpath
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(classpath_separator());
        let java_bin = java_bin.clone();
        let output = tokio::task::spawn_blocking(move || {
            Command::new(java_bin)
                .arg("-cp")
                .arg(cp)
                .arg(main)
                .args(args)
                .current_dir(mc_root())
                .output()
        })
        .await
        .map_err(super::err)?
        .map_err(|e| format!("Could not run a Forge processor: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let lines: Vec<&str> = stderr.lines().collect();
            let tail = lines[lines.len().saturating_sub(6)..].join("\n");
            return Err(format!("Forge processor {jar_coords} failed:\n{tail}"));
        }
    }
    progress("Processing", total, total);
    Ok(())
}

fn noop_progress(_stage: &str, _done: usize, _total: usize) {}

fn classpath_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// One processor argument: `[coords]` is a library path, `{NAME}` a placeholder, else literal text.
fn resolve_arg(arg: &str, vars: &HashMap<String, String>, libraries: &Path) -> Result<String, String> {
    if let Some(inner) = arg.strip_prefix('[').and_then(|a| a.strip_suffix(']')) {
        let path = coords_path(inner).ok_or_else(|| format!("Bad coordinates {inner}"))?;
        return Ok(libraries.join(path).to_string_lossy().into());
    }
    let mut out = String::new();
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else { break };
        out.push_str(&rest[..start]);
        let key = &rest[start + 1..start + end];
        match vars.get(key) {
            Some(v) => out.push_str(v),
            None => out.push_str(&rest[start..start + end + 1]),
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The `Main-Class` of a jar, from its manifest.
fn main_class(jar: &Path) -> Result<String, String> {
    let file = std::fs::File::open(jar).map_err(|e| format!("Missing processor {}: {e}", jar.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(super::err)?;
    let mut manifest = String::new();
    zip.by_name("META-INF/MANIFEST.MF")
        .map_err(|_| format!("{} has no manifest", jar.display()))?
        .read_to_string(&mut manifest)
        .map_err(super::err)?;
    manifest
        .lines()
        .find_map(|l| l.strip_prefix("Main-Class:"))
        .map(|m| m.trim().to_string())
        .ok_or_else(|| format!("{} has no Main-Class", jar.display()))
}

#[cfg(test)]
mod tests {
    use super::{coords_path, maven_versions, resolve_arg};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn maven_coordinates_become_paths() {
        assert_eq!(
            coords_path("net.minecraft:client:1.21.1:mappings@tsrg").as_deref(),
            Some("net/minecraft/client/1.21.1/client-1.21.1-mappings.tsrg")
        );
        assert_eq!(
            coords_path("net.minecraftforge:forge:1.21.1-52.0.28:client").as_deref(),
            Some("net/minecraftforge/forge/1.21.1-52.0.28/forge-1.21.1-52.0.28-client.jar")
        );
        assert_eq!(
            coords_path("org.ow2.asm:asm:9.6").as_deref(),
            Some("org/ow2/asm/asm/9.6/asm-9.6.jar")
        );
    }

    #[test]
    fn placeholders_and_library_arguments_resolve() {
        let mut vars = HashMap::new();
        vars.insert("ROOT".to_string(), "/games/mc".to_string());
        let libs = Path::new("/games/mc/libraries");
        assert_eq!(resolve_arg("--output", &vars, libs).unwrap(), "--output");
        assert_eq!(resolve_arg("{ROOT}/out", &vars, libs).unwrap(), "/games/mc/out");
        // A library path is built from the libraries folder, so compare paths (Windows uses `\`).
        assert_eq!(
            Path::new(&resolve_arg("[org.ow2.asm:asm:9.6]", &vars, libs).unwrap()),
            libs.join("org/ow2/asm/asm/9.6/asm-9.6.jar")
        );
    }

    #[test]
    fn maven_metadata_lists_every_version_in_order() {
        let xml = "<metadata><versioning><versions><version>1.21.1-52.0.1</version><version>1.21.1-52.0.2</version></versions></versioning></metadata>";
        assert_eq!(maven_versions(xml), vec!["1.21.1-52.0.1", "1.21.1-52.0.2"]);
    }
}

#[cfg(test)]
mod install_test {
    /// Ignored by default: downloads Minecraft and runs the Forge processors. Run with
    /// `HOME=<scratch dir> cargo test forge_1_21_1 -- --ignored --nocapture`, so the game
    /// files land in the scratch home and not in a real `.minecraft`.
    #[tokio::test]
    #[ignore]
    async fn installs_forge_1_21_1() {
        let client = crate::mc::http();
        let progress = |stage: &str, done: usize, total: usize| println!("{stage} {done}/{total}");
        let id = super::ensure_installed(&client, "forge", "1.21.1", Some("52.0.28"), &progress)
            .await
            .expect("forge install");
        println!("installed {id}");
        assert!(crate::mc::versions_dir().join(&id).join(super::DONE_MARKER).is_file());
    }
}
