//! Looks up a player's skin and cape by username, from Mojang's public profile endpoints.
//! Results are kept in memory for the session, so each name costs one lookup at most.

use crate::mc::http;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerTextures {
    pub name: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    /// Slim (Alex-style) arms rather than the classic wide ones.
    pub slim: bool,
}

#[derive(Deserialize)]
struct MojangProfile {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct SessionProfile {
    id: String,
    name: String,
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct TexturesPayload {
    #[serde(default)]
    textures: TexturesMap,
}

#[derive(Deserialize, Default)]
struct TexturesMap {
    #[serde(rename = "SKIN")]
    skin: Option<TextureEntry>,
    #[serde(rename = "CAPE")]
    cape: Option<TextureEntry>,
}

#[derive(Deserialize)]
struct TextureEntry {
    url: String,
    metadata: Option<SkinMetadata>,
}

#[derive(Deserialize)]
struct SkinMetadata {
    model: Option<String>,
}

static CACHE: OnceLock<Mutex<HashMap<String, PlayerTextures>>> = OnceLock::new();

/// Minecraft usernames: 3 to 16 letters, digits or underscores.
pub fn valid_username(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Texture links point at http://textures.minecraft.net; the https form serves the same files.
fn secure(url: &str) -> String {
    url.replacen("http://", "https://", 1)
}

/// A 64x64 or 64x32 skin from minotar.net as a PNG data URL, or None if it isn't one.
async fn minotar_skin(client: &reqwest::Client, name: &str) -> Option<String> {
    let bytes = client
        .get(format!("https://minotar.net/skin/{name}"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .bytes()
        .await
        .ok()?;
    // PNG header: width and height are big-endian u32s right after the IHDR tag.
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    if w != 64 || (h != 64 && h != 32) {
        return None;
    }
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// Downloads a texture from Mojang's texture host (only that host is allowed) as a PNG data URL.
async fn fetch_data_url(client: &reqwest::Client, url: &str) -> Result<String, String> {
    if !url.starts_with("https://textures.minecraft.net/") {
        return Err("Unexpected texture host".to_string());
    }
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

pub async fn lookup(username: &str) -> Result<PlayerTextures, String> {
    let name = username.trim();
    if !valid_username(name) {
        return Err("Usernames are 3 to 16 letters, numbers or underscores".to_string());
    }
    let key = name.to_lowercase();
    if let Some(hit) = CACHE.get_or_init(Default::default).lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(hit.clone());
    }

    let client = http();
    let profile_url = format!("https://api.mojang.com/users/profiles/minecraft/{name}");
    let res = client.get(&profile_url).send().await.map_err(|e| e.to_string())?;
    // Mojang answers 204 (or 404) for a name nobody has.
    if res.status() == reqwest::StatusCode::NO_CONTENT || res.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(format!("No player named {name}"));
    }
    let mojang: MojangProfile = res
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let session: SessionProfile = client
        .get(format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", mojang.id))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let textures = session
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .map(|p| {
            base64::engine::general_purpose::STANDARD
                .decode(&p.value)
                .map_err(|e| e.to_string())
                .and_then(|bytes| serde_json::from_slice::<TexturesPayload>(&bytes).map_err(|e| e.to_string()))
        })
        .transpose()?
        .unwrap_or(TexturesPayload { textures: TexturesMap::default() });

    // The images are fetched here and handed to the page as data URLs, so the page never has to
    // load them cross-origin from Mojang's texture server.
    // Skin comes from minotar, like the older launcher; Mojang's texture is the fallback.
    let skin_data = match minotar_skin(&client, &mojang.name).await {
        Some(data) => Some(data),
        None => match textures.textures.skin.as_ref() {
            Some(s) => Some(fetch_data_url(&client, &secure(&s.url)).await?),
            None => None,
        },
    };
    let cape_data = match textures.textures.cape.as_ref() {
        Some(c) => fetch_data_url(&client, &secure(&c.url)).await.ok(),
        None => None,
    };

    let result = PlayerTextures {
        name: if session.name.is_empty() { mojang.name } else { session.name },
        uuid: session.id,
        skin_url: skin_data,
        cape_url: cape_data,
        slim: textures
            .textures
            .skin
            .as_ref()
            .and_then(|s| s.metadata.as_ref())
            .and_then(|m| m.model.as_deref())
            == Some("slim"),
    };
    CACHE
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?
        .insert(key, result.clone());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{secure, valid_username};

    #[test]
    fn usernames_follow_minecraft_rules() {
        assert!(valid_username("Dream"));
        assert!(valid_username("a_b_1"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("this_name_is_too_long"));
        assert!(!valid_username("bad name"));
        assert!(!valid_username("../etc"));
    }

    #[test]
    fn texture_links_are_served_over_https() {
        assert_eq!(
            secure("http://textures.minecraft.net/texture/abc"),
            "https://textures.minecraft.net/texture/abc"
        );
    }
}
