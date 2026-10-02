use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::download::{download_file, verify_sha1};
use crate::minecraft::get_minecraft_dir;

const JAVA_RUNTIME_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

const RUNTIME_NAME: &str = "java-runtime-delta";
const RUNTIME_PLATFORM: &str = "windows-x64";

#[derive(Debug, Deserialize)]
struct RuntimePlatform {
    #[serde(rename = "java-runtime-delta")]
    java_runtime_delta: Vec<RuntimeVersion>,
}

#[derive(Debug, Deserialize)]
struct RuntimeVersion {
    manifest: RuntimeManifest,
    version: RuntimeVersionInfo,
}

#[derive(Debug, Deserialize)]
struct RuntimeVersionInfo {
    name: String,
    released: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    sha1: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeFiles {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Deserialize)]
struct RuntimeFile {
    #[serde(default)]
    downloads: HashMap<String, RuntimeDownload>,
}

#[derive(Debug, Deserialize)]
struct RuntimeDownload {
    sha1: String,
    url: String,
}

fn get_java_runtime_dir() -> Result<PathBuf, String> {
    let minecraft_dir = get_minecraft_dir()?;

    let runtime_dir =
        PathBuf::from(minecraft_dir)
            .join("runtime")
            .join("java");

    std::fs::create_dir_all(&runtime_dir)
        .map_err(|e| {
            format!(
                "Impossible de créer le dossier Java : {}",
                e
            )
        })?;

    Ok(runtime_dir)
}

fn get_java_executable() -> Result<PathBuf, String> {
    Ok(
        get_java_runtime_dir()?
            .join("bin")
            .join("java.exe")
    )
}

#[tauri::command]
pub fn check_java() -> Result<String, String> {
    println!("========================================");
    println!("VÉRIFICATION JAVA");
    println!("========================================");

    let output =
        Command::new("java")
            .arg("-version")
            .output()
            .map_err(|e| {
                format!(
                    "Java système introuvable : {}",
                    e
                )
            })?;

    let stderr =
        String::from_utf8_lossy(
            &output.stderr,
        );

    println!("{}", stderr);

    Ok(stderr.to_string())
}

#[tauri::command]
pub fn get_java_version() -> Result<String, String> {
    let output =
        Command::new("java")
            .arg("-version")
            .output()
            .map_err(|e| {
                format!(
                    "Impossible de lancer Java : {}",
                    e
                )
            })?;

    let stderr =
        String::from_utf8_lossy(
            &output.stderr,
        );

    Ok(stderr.to_string())
}

async fn get_runtime_version()
    -> Result<RuntimeVersion, String>
{
    println!(
        "Récupération du manifest Java Runtime Mojang..."
    );

    let response =
        reqwest::get(
            JAVA_RUNTIME_MANIFEST_URL
        )
        .await
        .map_err(|e| {
            format!(
                "Impossible de récupérer le manifest Java : {}",
                e
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Erreur HTTP {} lors de la récupération du manifest Java.",
            response.status()
        ));
    }

    let data: serde_json::Value =
        response
            .json()
            .await
            .map_err(|e| {
                format!(
                    "Impossible de lire le manifest Java : {}",
                    e
                )
            })?;

    let platform =
        data.get(RUNTIME_PLATFORM)
            .ok_or_else(|| {
                format!(
                    "Plateforme {} absente du manifest Java.",
                    RUNTIME_PLATFORM
                )
            })?;

    let runtimes =
        platform
            .get(RUNTIME_NAME)
            .ok_or_else(|| {
                format!(
                    "Runtime {} absent du manifest Java.",
                    RUNTIME_NAME
                )
            })?;

    let mut versions: Vec<RuntimeVersion> =
        serde_json::from_value(
            runtimes.clone()
        )
        .map_err(|e| {
            format!(
                "Impossible de lire les versions de Java : {}",
                e
            )
        })?;

    versions.sort_by(|a, b| {
        a.version
            .released
            .cmp(&b.version.released)
    });

    versions
        .pop()
        .ok_or_else(|| {
            format!(
                "Aucune version {} disponible pour {}.",
                RUNTIME_NAME,
                RUNTIME_PLATFORM
            )
        })
}

fn safe_runtime_path(
    root: &Path,
    relative: &str,
) -> Result<PathBuf, String> {
    let relative_path =
        PathBuf::from(relative);

    if relative_path.is_absolute() {
        return Err(format!(
            "Chemin Java absolu refusé : {}",
            relative
        ));
    }

    for component in relative_path.components() {
        if matches!(
            component,
            std::path::Component::ParentDir
        ) {
            return Err(format!(
                "Chemin Java invalide : {}",
                relative
            ));
        }
    }

    Ok(root.join(relative_path))
}

#[tauri::command]
pub async fn install_java()
    -> Result<String, String>
{
    println!("========================================");
    println!("INSTALLATION JAVA 21");
    println!("========================================");

    let runtime =
        get_runtime_version().await?;

    println!(
        "Runtime : {}",
        RUNTIME_NAME
    );

    println!(
        "Plateforme : {}",
        RUNTIME_PLATFORM
    );

    println!(
        "Version : {}",
        runtime.version.name
    );

    println!(
        "Release : {}",
        runtime.version.released
    );

    println!(
        "Manifest runtime : {}",
        runtime.manifest.url
    );

    println!(
        "SHA-1 manifest : {}",
        runtime.manifest.sha1
    );

    let runtime_dir =
        get_java_runtime_dir()?;

    println!(
        "Dossier Java : {}",
        runtime_dir.display()
    );

    let manifest_path =
        runtime_dir.join(
            "runtime-manifest.json"
        );

    let manifest_valid =
        verify_sha1(
            &manifest_path,
            &runtime.manifest.sha1,
        )?;

    if !manifest_valid {
        println!(
            "Téléchargement du manifest du runtime..."
        );

        download_file(
            &runtime.manifest.url,
            &manifest_path,
        )
        .await?;

        let valid =
            verify_sha1(
                &manifest_path,
                &runtime.manifest.sha1,
            )?;

        if !valid {
            let _ =
                std::fs::remove_file(
                    &manifest_path
                );

            return Err(
                "SHA-1 incorrect pour le manifest Java."
                    .to_string()
            );
        }

        println!(
            "Manifest Java vérifié."
        );
    } else {
        println!(
            "Manifest Java déjà présent et valide."
        );
    }

    let manifest_data =
        std::fs::read_to_string(
            &manifest_path
        )
        .map_err(|e| {
            format!(
                "Impossible de lire le manifest Java : {}",
                e
            )
        })?;

    let runtime_files: RuntimeFiles =
        serde_json::from_str(
            &manifest_data
        )
        .map_err(|e| {
            format!(
                "Impossible de parser le manifest Java : {}",
                e
            )
        })?;

    println!(
        "{} fichiers Java trouvés.",
        runtime_files.files.len()
    );

    let mut downloaded = 0u64;
    let mut already_present = 0u64;

    for (relative_path, file) in
        runtime_files.files
    {
        let destination =
            safe_runtime_path(
                &runtime_dir,
                &relative_path
            )?;

        let download =
            match file.downloads.get("raw") {
                Some(download) => download,
                None => {
                    println!(
                        "Fichier sans téléchargement raw : {}",
                        relative_path
                    );

                    continue;
                }
            };

        if verify_sha1(
            &destination,
            &download.sha1,
        )? {
            already_present += 1;
            continue;
        }

        println!(
            "Java : {}",
            relative_path
        );

        download_file(
            &download.url,
            &destination,
        )
        .await?;

        let valid =
            verify_sha1(
                &destination,
                &download.sha1,
            )?;

        if !valid {
            let _ =
                std::fs::remove_file(
                    &destination
                );

            return Err(format!(
                "SHA-1 incorrect pour le fichier Java : {}",
                relative_path
            ));
        }

        downloaded += 1;
    }

    let java_exe =
        get_java_executable()?;

    if !java_exe.exists() {
        return Err(format!(
            "Java.exe introuvable après installation : {}",
            java_exe.display()
        ));
    }

    println!("========================================");
    println!("VÉRIFICATION JAVA INSTALLÉ");
    println!("========================================");
    println!(
        "Executable : {}",
        java_exe.display()
    );

    let output =
        Command::new(&java_exe)
            .arg("-version")
            .output()
            .map_err(|e| {
                format!(
                    "Impossible de lancer le Java installé : {}",
                    e
                )
            })?;

    let version_output =
        String::from_utf8_lossy(
            &output.stderr
        );

    println!(
        "{}",
        version_output
    );

    println!("========================================");
    println!("JAVA 21 INSTALLÉ");
    println!("========================================");
    println!(
        "Fichiers téléchargés : {}",
        downloaded
    );
    println!(
        "Fichiers déjà présents : {}",
        already_present
    );
    println!(
        "Java : {}",
        java_exe.display()
    );
    println!("========================================");

    Ok(format!(
        "Java {} installé dans {}",
        runtime.version.name,
        runtime_dir.display()
    ))
}