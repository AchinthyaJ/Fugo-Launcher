//! Puts the player's skin and cape where the in-game death-client mod reads them: the instance's
//! `config/death-client` folder, with a `death-client.json` that says which files to use.

use base64::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;

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
