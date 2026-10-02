use serde::{Deserialize, Serialize};

use std::fs;

use std::path::PathBuf;

use std::sync::Mutex;

use std::sync::{
    atomic::AtomicBool,
    Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSession {
    pub minecraft_uuid: String,
    pub minecraft_name: String,
    pub minecraft_access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountsFile {
    pub accounts: Vec<MinecraftSession>,
    pub selected_uuid: Option<String>,
}

pub struct SessionState {
    pub session: Mutex<Option<MinecraftSession>>,
    pub accounts: Mutex<Vec<MinecraftSession>>,
    pub cancel_login: Arc<AtomicBool>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            accounts: Mutex::new(Vec::new()),
            cancel_login: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn session_file() -> Result<PathBuf, String> {
    let app_data = dirs::data_local_dir()
        .ok_or_else(|| {
            "Impossible de trouver le dossier AppData.".to_string()
        })?;

    let eternia_dir = app_data.join("Eternia");

    fs::create_dir_all(&eternia_dir)
        .map_err(|e| {
            format!(
                "Impossible de créer le dossier Eternia : {}",
                e
            )
        })?;

    Ok(eternia_dir.join("accounts.json"))
}

pub fn save_accounts_to_disk(
    accounts: &[MinecraftSession],
    selected_uuid: Option<&str>,
) -> Result<(), String> {
    let path = session_file()?;

    let data = AccountsFile {
        accounts: accounts.to_vec(),
        selected_uuid: selected_uuid.map(str::to_string),
    };

    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| {
            format!(
                "Impossible de convertir les comptes : {}",
                e
            )
        })?;

    fs::write(&path, json)
        .map_err(|e| {
            format!(
                "Impossible de sauvegarder les comptes : {}",
                e
            )
        })?;

    println!("Comptes Eternia : sauvegardés sur le disque.");

    Ok(())
}

pub fn load_accounts_from_disk()
    -> Result<AccountsFile, String>
{
    let path = session_file()?;

    if !path.exists() {
        return Ok(AccountsFile {
            accounts: Vec::new(),
            selected_uuid: None,
        });
    }

    let json = fs::read_to_string(&path)
        .map_err(|e| {
            format!(
                "Impossible de lire les comptes Eternia : {}",
                e
            )
        })?;

    serde_json::from_str(&json)
        .map_err(|e| {
            format!(
                "Fichier accounts.json invalide : {}",
                e
            )
        })
}

pub fn load_session_from_disk()
    -> Result<Option<MinecraftSession>, String>
{
    let data = load_accounts_from_disk()?;

    if let Some(uuid) = data.selected_uuid {
        if let Some(account) = data
            .accounts
            .iter()
            .find(|account| account.minecraft_uuid == uuid)
        {
            println!(
                "Session Eternia : compte sauvegardé trouvé ({})",
                account.minecraft_name
            );

            return Ok(Some(account.clone()));
        }
    }

    if let Some(account) = data.accounts.first() {
        println!(
            "Session Eternia : premier compte trouvé ({})",
            account.minecraft_name
        );

        return Ok(Some(account.clone()));
    }

    Ok(None)
}

#[tauri::command]
pub fn get_session(
    state: tauri::State<'_, SessionState>,
) -> Result<Option<MinecraftSession>, String> {
    let session = state
        .session
        .lock()
        .map_err(|_| {
            "Impossible d'accéder à la session Eternia.".to_string()
        })?;

    Ok(session.clone())
}

#[tauri::command]
pub fn get_accounts(
    state: tauri::State<'_, SessionState>,
) -> Result<Vec<MinecraftSession>, String> {
    let accounts = state
        .accounts
        .lock()
        .map_err(|_| {
            "Impossible d'accéder aux comptes Eternia.".to_string()
        })?;

    Ok(accounts.clone())
}

#[tauri::command]
pub fn select_account(
    uuid: String,
    state: tauri::State<'_, SessionState>,
) -> Result<MinecraftSession, String> {
    let selected_account = {
        let accounts = state
            .accounts
            .lock()
            .map_err(|_| {
                "Impossible d'accéder aux comptes Eternia.".to_string()
            })?;

        accounts
            .iter()
            .find(|account| account.minecraft_uuid == uuid)
            .cloned()
            .ok_or_else(|| {
                "Compte Eternia introuvable.".to_string()
            })?
    };

    {
        let mut session = state
            .session
            .lock()
            .map_err(|_| {
                "Impossible d'accéder à la session Eternia.".to_string()
            })?;

        *session = Some(selected_account.clone());
    }

    let accounts = state
        .accounts
        .lock()
        .map_err(|_| {
            "Impossible d'accéder aux comptes Eternia.".to_string()
        })?;

    save_accounts_to_disk(
        &accounts,
        Some(&selected_account.minecraft_uuid),
    )?;

    println!(
        "Compte Eternia sélectionné : {}",
        selected_account.minecraft_name
    );

    Ok(selected_account)
}

#[tauri::command]
pub fn remove_account(
    uuid: String,
    state: tauri::State<'_, SessionState>,
) -> Result<(), String> {
    let mut accounts = state
        .accounts
        .lock()
        .map_err(|_| {
            "Impossible d'accéder aux comptes Eternia.".to_string()
        })?;

    accounts.retain(|account| account.minecraft_uuid != uuid);

    let new_selected = accounts.first().cloned();

    {
        let mut session = state
            .session
            .lock()
            .map_err(|_| {
                "Impossible d'accéder à la session Eternia.".to_string()
            })?;

        *session = new_selected.clone();
    }

    save_accounts_to_disk(
        &accounts,
        new_selected
            .as_ref()
            .map(|account| account.minecraft_uuid.as_str()),
    )?;

    println!("Compte Eternia supprimé : {}", uuid);

    Ok(())
}