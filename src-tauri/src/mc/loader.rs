//! Fabric and Quilt. Their profile JSON extends a vanilla version (`inheritsFrom`) and adds
//! libraries and a main class. This merges the profile with the vanilla version and writes the
//! result as a version of its own, with the vanilla client jar beside it, so the installer and
//! the launcher treat it like any other version.

use super::{err, install, versions_dir, version_json};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Loaders whose profile this module can install.
pub fn is_profile_loader(loader: &str) -> bool {
    meta_base(loader).is_some()
}

fn meta_base(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("https://meta.fabricmc.net/v2"),
        "quilt" => Some("https://meta.quiltmc.org/v3"),
        _ => None,
    }
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value, String> {
    client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(err)?
        .json()
        .await
        .map_err(err)
}

/// Loader builds published for a Minecraft version, newest first, as the meta service lists them.
pub async fn list_versions(client: &reqwest::Client, loader: &str, mc_version: &str) -> Result<Vec<String>, String> {
    let base = meta_base(loader).ok_or_else(|| format!("{loader} is not supported yet"))?;
    let list = get_json(client, &format!("{base}/versions/loader/{mc_version}")).await?;
    Ok(list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e.pointer("/loader/version").and_then(Value::as_str).map(String::from))
        .collect())
}

/// Makes sure the loader's version for `mc_version` is installed on disk and returns its id.
/// `requested` picks a build; None takes the newest stable one. Needs the network the first
/// time; after that, an installed copy works offline.
pub async fn ensure_profile(
    client: &reqwest::Client,
    loader: &str,
    mc_version: &str,
    requested: Option<&str>,
    progress: &dyn Fn(&str, usize, usize),
) -> Result<String, String> {
    let base = meta_base(loader).ok_or_else(|| format!("{loader} is not supported yet"))?;
    let requested = requested.map(str::trim).filter(|v| !v.is_empty());

    let profile = match fetch_profile(client, base, loader, mc_version, requested).await {
        Ok(profile) => profile,
        Err(network_error) => {
            // Offline: fall back to a copy already installed for this Minecraft version.
            return installed_profile(loader, mc_version, requested).ok_or(network_error);
        }
    };
    let id = profile
        .get("id")
        .and_then(Value::as_str)
        .ok_or("Loader profile has no id")?
        .to_string();

    let dir = versions_dir().join(&id);
    let json_file = dir.join(format!("{id}.json"));
    let jar = dir.join(format!("{id}.jar"));
    if json_file.is_file() && jar.is_file() {
        return Ok(id);
    }

    progress("Loader", 0, 1);
    let parent = version_json(client, mc_version).await?;
    let merged = merge(&parent, &profile);

    // The vanilla client jar is the loader's client jar too.
    let client_info = parent
        .pointer("/downloads/client")
        .ok_or_else(|| format!("Version {mc_version} has no client download"))?;
    install::download(
        client,
        client_info.get("url").and_then(Value::as_str).unwrap_or_default(),
        &jar,
        client_info.get("sha1").and_then(Value::as_str),
    )
    .await?;

    tokio::fs::create_dir_all(&dir).await.map_err(err)?;
    let text = serde_json::to_string_pretty(&merged).map_err(err)?;
    tokio::fs::write(&json_file, text).await.map_err(err)?;
    progress("Loader", 1, 1);
    Ok(id)
}

async fn fetch_profile(
    client: &reqwest::Client,
    base: &str,
    loader: &str,
    mc_version: &str,
    requested: Option<&str>,
) -> Result<Value, String> {
    let loader_version = match requested {
        Some(v) => v.to_string(),
        None => {
            let list = get_json(client, &format!("{base}/versions/loader/{mc_version}")).await?;
            // The meta list is newest first. Take the newest stable entry: Fabric flags it, and
            // Quilt marks betas with a '-' in the version.
            list.as_array()
                .and_then(|entries| {
                    entries.iter().find(|e| {
                        let stable = e.pointer("/loader/stable").and_then(Value::as_bool);
                        let version = e.pointer("/loader/version").and_then(Value::as_str).unwrap_or_default();
                        stable.unwrap_or(!version.contains('-'))
                    })
                })
                .and_then(|e| e.pointer("/loader/version").and_then(Value::as_str))
                .ok_or_else(|| format!("No {loader} loader is published for Minecraft {mc_version}"))?
                .to_string()
        }
    };

    get_json(
        client,
        &format!("{base}/versions/loader/{mc_version}/{loader_version}/profile/json"),
    )
    .await
}

/// An installed copy of this loader for `mc_version`: the requested build, or else the newest one.
fn installed_profile(loader: &str, mc_version: &str, requested: Option<&str>) -> Option<String> {
    let installed = |name: &str| versions_dir().join(name).join(format!("{name}.json")).is_file();
    if let Some(v) = requested {
        let name = format!("{loader}-loader-{v}-{mc_version}");
        return installed(&name).then_some(name);
    }
    let prefix = format!("{loader}-loader-");
    let suffix = format!("-{mc_version}");
    let mut found: Vec<String> = std::fs::read_dir(versions_dir())
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| name.starts_with(&prefix) && name.ends_with(&suffix) && installed(name))
        .collect();
    found.sort();
    found.pop()
}

/// Merges a loader profile into its vanilla parent. The profile's id and main class win; its
/// libraries come first (a library in both keeps the profile's version); its game and JVM
/// arguments are added after the parent's.
pub fn merge(parent: &Value, profile: &Value) -> Value {
    let mut out = parent.clone();
    {
        let obj = out.as_object_mut().expect("version JSON is an object");
        obj.insert("id".into(), profile.get("id").cloned().unwrap_or(Value::Null));
        if let Some(main) = profile.get("mainClass") {
            obj.insert("mainClass".into(), main.clone());
        }
        obj.remove("inheritsFrom");
    }

    let mut seen = HashSet::new();
    let mut libraries = Vec::new();
    for lib in profile
        .get("libraries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(parent.get("libraries").and_then(Value::as_array).into_iter().flatten())
    {
        let name = lib.get("name").and_then(Value::as_str).unwrap_or_default();
        if seen.insert(library_key(name)) {
            libraries.push(lib.clone());
        }
    }
    out["libraries"] = Value::Array(libraries);

    for kind in ["game", "jvm"] {
        let mut args: Vec<Value> = Vec::new();
        for source in [parent, profile] {
            if let Some(list) = source.pointer(&format!("/arguments/{kind}")).and_then(Value::as_array) {
                args.extend(list.iter().cloned());
            }
        }
        if !args.is_empty() {
            if out.get("arguments").is_none() {
                out["arguments"] = json!({});
            }
            out["arguments"][kind] = Value::Array(args);
        }
    }
    out
}

/// `group:artifact[:classifier]`, without the version, so two versions of one library compare equal.
fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.as_slice() {
        [group, artifact, _version, classifier, ..] => format!("{group}:{artifact}:{classifier}"),
        [group, artifact, ..] => format!("{group}:{artifact}"),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{library_key, merge};
    use serde_json::json;

    #[test]
    fn profile_libraries_win_over_vanilla_versions() {
        let parent = json!({
            "id": "1.21.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [{"name": "org.ow2.asm:asm:9.6"}, {"name": "com.google.guava:guava:32.0.0"}]
        });
        let profile = json!({
            "id": "fabric-loader-0.16.9-1.21.1",
            "inheritsFrom": "1.21.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [{"name": "org.ow2.asm:asm:9.7"}, {"name": "net.fabricmc:fabric-loader:0.16.9"}]
        });
        let merged = merge(&parent, &profile);
        assert_eq!(merged["id"], "fabric-loader-0.16.9-1.21.1");
        assert_eq!(merged["mainClass"], "net.fabricmc.loader.impl.launch.knot.KnotClient");
        assert!(merged.get("inheritsFrom").is_none());
        let names: Vec<&str> = merged["libraries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["org.ow2.asm:asm:9.7", "net.fabricmc:fabric-loader:0.16.9", "com.google.guava:guava:32.0.0"]
        );
    }

    #[test]
    fn library_key_drops_the_version() {
        assert_eq!(library_key("org.lwjgl:lwjgl:3.3.3:natives-linux"), "org.lwjgl:lwjgl:natives-linux");
        assert_eq!(library_key("org.ow2.asm:asm:9.6"), "org.ow2.asm:asm");
    }
}
