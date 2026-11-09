use crate::models::Account;
use crate::settings::get_app_dir;
use std::fs;

pub fn load_accounts() -> Vec<Account> {
    let file = get_app_dir().join("accounts.json");
    if let Ok(data) = fs::read_to_string(&file) {
        if let Ok(accs) = serde_json::from_str::<Vec<Account>>(&data) {
            if !accs.is_empty() {
                return accs;
            }
        }
    }

    // Default initial account
    let default_acc = Account {
        id: uuid::Uuid::new_v4().to_string(),
        username: "FugoPlayer".to_string(),
        uuid: uuid::Uuid::new_v4().to_string(),
        is_offline: true,
        skin_preset: Some("Steve".to_string()),
        cape_preset: Some("2016".to_string()),
        skin_url: None,
        cape_data: None,
        is_active: true,
    };
    let accs = vec![default_acc];
    let _ = save_accounts(&accs);
    accs
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
        is_active: true,
    };
    accounts.push(new_acc.clone());
    save_accounts(&accounts)?;
    Ok(new_acc)
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
pub fn set_active_skin(skin_url: Option<String>, preset: Option<String>) -> Result<(), String> {
    let mut accounts = load_accounts();
    let active = accounts.iter_mut().find(|a| a.is_active).ok_or("No active account")?;
    active.skin_url = skin_url;
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
