use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::download::{download_file, verify_sha1};
use crate::minecraft::get_minecraft_dir;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    files: Vec<ModrinthFile>,
}

#[derive(Debug, Deserialize)]
struct ModrinthFile {
    filename: String,
    url: String,
    hashes: ModrinthHashes,
    #[serde(rename = "primary")]
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct ModrinthHashes {
    sha1: String,
}

#[derive(Clone, serde::Serialize)]
struct ModDownloadProgress {
    current: u64,
    total: u64,
    percentage: u32,
    message: String,
}

struct EterniaMod {
    name: &'static str,
    version_id: &'static str,
}

const ETERNIA_MODS: &[EterniaMod] = &[
    EterniaMod {
        name: "Emotecraft",
        version_id: "Hwqp4xhc",
    },
    EterniaMod {
        name: "Fabric API",
        version_id: "6qAuTtLR",
    },
    EterniaMod {
        name: "Iris Shaders",
        version_id: "fDpuVzVr",
    },
    EterniaMod {
        name: "Player Animation Library",
        version_id: "wJLdv4ZQ",
    },
    EterniaMod {
        name: "Simple Voice Chat",
        version_id: "CN53keBo",
    },
    EterniaMod {
        name: "Sodium",
        version_id: "UddlN6L4",
    },
];

fn emit_mod_progress(
    app: &AppHandle,
    current: u64,
    total: u64,
    message: &str,
) {
    let percentage = if total == 0 {
        100
    } else {
        ((current * 100) / total) as u32
    };

    let progress = ModDownloadProgress {
        current,
        total,
        percentage,
        message: message.to_string(),
    };

    if let Err(error) = app.emit(
        "launcher-download-progress",
        progress,
    ) {
        eprintln!(
            "Impossible d'envoyer la progression des mods : {}",
            error
        );
    }
}

async fn get_modrinth_version(
    version_id: &str,
) -> Result<ModrinthVersion, String> {
    let url = format!(
        "{}/version/{}",
        MODRINTH_API,
        version_id
    );

    reqwest::get(&url)
        .await
        .map_err(|e| {
            format!(
                "Impossible de contacter Modrinth : {}",
                e
            )
        })?
        .error_for_status()
        .map_err(|e| {
            format!(
                "Erreur Modrinth pour la version {} : {}",
                version_id,
                e
            )
        })?
        .json::<ModrinthVersion>()
        .await
        .map_err(|e| {
            format!(
                "Impossible de lire la réponse Modrinth : {}",
                e
            )
        })
}

async fn install_single_mod(
    _app: &AppHandle,
    mod_info: &EterniaMod,
    mods_dir: &PathBuf,
) -> Result<(), String> {
    let version = get_modrinth_version(
        mod_info.version_id
    )
    .await?;

    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| {
            format!(
                "Aucun fichier trouvé pour le mod {}.",
                mod_info.name
            )
        })?;

    let destination = mods_dir.join(&file.filename);

    println!(
        "[MODS] {} - {}",
        mod_info.name,
        file.filename
    );

    if destination.exists() {
        if verify_sha1(
            &destination,
            &file.hashes.sha1,
        )? {
            println!(
                "[MODS] {} déjà présent et valide.",
                mod_info.name
            );

            return Ok(());
        }

        println!(
            "[MODS] {} invalide, remplacement...",
            mod_info.name
        );

        fs::remove_file(&destination)
            .map_err(|e| {
                format!(
                    "Impossible de supprimer le mod {} : {}",
                    mod_info.name,
                    e
                )
            })?;
    }

    println!(
        "[MODS] Téléchargement de {}...",
        mod_info.name
    );

    download_file(
        &file.url,
        &destination,
    )
    .await?;

    if !verify_sha1(
        &destination,
        &file.hashes.sha1,
    )? {
        let _ = fs::remove_file(&destination);

        return Err(format!(
            "SHA-1 invalide pour le mod {}.",
            mod_info.name
        ));
    }

    println!(
        "[MODS] {} installé.",
        mod_info.name
    );

    Ok(())
}

#[tauri::command]
pub async fn install_eternia_mods(
    app: AppHandle,
) -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION DES MODS ETERNIA");
    println!("========================================");

    let minecraft_dir = PathBuf::from(
        get_minecraft_dir()?
    );

    let mods_dir = minecraft_dir.join("mods");

    fs::create_dir_all(&mods_dir)
        .map_err(|e| {
            format!(
                "Impossible de créer le dossier mods : {}",
                e
            )
        })?;

    let total = ETERNIA_MODS.len() as u64;
    let mut current = 0u64;

    emit_mod_progress(
        &app,
        0,
        total,
        "Préparation des mods Eternia...",
    );

    for mod_info in ETERNIA_MODS {
        emit_mod_progress(
            &app,
            current,
            total,
            &format!(
                "Vérification de {}...",
                mod_info.name
            ),
        );

        install_single_mod(
            &app,
            mod_info,
            &mods_dir,
        )
        .await?;

        current += 1;

        emit_mod_progress(
            &app,
            current,
            total,
            &format!(
                "{} prêt.",
                mod_info.name
            ),
        );
    }

    println!("========================================");
    println!("MODS ETERNIA TERMINÉS");
    println!("========================================");
    println!(
        "{} mods vérifiés.",
        ETERNIA_MODS.len()
    );
    println!(
        "Dossier : {}",
        mods_dir.display()
    );
    println!("========================================");

    Ok(format!(
        "{} mods Eternia vérifiés.",
        ETERNIA_MODS.len()
    ))
}