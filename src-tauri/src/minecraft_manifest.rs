use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use crate::download::{download_file, verify_sha1};
use crate::minecraft::get_minecraft_dir;

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

const ASSET_BASE_URL: &str = "https://resources.download.minecraft.net/";

#[derive(Debug, Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<Version>,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VersionJson {
    pub id: String,
    pub downloads: Downloads,

    #[serde(default)]
    pub libraries: Vec<MinecraftLibrary>,

    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,

    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JavaVersion {
    pub component: String,

    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Downloads {
    pub client: DownloadInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetsFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MinecraftLibrary {
    pub name: String,

    #[serde(default)]
    pub downloads: Option<MinecraftLibraryDownloads>,

    #[serde(default)]
    pub rules: Option<Vec<LibraryRule>>,

    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MinecraftLibraryDownloads {
    #[serde(default)]
    pub artifact: Option<MinecraftArtifact>,

    #[serde(default)]
    pub classifiers: Option<HashMap<String, MinecraftArtifact>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MinecraftArtifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LibraryRule {
    pub action: String,

    #[serde(default)]
    pub os: Option<LibraryOs>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LibraryOs {
    pub name: Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    current: u64,
    total: u64,
    percentage: u32,
    message: String,
}

fn emit_download_progress(app: &AppHandle, current: u64, total: u64, message: &str) {
    let percentage = if total == 0 {
        100
    } else {
        ((current * 100) / total) as u32
    };

    let progress = DownloadProgress {
        current,
        total,
        percentage,
        message: message.to_string(),
    };

    if let Err(error) = app.emit("launcher-download-progress", progress) {
        eprintln!("Impossible d'envoyer la progression : {}", error);
    }
}

pub async fn get_version_url(version: &str) -> Result<String, String> {
    let manifest: VersionManifest = reqwest::get(VERSION_MANIFEST_URL)
        .await
        .map_err(|e| format!("Erreur récupération du manifest : {}", e))?
        .json()
        .await
        .map_err(|e| format!("Erreur lecture du manifest : {}", e))?;

    manifest
        .versions
        .into_iter()
        .find(|v| v.id == version)
        .map(|v| v.url)
        .ok_or_else(|| format!("Version Minecraft introuvable : {}", version))
}

fn library_allowed_on_windows(rules: &Option<Vec<LibraryRule>>) -> bool {
    let rules = match rules {
        Some(rules) => rules,
        None => return true,
    };

    let mut allowed = true;

    for rule in rules {
        let matches = match &rule.os {
            Some(os) => match &os.name {
                Some(name) => name == "windows",
                None => true,
            },
            None => true,
        };

        if matches {
            allowed = rule.action == "allow";
        }
    }

    allowed
}

fn library_is_windows_x64_native(name: &str) -> bool {
    name.ends_with(":natives-windows")
}

fn library_is_other_native(name: &str) -> bool {
    name.contains(":natives-")
}

async fn download_and_verify_artifact(
    artifact: &MinecraftArtifact,
    libraries_dir: &PathBuf,
    description: &str,
) -> Result<bool, String> {
    let destination = libraries_dir.join(&artifact.path);

    let valid = verify_sha1(&destination, &artifact.sha1)?;

    if valid {
        println!("Library déjà présente : {}", description);

        return Ok(false);
    }

    println!("Téléchargement library : {}", description);

    println!("URL : {}", artifact.url);

    download_file(&artifact.url, &destination).await?;

    let valid = verify_sha1(&destination, &artifact.sha1)?;

    if !valid {
        let _ = std::fs::remove_file(&destination);

        return Err(format!("SHA-1 incorrect pour la library : {}", description));
    }

    Ok(true)
}

#[tauri::command]
pub async fn check_minecraft_version(version: String) -> Result<String, String> {
    println!("========================================");
    println!("VÉRIFICATION VERSION MINECRAFT");
    println!("========================================");
    println!("Version demandée : {}", version);

    let version_url = get_version_url(&version).await?;

    println!("Manifest Minecraft trouvé : {}", version_url);

    Ok(version_url)
}

#[tauri::command]
pub async fn install_minecraft(version: String) -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION MINECRAFT");
    println!("========================================");
    println!("Version : {}", version);

    let version_url = get_version_url(&version).await?;

    let version_json: VersionJson = reqwest::get(&version_url)
        .await
        .map_err(|e| format!("Erreur récupération JSON Minecraft : {}", e))?
        .json()
        .await
        .map_err(|e| format!("Erreur lecture JSON Minecraft : {}", e))?;

    let minecraft_dir = get_minecraft_dir()?;

    if let Some(java) = &version_json.java_version {
        println!("========================================");
        println!("JAVA REQUIS PAR MINECRAFT");
        println!("========================================");
        println!("Composant : {}", java.component);
        println!("Version   : {}", java.major_version);
        println!("========================================");
    }

    let version_dir = PathBuf::from(&minecraft_dir)
        .join("versions")
        .join(&version);

    std::fs::create_dir_all(&version_dir)
        .map_err(|e| format!("Impossible de créer le dossier version : {}", e))?;

    let json_path = version_dir.join(format!("{}.json", version));

    let json_data = serde_json::to_string_pretty(&version_json)
        .map_err(|e| format!("Impossible de sérialiser le JSON : {}", e))?;

    std::fs::write(&json_path, json_data)
        .map_err(|e| format!("Impossible d'enregistrer le JSON : {}", e))?;

    println!("JSON enregistré : {}", json_path.display());

    let client_path = version_dir.join(format!("{}.jar", version));

    let client_valid = verify_sha1(&client_path, &version_json.downloads.client.sha1)?;

    if client_valid {
        println!("Client Minecraft déjà présent et valide.");
    } else {
        println!("Téléchargement du client Minecraft...");

        println!("URL : {}", version_json.downloads.client.url);

        download_file(&version_json.downloads.client.url, &client_path).await?;

        let valid = verify_sha1(&client_path, &version_json.downloads.client.sha1)?;

        if !valid {
            let _ = std::fs::remove_file(&client_path);

            return Err("SHA-1 incorrect pour le client Minecraft.".to_string());
        }

        println!("Client Minecraft vérifié avec succès.");
    }

    println!("MINECRAFT INSTALLÉ");

    Ok(format!("Minecraft {} installé.", version))
}

#[tauri::command]
pub async fn install_minecraft_libraries(version: String) -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION LIBRARIES MINECRAFT");
    println!("========================================");
    println!("Version : {}", version);

    let version_url = get_version_url(&version).await?;

    let version_json: VersionJson = reqwest::get(&version_url)
        .await
        .map_err(|e| format!("Erreur récupération JSON Minecraft : {}", e))?
        .json()
        .await
        .map_err(|e| format!("Erreur lecture JSON Minecraft : {}", e))?;

    let minecraft_dir = get_minecraft_dir()?;

    let libraries_dir = PathBuf::from(&minecraft_dir).join("libraries");

    std::fs::create_dir_all(&libraries_dir)
        .map_err(|e| format!("Impossible de créer libraries : {}", e))?;

    println!(
        "{} libraries trouvées dans Minecraft {}.",
        version_json.libraries.len(),
        version
    );

    let mut downloaded = 0;
    let mut already_present = 0;
    let mut skipped = 0;

    for library in version_json.libraries {
        if !library_allowed_on_windows(&library.rules) {
            println!("Library ignorée par les règles : {}", library.name);

            skipped += 1;
            continue;
        }

        if library_is_other_native(&library.name) {
            if !library_is_windows_x64_native(&library.name) {
                println!("Native non compatible avec Windows x64 : {}", library.name);

                skipped += 1;
                continue;
            }

            println!("Native Windows x64 sélectionnée : {}", library.name);
        }

        let downloads = match &library.downloads {
            Some(downloads) => downloads,
            None => {
                println!("Library sans téléchargement : {}", library.name);

                skipped += 1;
                continue;
            }
        };

        if let Some(artifact) = &downloads.artifact {
            let was_downloaded =
                download_and_verify_artifact(artifact, &libraries_dir, &library.name).await?;

            if was_downloaded {
                downloaded += 1;
            } else {
                already_present += 1;
            }
        }
    }

    println!("========================================");
    println!("LIBRARIES MINECRAFT TERMINÉES");
    println!("========================================");
    println!("Libraries téléchargées : {}", downloaded);
    println!("Libraries présentes     : {}", already_present);
    println!("Libraries ignorées      : {}", skipped);
    println!("========================================");

    Ok(format!(
        "{} libraries téléchargées, {} déjà présentes, {} libraries ignorées.",
        downloaded, already_present, skipped
    ))
}

#[tauri::command]
pub async fn install_minecraft_assets(app: AppHandle, version: String) -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION ASSETS MINECRAFT");
    println!("========================================");
    println!("Version : {}", version);

    let version_url = get_version_url(&version).await?;

    let version_json: VersionJson = reqwest::get(&version_url)
        .await
        .map_err(|e| format!("Erreur récupération JSON Minecraft : {}", e))?
        .json()
        .await
        .map_err(|e| format!("Erreur lecture JSON Minecraft : {}", e))?;

    println!("Asset index : {}", version_json.asset_index.id);

    println!("URL asset index : {}", version_json.asset_index.url);

    let minecraft_dir = get_minecraft_dir()?;

    let assets_dir = PathBuf::from(&minecraft_dir).join("assets");

    let indexes_dir = assets_dir.join("indexes");

    let objects_dir = assets_dir.join("objects");

    std::fs::create_dir_all(&indexes_dir)
        .map_err(|e| format!("Impossible de créer assets/indexes : {}", e))?;

    std::fs::create_dir_all(&objects_dir)
        .map_err(|e| format!("Impossible de créer assets/objects : {}", e))?;

    let index_path = indexes_dir.join(format!("{}.json", version_json.asset_index.id));

    let index_valid = verify_sha1(&index_path, &version_json.asset_index.sha1)?;

    if index_valid {
        println!("Asset index déjà présent et valide.");
    } else {
        println!("Téléchargement de l'asset index...");

        download_file(&version_json.asset_index.url, &index_path).await?;

        let valid = verify_sha1(&index_path, &version_json.asset_index.sha1)?;

        if !valid {
            let _ = std::fs::remove_file(&index_path);

            return Err("SHA-1 incorrect pour l'asset index.".to_string());
        }

        println!("Asset index vérifié avec succès.");
    }

    let index_data = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("Impossible de lire l'asset index : {}", e))?;

    let assets: AssetsFile = serde_json::from_str(&index_data)
        .map_err(|e| format!("Impossible de lire les assets : {}", e))?;

    println!("{} assets trouvés.", assets.objects.len());

    let mut downloaded = 0u64;
    let mut already_present = 0u64;

    let total_assets = assets.objects.len() as u64;
    let mut processed_assets = 0u64;

    emit_download_progress(&app, 0, total_assets, "Préparation des assets...");

    for (name, object) in assets.objects {
        let prefix = &object.hash[..2];

        let destination_dir = objects_dir.join(prefix);

        let destination = destination_dir.join(&object.hash);

        if verify_sha1(&destination, &object.hash)? {
            already_present += 1;
            processed_assets += 1;

            emit_download_progress(
                &app,
                processed_assets,
                total_assets,
                "Vérification des assets...",
            );

            continue;
        }

        let url = format!("{}{}/{}", ASSET_BASE_URL, prefix, object.hash);

        println!("Asset : {}", name);

        download_file(&url, &destination).await?;

        let valid = verify_sha1(&destination, &object.hash)?;

        if !valid {
            let _ = std::fs::remove_file(&destination);

            return Err(format!("SHA-1 incorrect pour l'asset : {}", name));
        }

        downloaded += 1;
        processed_assets += 1;

        emit_download_progress(
            &app,
            processed_assets,
            total_assets,
            "Téléchargement des assets...",
        );
    }

    println!("========================================");
    println!("ASSETS MINECRAFT TERMINÉS");
    println!("========================================");
    println!("Assets téléchargés : {}", downloaded);
    println!("Assets déjà présents : {}", already_present);
    println!("========================================");

    Ok(format!(
        "{} assets téléchargés, {} déjà présents.",
        downloaded, already_present
    ))
}
