//! Modrinth: search, and installs of mods (with their required dependencies), resource packs,
//! shader packs and modpacks (.mrpack).

use crate::instance_manager::{create_instance, load_instances, update_instance};
use crate::mc::{http, install};
use crate::models::{CreateInstanceRequest, EditionType, Instance, InstalledMod, ModrinthHit, ModrinthSearchResult};
use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const APP_USER_AGENT: &str = "fugo-launcher/3.2.0 (achinthyaj)";
/// Sidecar in the mods folder: project id to the file that carries it, so a dependency is
/// not downloaded twice.
const PROJECTS_FILE: &str = ".horizon-projects.json";

#[derive(Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<RawModrinthHit>,
    total_hits: u64,
}

#[derive(Deserialize)]
struct RawModrinthHit {
    project_id: String,
    title: String,
    description: String,
    categories: Option<Vec<String>>,
    client_side: Option<String>,
    server_side: Option<String>,
    downloads: Option<u64>,
    icon_url: Option<String>,
    author: String,
    slug: String,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    project_type: String,
    #[serde(default)]
    title: String,
}

#[derive(Clone, Deserialize)]
struct Version {
    id: String,
    project_id: String,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
    #[serde(default)]
    files: Vec<VersionFile>,
}

#[derive(Clone, Deserialize)]
struct Dependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Clone, Deserialize)]
struct VersionFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    hashes: HashMap<String, String>,
}

impl Version {
    fn main_file(&self) -> Option<&VersionFile> {
        self.files.iter().find(|f| f.primary).or_else(|| self.files.first())
    }
}

/// The instance a mod or pack is installed into: which loader and Minecraft version it must match.
struct Target {
    loader: Option<String>,
    game_version: Option<String>,
}

impl Target {
    fn for_instance(inst: &Instance) -> Self {
        let java = matches!(inst.edition, EditionType::Java);
        let modded = java && !matches!(inst.loader.as_str(), "" | "vanilla");
        Target {
            loader: modded.then(|| inst.loader.clone()),
            game_version: java.then(|| inst.game_version.clone()),
        }
    }
}

fn project_name(project: &Project) -> String {
    if project.title.is_empty() { project.id.clone() } else { project.title.clone() }
}

/// Refuses a mod version built for another loader than the instance's (a Forge mod in a Fabric instance).
fn check_loader(version: &Version, target: &Target) -> Result<(), String> {
    let Some(loader) = &target.loader else {
        return Err("This instance has no mod loader".to_string());
    };
    if !version.loaders.is_empty() && !version.loaders.iter().any(|l| l == loader) {
        return Err(format!(
            "This mod is for {} and cannot go into a {} instance",
            version.loaders.join(", "),
            loader
        ));
    }
    Ok(())
}

fn folder_for(project_type: &str) -> &'static str {
    match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

async fn get_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T, String> {
    let res = client
        .get(url)
        .header(USER_AGENT, APP_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Modrinth network error: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Modrinth API error: {}", res.status()));
    }
    res.json().await.map_err(|e| format!("Failed to parse Modrinth JSON: {e}"))
}

async fn get_project(client: &reqwest::Client, project_id: &str) -> Result<Project, String> {
    get_json(client, &format!("{MODRINTH_API}/project/{project_id}")).await
}

async fn get_version(client: &reqwest::Client, version_id: &str) -> Result<Version, String> {
    get_json(client, &format!("{MODRINTH_API}/version/{version_id}")).await
}

/// The newest version of a project that fits the target. Mods are filtered by loader too.
async fn pick_version(client: &reqwest::Client, project: &Project, target: &Target) -> Result<Version, String> {
    let mut url = format!("{MODRINTH_API}/project/{}/version", project.id);
    let mut params = Vec::new();
    if project.project_type == "mod"
        && let Some(loader) = &target.loader {
            params.push(format!("loaders={}", urlencoding::encode(&serde_json::json!([loader]).to_string())));
        }
    if let Some(game_version) = &target.game_version {
        params.push(format!("game_versions={}", urlencoding::encode(&serde_json::json!([game_version]).to_string())));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }
    let versions: Vec<Version> = get_json(client, &url).await?;
    versions.into_iter().next().ok_or_else(|| {
        let name = if project.title.is_empty() { project.id.clone() } else { project.title.clone() };
        match &target.game_version {
            Some(v) => format!("No version of {name} fits Minecraft {v}"),
            None => format!("No version of {name} is available"),
        }
    })
}

async fn download_file(client: &reqwest::Client, file: &VersionFile, dest: &Path) -> Result<(), String> {
    install::download(client, &file.url, dest, file.hashes.get("sha1").map(String::as_str)).await
}

fn read_projects(dir: &Path) -> HashMap<String, String> {
    fs::read_to_string(dir.join(PROJECTS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_projects(dir: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(dir.join(PROJECTS_FILE), text).map_err(|e| e.to_string())
}

fn find_instance(instance_id: &str) -> Result<Instance, String> {
    load_instances()
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| format!("Instance {instance_id} not found"))
}

pub async fn search_modrinth(
    query: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
    limit: Option<u32>,
    project_type: Option<&str>,
    sort: Option<&str>,
) -> Result<ModrinthSearchResult, String> {
    let client = http();
    let project_type = project_type.filter(|t| !t.is_empty()).unwrap_or("mod");
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{project_type}")]];

    // Loader filter applies to mods only; packs and shaders match their own loaders.
    if let Some(l) = loader
        && project_type == "mod" && !matches!(l, "" | "all" | "vanilla" | "horizon-rust") {
            facets.push(vec![format!("categories:{}", l.to_lowercase())]);
        }
    if let Some(v) = game_version
        && !v.is_empty() && v != "any" {
            facets.push(vec![format!("versions:{}", v)]);
        }

    let facets_str = serde_json::to_string(&facets).unwrap_or_default();
    let limit_num = limit.unwrap_or(20).to_string();
    // Modrinth sorts by relevance, downloads, follows, newest or updated.
    let index = match sort.unwrap_or("follows") {
        s @ ("relevance" | "downloads" | "newest" | "updated") => s,
        _ => "follows",
    };
    let url = format!(
        "{MODRINTH_API}/search?query={}&limit={}&index={}&facets={}",
        urlencoding::encode(query),
        limit_num,
        index,
        urlencoding::encode(&facets_str)
    );

    let raw: ModrinthSearchResponse = get_json(&client, &url).await?;
    let hits = raw
        .hits
        .into_iter()
        .map(|h| ModrinthHit {
            project_id: h.project_id,
            title: h.title,
            description: h.description,
            categories: h.categories.unwrap_or_default(),
            client_side: h.client_side.unwrap_or_else(|| "unknown".into()),
            server_side: h.server_side.unwrap_or_else(|| "unknown".into()),
            downloads: h.downloads.unwrap_or(0),
            icon_url: h.icon_url,
            author: h.author,
            slug: h.slug,
        })
        .collect();

    Ok(ModrinthSearchResult {
        hits,
        total_hits: raw.total_hits,
    })
}

/// Installs a mod, resource pack or shader pack into an instance, with the required
/// dependencies of a mod. Dependencies are installed without being listed to the player.
pub async fn download_modrinth_mod(
    instance_id: &str,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<InstalledMod, String> {
    let inst = find_instance(instance_id)?;
    let client = http();
    let target = Target::for_instance(&inst);

    let project = get_project(&client, project_id).await?;
    let folder = folder_for(&project.project_type);
    // A mod only goes into an instance that runs a loader, and every mod in the install
    // (the chosen one and its dependencies) must support that loader.
    if folder == "mods" && target.loader.is_none() {
        return Err(format!(
            "{} is a mod, and {} has no mod loader. Set one under Edit first.",
            project_name(&project),
            inst.name
        ));
    }
    let dir = PathBuf::from(&inst.game_dir).join(folder);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let root = match version_id {
        Some(v) => get_version(&client, v).await?,
        None => pick_version(&client, &project, &target).await?,
    };
    let root_file = root
        .main_file()
        .cloned()
        .ok_or_else(|| "No files found for version".to_string())?;

    let mut projects = read_projects(&dir);
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue = vec![root.clone()];
    let mut main: Option<VersionFile> = None;

    while let Some(version) = queue.pop() {
        if !visited.insert(version.project_id.clone()) {
            continue;
        }
        if folder == "mods" {
            check_loader(&version, &target)?;
        }
        let file = version
            .main_file()
            .cloned()
            .ok_or_else(|| "No files found for version".to_string())?;
        download_file(&client, &file, &dir.join(&file.filename)).await?;
        projects.insert(version.project_id.clone(), file.filename.clone());
        write_projects(&dir, &projects)?;
        if version.id == root.id {
            main = Some(file);
        }

        for dep in version.dependencies.iter().filter(|d| d.dependency_type == "required") {
            if let Some(pid) = &dep.project_id {
                let on_disk = projects.get(pid).is_some_and(|f| dir.join(f).is_file());
                if visited.contains(pid) || on_disk {
                    continue;
                }
            }
            let dep_version = match (&dep.version_id, &dep.project_id) {
                (Some(vid), _) => get_version(&client, vid).await?,
                (None, Some(pid)) => {
                    let dep_project = get_project(&client, pid).await?;
                    pick_version(&client, &dep_project, &target).await?
                }
                (None, None) => continue,
            };
            queue.push(dep_version);
        }
    }

    let file = main.unwrap_or(root_file);
    Ok(InstalledMod {
        filename: file.filename.clone(),
        name: file
            .filename
            .trim_end_matches(".jar")
            .trim_end_matches(".zip")
            .trim_end_matches(".hmod")
            .to_string(),
        enabled: true,
        size_bytes: 0,
    })
}

/// Modrinth modpack index (`modrinth.index.json` in an .mrpack).
#[derive(Deserialize)]
struct MrIndex {
    name: String,
    dependencies: HashMap<String, String>,
    files: Vec<MrFile>,
}

#[derive(Deserialize)]
struct MrFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<MrEnv>,
    #[serde(default)]
    downloads: Vec<String>,
}

#[derive(Deserialize)]
struct MrEnv {
    client: Option<String>,
}

/// A path from a pack, kept inside the instance: no absolute paths and no `..`.
fn safe_relative(path: &str) -> Option<PathBuf> {
    let p = Path::new(path);
    if p.components().all(|c| matches!(c, Component::Normal(_))) && !path.is_empty() {
        Some(p.to_path_buf())
    } else {
        None
    }
}

/// Installs a Modrinth modpack as a new Java instance: its Minecraft version and loader,
/// every client file, and the overrides folders.
pub async fn install_modrinth_modpack(project_id: &str, version_id: Option<&str>) -> Result<Instance, String> {
    let client = http();
    let project = get_project(&client, project_id).await?;
    let version = match version_id {
        Some(v) => get_version(&client, v).await?,
        None => {
            let versions: Vec<Version> =
                get_json(&client, &format!("{MODRINTH_API}/project/{project_id}/version")).await?;
            versions.into_iter().next().ok_or("No versions found for modpack")?
        }
    };
    let pack_file = version.main_file().cloned().ok_or("Modpack version has no file")?;

    let archive_path = std::env::temp_dir().join(format!("horizon-modpack-{}.mrpack", uuid::Uuid::new_v4()));
    download_file(&client, &pack_file, &archive_path).await?;
    let result = build_from_archive(&client, &project, &archive_path).await;
    let _ = fs::remove_file(&archive_path);
    result
}

async fn build_from_archive(client: &reqwest::Client, project: &Project, archive_path: &Path) -> Result<Instance, String> {
    let index: MrIndex = {
        let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut text = String::new();
        zip.by_name("modrinth.index.json")
            .map_err(|_| "Not a Modrinth modpack: modrinth.index.json is missing".to_string())?
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Bad modrinth.index.json: {e}"))?
    };

    let game_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or("The modpack does not name a Minecraft version")?;
    let (loader, loader_version) = [
        ("fabric-loader", "fabric"),
        ("quilt-loader", "quilt"),
        ("forge", "forge"),
        ("neoforge", "neoforge"),
    ]
    .iter()
    .find_map(|(key, name)| index.dependencies.get(*key).map(|v| (name.to_string(), v.clone())))
    .unwrap_or_else(|| ("vanilla".to_string(), String::new()));

    let name = if index.name.trim().is_empty() { project.title.clone() } else { index.name.clone() };
    let mut inst = create_instance(CreateInstanceRequest {
        name,
        edition: EditionType::Java,
        game_version,
        loader: loader.clone(),
        banner: None,
    })?;
    if matches!(loader.as_str(), "fabric" | "quilt") && !loader_version.is_empty() {
        inst.loader_version = Some(loader_version);
        inst = update_instance(inst)?;
    }
    let game_dir = PathBuf::from(&inst.game_dir);

    // Client files, a few at a time.
    use futures_util::{stream, StreamExt};
    let jobs: Vec<(PathBuf, MrFile)> = index
        .files
        .into_iter()
        .filter(|f| f.env.as_ref().and_then(|e| e.client.as_deref()) != Some("unsupported"))
        .filter_map(|f| safe_relative(&f.path).map(|rel| (game_dir.join(rel), f)))
        .collect();
    let results: Vec<Result<(), String>> = stream::iter(jobs)
        .map(|(dest, file)| {
            let client = client.clone();
            async move {
                let url = file.downloads.first().ok_or_else(|| format!("No download for {}", file.path))?;
                install::download(&client, url, &dest, file.hashes.get("sha1").map(String::as_str)).await
            }
        })
        .buffer_unordered(6)
        .collect()
        .await;
    if let Some(Err(e)) = results.into_iter().find(|r| r.is_err()) {
        return Err(e);
    }

    // Overrides: `overrides/` for everyone, `client-overrides/` for the client, both into the game folder.
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let rel = match name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("client-overrides/"))
        {
            Some(r) if !r.is_empty() => r.to_string(),
            _ => continue,
        };
        let Some(rel) = safe_relative(rel.trim_end_matches('/')) else { continue };
        let dest = game_dir.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        }
    }

    Ok(inst)
}

#[cfg(test)]
mod tests {
    use super::safe_relative;
    use std::path::PathBuf;

    #[test]
    fn pack_paths_stay_inside_the_instance() {
        assert_eq!(safe_relative("mods/sodium.jar"), Some(PathBuf::from("mods/sodium.jar")));
        assert_eq!(safe_relative("../../etc/passwd"), None);
        assert_eq!(safe_relative("/abs/path.jar"), None);
        assert_eq!(safe_relative(""), None);
    }
}
