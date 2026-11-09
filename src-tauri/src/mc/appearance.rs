//! Puts the player's skin and cape where the in-game death-client mod reads them: the instance's
//! `config/death-client` folder, with a `death-client.json` that says which files to use.

use base64::Engine;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// The skin service (Yggdrasil-compatible): skins are uploaded here and the game reads them via authlib-injector.
pub const SKIN_SERVER: &str = "https://aether.aetherservers.workers.dev/";
const AUTHLIB_URL: &str =
    "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";

/// Downloads authlib-injector once and returns its path.
pub async fn authlib_injector(client: &reqwest::Client) -> Result<PathBuf, String> {
    let dest = crate::settings::get_app_dir().join("authlib-injector-1.2.5.jar");
    super::install::download(client, AUTHLIB_URL, &dest, None).await?;
    Ok(dest)
}

/// A PNG from a data URL, or fetched from a web URL.
async fn load_png(client: &reqwest::Client, source: &str) -> Result<Vec<u8>, String> {
    if source.starts_with("data:") {
        return decode_png(source);
    }
    if source.starts_with("https://") || source.starts_with("http://") {
        let bytes = client
            .get(source)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        return Ok(bytes.to_vec());
    }
    Err("Skins and capes must be PNG images".to_string())
}

/// Sends a PNG to the skin service as a multipart upload.
async fn put_png(client: &reqwest::Client, url: String, field: &str, png: Vec<u8>, extra: &[(&str, &str)]) -> Result<(), String> {
    let boundary = "fugoSkinBoundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{field}.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&png);
    for (name, value) in extra {
        body.extend_from_slice(
            format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}").as_bytes(),
        );
    }
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    client
        .put(url)
        .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
        .body(body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Uploads a skin for a username. `model` is "classic" or "slim" (Alex-style arms).
pub async fn upload_skin(client: &reqwest::Client, username: &str, source: &str, model: &str) -> Result<(), String> {
    let png = load_png(client, source).await?;
    let url = format!("{SKIN_SERVER}api/skins/{}", urlencoding::encode(username));
    put_png(client, url, "skin", png, &[("model", model)]).await
}

/// Uploads a cape for a username.
pub async fn upload_cape(client: &reqwest::Client, username: &str, source: &str) -> Result<(), String> {
    let png = load_png(client, source).await?;
    let url = format!("{SKIN_SERVER}api/capes/{}", urlencoding::encode(username));
    put_png(client, url, "cape", png, &[]).await
}

const PNG_PREFIX: &str = "data:image/png;base64,";

fn decode_png(data: &str) -> Result<Vec<u8>, String> {
    let body = data
        .strip_prefix(PNG_PREFIX)
        .ok_or("Skins and capes must be PNG images")?;
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|e| e.to_string())
}

/// Writes the skin and cape (or removes them when absent) and the config the mod reads.
pub fn deploy(game_dir: &Path, skin: Option<&str>, cape: Option<&str>) -> Result<(), String> {
    let config = game_dir.join("config").join("death-client");
    let skins = config.join("skins");
    let capes = config.join("capes");
    fs::create_dir_all(&skins).map_err(|e| e.to_string())?;
    fs::create_dir_all(&capes).map_err(|e| e.to_string())?;

    let skin_enabled = place(skin, &skins.join("skin.png"))?;
    let cape_enabled = place(cape, &capes.join("cape.png"))?;

    let text = serde_json::to_string_pretty(&json!({
        "skinFile": "skin.png",
        "capeFile": "cape.png",
        "skinEnabled": skin_enabled,
        "capeEnabled": cape_enabled,
    }))
    .map_err(|e| e.to_string())?;
    fs::write(config.join("death-client.json"), text).map_err(|e| e.to_string())
}

/// Writes `data` to `path` when there is one; otherwise removes any old copy. Returns whether it's in use.
fn place(data: Option<&str>, path: &Path) -> Result<bool, String> {
    match data {
        Some(d) => {
            fs::write(path, decode_png(d)?).map_err(|e| e.to_string())?;
            Ok(true)
        }
        None => {
            let _ = fs::remove_file(path);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::deploy;
    use base64::Engine;
    use std::fs;

    #[test]
    fn skin_and_cape_are_written_with_their_config() {
        let dir = std::env::temp_dir().join(format!("fugo-appearance-{}", uuid::Uuid::new_v4()));
        let png = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b"\x89PNG-test"));
        deploy(&dir, Some(&png), None).unwrap();
        let config = dir.join("config/death-client");
        assert_eq!(fs::read(config.join("skins/skin.png")).unwrap(), b"\x89PNG-test");
        assert!(!config.join("capes/cape.png").exists());
        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(config.join("death-client.json")).unwrap()).unwrap();
        assert_eq!(json["skinEnabled"], true);
        assert_eq!(json["capeEnabled"], false);
        let _ = fs::remove_dir_all(&dir);
    }
}
