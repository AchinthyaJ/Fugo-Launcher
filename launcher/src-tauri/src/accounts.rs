use crate::models::Account;
use crate::settings::get_app_dir;
use std::fs;

pub fn load_accounts() -> Vec<Account> {
    let file = get_app_dir().join("accounts.json");
    if let Ok(data) = fs::read_to_string(&file)
        && let Ok(accs) = serde_json::from_str::<Vec<Account>>(&data)
            && !accs.is_empty() {
                return accs;
            }

    // No account yet: the launcher's welcome asks for one before anything else is shown.
    Vec::new()
}

/// Minecraft's rule for an offline name: 3 to 16 letters, digits or underscores.
pub fn valid_offline_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::valid_offline_name;

    #[test]
    fn offline_names_follow_minecraft_s_rule() {
        assert!(valid_offline_name("Ari"));
        assert!(valid_offline_name("Steve_01"));
        assert!(valid_offline_name("abcdefghijklmnop"));
        assert!(!valid_offline_name("ab"));
        assert!(!valid_offline_name("abcdefghijklmnopq"));
        assert!(!valid_offline_name("two words"));
        assert!(!valid_offline_name("bad-name"));
        assert!(!valid_offline_name("héllo"));
    }
}

pub fn save_accounts(accounts: &[Account]) -> Result<(), String> {
    let file = get_app_dir().join("accounts.json");
    let json = serde_json::to_string_pretty(accounts).map_err(|e| e.to_string())?;
    fs::write(file, json).map_err(|e| e.to_string())
}

pub fn get_active_account() -> Option<Account> {
    let accounts = load_accounts();
    accounts.into_iter().find(|a| a.is_active)
}

pub fn set_active_account(id: &str) -> Result<(), String> {
    let mut accounts = load_accounts();
    for acc in accounts.iter_mut() {
        acc.is_active = acc.id == id;
    }
    save_accounts(&accounts)
}

pub fn create_offline_account(username: &str, skin: Option<String>, cape: Option<String>) -> Result<Account, String> {
    if !valid_offline_name(username) {
        return Err("Names are 3 to 16 letters, digits or underscores.".into());
    }
    let mut accounts = load_accounts();
    for acc in accounts.iter_mut() {
        acc.is_active = false;
    }
    let new_acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        username: username.to_string(),
        uuid: uuid::Uuid::new_v4().to_string(),
        is_offline: true,
        skin_preset: skin.or_else(|| Some("Steve".to_string())),
        cape_preset: cape,
        skin_url: None,
        cape_data: None,
        skin_slim: false,
        is_active: true,
        refresh_token: None,
        access_token: None,
        access_token_expires: None,
    };
    accounts.push(new_acc.clone());
    save_accounts(&accounts)?;
    Ok(new_acc)
}

/// Saves an account that just signed in with Microsoft and makes it the active one. An earlier
/// sign-in of the same Minecraft profile is replaced, not duplicated.
pub fn save_signed_in(mut account: Account) -> Result<Account, String> {
    let mut accounts = load_accounts();
    accounts.retain(|a| a.is_offline || a.uuid != account.uuid);
    for acc in accounts.iter_mut() {
        acc.is_active = false;
    }
    account.is_active = true;
    accounts.push(account.clone());
    save_accounts(&accounts)?;
    Ok(account)
}

/// Replaces the saved account with the same id, keeping its place: a renewed Microsoft sign-in.
pub fn replace_account(account: Account) -> Result<(), String> {
    let mut accounts = load_accounts();
    if let Some(slot) = accounts.iter_mut().find(|a| a.id == account.id) {
        *slot = account;
    }
    save_accounts(&accounts)
}

pub fn delete_account(id: &str) -> Result<(), String> {
    let mut accounts = load_accounts();
    accounts.retain(|a| a.id != id);
    if !accounts.is_empty() && !accounts.iter().any(|a| a.is_active) {
        accounts[0].is_active = true;
    }
    save_accounts(&accounts)
}

/// Saves the active account's skin: a preset name, or a picked skin image (which wins when set).
pub fn set_active_skin(skin_url: Option<String>, preset: Option<String>, slim: bool) -> Result<(), String> {
    let mut accounts = load_accounts();
    let active = accounts.iter_mut().find(|a| a.is_active).ok_or("No active account")?;
    active.skin_url = skin_url;
    active.skin_slim = slim;
    if preset.is_some() {
        active.skin_preset = preset;
    }
    save_accounts(&accounts)
}

pub fn set_active_cape(cape: Option<String>, cape_data: Option<String>) -> Result<(), String> {
    let mut accounts = load_accounts();
    let active = accounts.iter_mut().find(|a| a.is_active).ok_or("No active account")?;
    active.cape_preset = cape;
    active.cape_data = cape_data;
    save_accounts(&accounts)
}
