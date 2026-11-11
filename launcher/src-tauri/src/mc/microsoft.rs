//! Microsoft accounts, the way Minecraft: Java Edition needs them: a device-code sign-in with
//! Microsoft, then Xbox Live, XSTS and Minecraft's services, the same chain Aether Launcher follows.
//! The refresh token keeps the account signed in; a launch renews the Minecraft token when it is
//! close to its end.

use crate::accounts;
use crate::models::Account;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The public client ID Minecraft's own launchers sign in with (also Aether Launcher's default).
const CLIENT_ID: &str = "00000000402b5328";
const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str = "https://login.live.com/oauth20_connect.srf";
const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const XBOX_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
/// Renew the Minecraft token once less than this is left, so a launch never starts with a dead one.
const RENEW_WITHIN_SECS: u64 = 5 * 60;

/// What the player needs to finish signing in: the code to type and the page to type it on.
/// It goes to the window and comes back to finish the sign-in.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginPrompt {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// The cancel flag of the sign-in that is waiting for the player, if one is.
static CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Asks Microsoft for a code the player types on its sign-in page.
pub async fn begin(client: &reqwest::Client) -> Result<LoginPrompt, String> {
    let (status, body) = form_post(
        client,
        DEVICE_CODE_URL,
        form(&[("client_id", CLIENT_ID), ("scope", SCOPE), ("response_type", "device_code")]),
    )
    .await?;
    if status != 200 {
        return Err(error_text(&body));
    }
    let text = |key: &str| body[key].as_str().unwrap_or_default().to_string();
    Ok(LoginPrompt {
        user_code: text("user_code"),
        verification_uri: text("verification_uri"),
        device_code: text("device_code"),
        interval: body["interval"].as_u64().unwrap_or(5),
        expires_in: body["expires_in"].as_u64().unwrap_or(900),
    })
}

/// Waits until the player approves the code, then signs the account in and saves it as the
/// active one. [`cancel`] stops the wait.
pub async fn finish(client: &reqwest::Client, prompt: LoginPrompt) -> Result<Account, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    *CANCEL.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel.clone());
    let tokens = wait_for_tokens(client, &prompt, &cancel).await;
    *CANCEL.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let account = account_from_tokens(client, &tokens?, None).await?;
    accounts::save_signed_in(account)
}

/// Stops the sign-in that is waiting for the player, if there is one.
pub fn cancel() {
    if let Some(flag) = CANCEL.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

/// The account as a launch needs it. A Microsoft account whose Minecraft token is near its end is
/// renewed first and saved. An offline account passes through unchanged.
pub async fn ensure_fresh(client: &reqwest::Client, account: &Account) -> Result<Account, String> {
    if account.is_offline || !needs_renewal(account, unix_now()) {
        return Ok(account.clone());
    }
    let refresh = account
        .refresh_token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or("This Microsoft account must sign in again. Use Sign in with Microsoft.")?;
    let (status, tokens) = form_post(
        client,
        TOKEN_URL,
        form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("scope", SCOPE),
        ]),
    )
    .await?;
    if status != 200 {
        return Err(format!("{} Sign in with Microsoft again.", error_text(&tokens)));
    }
    let renewed = account_from_tokens(client, &tokens, Some(account)).await?;
    accounts::replace_account(renewed.clone())?;
    Ok(renewed)
}

fn needs_renewal(account: &Account, now: u64) -> bool {
    account.access_token_expires.is_none_or(|expires| now + RENEW_WITHIN_SECS >= expires)
}

/// Polls Microsoft until the player approves, or the code expires, or the sign-in is cancelled.
async fn wait_for_tokens(client: &reqwest::Client, prompt: &LoginPrompt, cancel: &AtomicBool) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(prompt.expires_in);
    let mut wait = prompt.interval.max(5);
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("Sign-in was cancelled.".into());
        }
        if Instant::now() >= deadline {
            return Err("The sign-in code expired. Start again.".into());
        }
        let (status, body) = form_post(
            client,
            TOKEN_URL,
            form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", CLIENT_ID),
                ("device_code", &prompt.device_code),
            ]),
        )
        .await?;
        if status == 200 {
            return Ok(body);
        }
        match body["error"].as_str() {
            Some("authorization_pending") => {}
            Some("slow_down") => wait += 5,
            _ => return Err(error_text(&body)),
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
    }
}

/// Microsoft's tokens to a Minecraft account: Xbox Live, XSTS, Minecraft's login, the ownership
/// check and the profile. `existing` keeps that account's id, place and picks when it is renewed.
async fn account_from_tokens(client: &reqwest::Client, tokens: &Value, existing: Option<&Account>) -> Result<Account, String> {
    let ms_token = tokens["access_token"]
        .as_str()
        .ok_or("Microsoft sent no sign-in token. Try again.")?;
    let refresh_token = tokens["refresh_token"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| existing.and_then(|a| a.refresh_token.clone()));

    let (status, xbox) = json_post(
        client,
        XBOX_URL,
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={ms_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await?;
    let xbox_token = xbox["Token"].as_str().filter(|_| status == 200).ok_or("Xbox Live did not accept this Microsoft account. Try again in a few minutes.")?;

    let (status, xsts) = json_post(
        client,
        XSTS_URL,
        json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if status != 200 {
        return Err(match xsts["XErr"].as_u64() {
            Some(2148916233) => "This Microsoft account has no Xbox profile. Make one at xbox.com, then try again.".into(),
            Some(2148916238) => "This is a child account. An adult has to add it to a family first.".into(),
            _ => "Xbox Live could not sign this account in.".into(),
        });
    }
    let user_hash = xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or("Xbox Live sent no user hash.")?;
    let xsts_token = xsts["Token"].as_str().ok_or("Xbox Live sent no token.")?;

    let (status, login) = json_post(
        client,
        MC_LOGIN_URL,
        json!({ "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}") }),
    )
    .await?;
    let mc_token = login["access_token"]
        .as_str()
        .filter(|_| status == 200)
        .ok_or("Minecraft did not accept this account. Try again later.")?;
    let expires_in = login["expires_in"].as_u64().unwrap_or(86_400);

    let (status, entitlements) = json_get(
        client,
        &format!("{MC_ENTITLEMENTS_URL}?requestId={}", uuid::Uuid::new_v4()),
        mc_token,
    )
    .await?;
    if status != 200 {
        return Err("Minecraft could not check this account's purchases. Try again later.".into());
    }
    let owns_java = entitlements["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| matches!(item["name"].as_str(), Some("product_minecraft" | "game_minecraft")))
    });
    if !owns_java {
        return Err("This Microsoft account does not own Minecraft: Java Edition.".into());
    }

    let (status, profile) = json_get(client, MC_PROFILE_URL, mc_token).await?;
    if status != 200 {
        return Err("This account has no Minecraft profile yet. Open Minecraft once to create it, then sign in again.".into());
    }
    let name = profile["name"].as_str().ok_or("Minecraft sent no player name.")?;
    let id = profile["id"].as_str().ok_or("Minecraft sent no player id.")?;
    let active_skin = profile["skins"]
        .as_array()
        .and_then(|skins| skins.iter().find(|skin| skin["state"] == "ACTIVE"));

    Ok(Account {
        id: existing.map_or_else(|| uuid::Uuid::new_v4().to_string(), |a| a.id.clone()),
        username: name.to_string(),
        uuid: dashed(id),
        is_offline: false,
        skin_preset: existing.and_then(|a| a.skin_preset.clone()),
        cape_preset: existing.and_then(|a| a.cape_preset.clone()),
        skin_url: active_skin.and_then(|skin| skin["url"].as_str()).map(str::to_owned),
        cape_data: existing.and_then(|a| a.cape_data.clone()),
        skin_slim: active_skin.is_some_and(|skin| skin["variant"] == "SLIM"),
        is_active: existing.is_some_and(|a| a.is_active),
        refresh_token,
        access_token: Some(mc_token.to_string()),
        access_token_expires: Some(unix_now() + expires_in),
    })
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", urlencoding::encode(key), urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Microsoft's own words for a refused request, when it gave any.
fn error_text(body: &Value) -> String {
    body["error_description"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("Microsoft did not accept the sign-in. Try again.")
        .to_string()
}

async fn form_post(client: &reqwest::Client, url: &str, body: String) -> Result<(u16, Value), String> {
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Microsoft: {e}"))?;
    let status = response.status().as_u16();
    Ok((status, response.json().await.unwrap_or(Value::Null)))
}

async fn json_post(client: &reqwest::Client, url: &str, body: Value) -> Result<(u16, Value), String> {
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Minecraft's sign-in: {e}"))?;
    let status = response.status().as_u16();
    Ok((status, response.json().await.unwrap_or(Value::Null)))
}

async fn json_get(client: &reqwest::Client, url: &str, token: &str) -> Result<(u16, Value), String> {
    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Could not reach Minecraft's services: {e}"))?;
    let status = response.status().as_u16();
    Ok((status, response.json().await.unwrap_or(Value::Null)))
}

/// Minecraft's 32-digit profile id with the dashes its own tools show.
fn dashed(id: &str) -> String {
    if id.len() != 32 || !id.is_ascii() {
        return id.to_string();
    }
    format!("{}-{}-{}-{}-{}", &id[..8], &id[8..12], &id[12..16], &id[16..20], &id[20..])
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ids_get_the_dashes_minecraft_shows() {
        assert_eq!(dashed("0123456789abcdef0123456789abcdef"), "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(dashed("not-an-id"), "not-an-id");
    }

    #[test]
    fn form_bodies_escape_their_values() {
        assert_eq!(form(&[("scope", "XboxLive.signin offline_access")]), "scope=XboxLive.signin%20offline_access");
    }

    #[test]
    fn a_token_is_renewed_when_it_is_nearly_spent_or_unknown() {
        let now = 1_000_000;
        let with_expiry = |expires| Account {
            access_token_expires: expires,
            ..Account::default()
        };
        assert!(!needs_renewal(&with_expiry(Some(now + 3_600)), now));
        assert!(needs_renewal(&with_expiry(Some(now + 60)), now));
        assert!(needs_renewal(&with_expiry(None), now));
    }
}
