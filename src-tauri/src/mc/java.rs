//! Finds a Java runtime new enough for a version: the chosen or system Java if it is new
//! enough, otherwise a Temurin JDK downloaded into the launcher's runtimes folder.

use super::install::Progress;
use super::{err, runtimes_dir};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The major version from `java -version` output, such as `openjdk version "25.0.5"` or
/// `java version "1.8.0_392"` (which is Java 8).
pub fn parse_java_major(output: &str) -> Option<u32> {
    let start = output.find('"')? + 1;
    let version = output[start..].split('"').next()?;
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn major_of(java: &Path) -> Option<u32> {
    let out = Command::new(java).arg("-version").output().ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    parse_java_major(&text)
}

/// `bin/java` inside an extracted JDK, which may sit one folder down (Temurin archives do).
fn find_java_in(dir: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let direct = dir.join("bin").join(exe);
    if direct.exists() {
        return Some(direct);
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        for candidate in [
            entry.join("bin").join(exe),
            entry.join("Contents").join("Home").join("bin").join(exe),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn adoptium_target() -> (&'static str, &'static str) {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    (os, arch)
}

fn unzip_into(archive: &Path, dest: &Path) -> Result<(), String> {
    let name = archive.to_string_lossy();
    std::fs::create_dir_all(dest).map_err(err)?;
    if name.ends_with(".zip") {
        let file = std::fs::File::open(archive).map_err(err)?;
        let mut zip = zip::ZipArchive::new(file).map_err(err)?;
        zip.extract(dest).map_err(err)
    } else {
        let file = std::fs::File::open(archive).map_err(err)?;
        let decoder = flate2::read::GzDecoder::new(file);
        tar::Archive::new(decoder).unpack(dest).map_err(err)
    }
}

/// A Java runtime with at least `required` as its major version.
pub async fn ensure_java(
    client: &reqwest::Client,
    required: u32,
    chosen: Option<&str>,
    progress: &Progress,
) -> Result<PathBuf, String> {
    if let Some(path) = chosen.filter(|p| !p.trim().is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let system = PathBuf::from(if cfg!(windows) { "java.exe" } else { "java" });
    if major_of(&system).is_some_and(|m| m >= required) {
        return Ok(system);
    }

    let dir = runtimes_dir().join(format!("temurin-{required}"));
    if let Some(found) = find_java_in(&dir) {
        return Ok(found);
    }

    let death_client_runtimes = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
        .join("death-client")
        .join("runtimes");
    if let Ok(entries) = std::fs::read_dir(&death_client_runtimes) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir()
                && let Some(found) = find_java_in(&p)
                    && major_of(&found).is_some_and(|m| m >= required) {
                        return Ok(found);
                    }
        }
    }

    let (os, arch) = adoptium_target();
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{required}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );
    progress(&format!("Downloading Java {required}"), 0, 1);
    let bytes = client
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not download Java {required}: {e}"))?
        .bytes()
        .await
        .map_err(err)?;
    tokio::fs::create_dir_all(&dir).await.map_err(err)?;
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    let archive = dir.join(format!("jdk.{extension}"));
    tokio::fs::write(&archive, &bytes).await.map_err(err)?;
    progress(&format!("Unpacking Java {required}"), 0, 1);
    let (archive_for_task, dir_for_task) = (archive.clone(), dir.clone());
    tokio::task::spawn_blocking(move || unzip_into(&archive_for_task, &dir_for_task))
        .await
        .map_err(err)??;
    let _ = tokio::fs::remove_file(&archive).await;

    let java = find_java_in(&dir).ok_or_else(|| format!("Java {required} did not unpack correctly"))?;
    progress(&format!("Java {required} ready"), 1, 1);
    Ok(java)
}

#[cfg(test)]
mod tests {
    use super::parse_java_major;

    #[test]
    fn reads_the_major_version_from_java_output() {
        assert_eq!(parse_java_major("openjdk version \"25.0.5\" 2026-07-21"), Some(25));
        assert_eq!(parse_java_major("java version \"1.8.0_392\""), Some(8));
        assert_eq!(parse_java_major("openjdk version \"21.0.12\" 2026-07-21 LTS"), Some(21));
        assert_eq!(parse_java_major("no version here"), None);
    }
}
