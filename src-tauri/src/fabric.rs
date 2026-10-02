use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::minecraft::get_minecraft_dir;

const FABRIC_PROFILE_URL: &str =
    "https://meta.fabricmc.net/v2/versions/loader/1.21.11/0.19.3/profile/json";

#[derive(Debug, Serialize, Deserialize)]
struct FabricProfile {
    id: String,

    #[serde(rename = "inheritsFrom")]
    inherits_from: String,

    #[serde(rename = "releaseTime")]
    release_time: String,

    time: String,

    #[serde(rename = "type")]
    profile_type: Option<String>,

    #[serde(rename = "mainClass")]
    main_class: String,

    #[serde(default)]
    arguments: Option<FabricArguments>,

    #[serde(rename = "jvmArguments")]
    #[serde(default)]
    jvm_arguments: Option<Vec<String>>,

    #[serde(default)]
    libraries: Vec<FabricLibrary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricArguments {
    #[serde(default)]
    game: Vec<String>,

    #[serde(default)]
    jvm: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricLibrary {
    name: String,

    #[serde(default)]
    url: Option<String>,

    #[serde(default)]
    downloads: Option<FabricLibraryDownloads>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricLibraryDownloads {
    #[serde(default)]
    artifact: Option<FabricArtifact>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricArtifact {
    path: String,
    url: String,

    #[serde(default)]
    sha1: Option<String>,

    #[serde(default)]
    size: Option<u64>,
}

#[tauri::command]
pub async fn install_fabric() -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION FABRIC");
    println!("========================================");
    println!("Minecraft : 1.21.11");
    println!("Fabric    : 0.19.3");

    // ========================================
    // 1. RÉCUPÉRATION DU PROFIL FABRIC
    // ========================================

    let response = reqwest::get(FABRIC_PROFILE_URL)
        .await
        .map_err(|e| {
            format!(
                "Erreur récupération du profil Fabric : {}",
                e
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Erreur HTTP Fabric : {}",
            response.status()
        ));
    }

    let json_bytes = response
        .bytes()
        .await
        .map_err(|e| {
            format!(
                "Impossible de lire le profil Fabric : {}",
                e
            )
        })?;

    // ========================================
    // 2. PARSING DU PROFIL
    // ========================================

    let profile: FabricProfile =
        serde_json::from_slice(&json_bytes)
            .map_err(|e| {
                format!(
                    "Erreur lecture du profil Fabric : {}",
                    e
                )
            })?;

    println!("Profil Fabric trouvé.");
    println!("ID           : {}", profile.id);
    println!(
        "Minecraft    : {}",
        profile.inherits_from
    );
    println!(
        "Release time : {}",
        profile.release_time
    );
    println!("Time         : {}", profile.time);
    println!(
        "Main class   : {}",
        profile.main_class
    );

    if let Some(profile_type) = &profile.profile_type {
        println!(
            "Type         : {}",
            profile_type
        );
    }

    println!(
        "Libraries Fabric : {}",
        profile.libraries.len()
    );

    // ========================================
    // 3. AFFICHAGE DES ARGUMENTS
    // ========================================

    if let Some(arguments) = &profile.arguments {
        println!(
            "Arguments game : {}",
            arguments.game.len()
        );

        println!(
            "Arguments JVM  : {}",
            arguments.jvm.len()
        );
    }

    if let Some(jvm_arguments) = &profile.jvm_arguments {
        println!(
            "JVM arguments supplémentaires : {}",
            jvm_arguments.len()
        );
    }

    // ========================================
    // 4. DOSSIER .ETERNIA
    // ========================================

    let minecraft_dir =
        PathBuf::from(get_minecraft_dir()?);

    // ========================================
    // 5. DOSSIER FABRIC
    // ========================================

    let fabric_dir = minecraft_dir
        .join("versions")
        .join("1.21.11-fabric");

    std::fs::create_dir_all(&fabric_dir)
        .map_err(|e| {
            format!(
                "Impossible de créer le dossier Fabric : {}",
                e
            )
        })?;

    // ========================================
    // 6. SAUVEGARDE DU PROFIL
    // ========================================

    let profile_path = fabric_dir
        .join("1.21.11-fabric.json");

    let formatted_json =
        serde_json::to_string_pretty(&profile)
            .map_err(|e| {
                format!(
                    "Impossible de formater le profil Fabric : {}",
                    e
                )
            })?;

    std::fs::write(
        &profile_path,
        formatted_json,
    )
    .map_err(|e| {
        format!(
            "Impossible d'écrire le profil Fabric : {}",
            e
        )
    })?;

    println!(
        "Profil Fabric enregistré : {}",
        profile_path.display()
    );

    // ========================================
    // 7. RÉSUMÉ
    // ========================================

    println!("========================================");
    println!("PROFIL FABRIC PRÊT POUR LE LANCEMENT");
    println!("========================================");

    println!(
        "Main class : {}",
        profile.main_class
    );

    println!(
        "Libraries : {}",
        profile.libraries.len()
    );

    if let Some(arguments) = &profile.arguments {
        println!(
            "Game arguments : {}",
            arguments.game.len()
        );

        println!(
            "JVM arguments : {}",
            arguments.jvm.len()
        );
    }

    println!("========================================");
    println!("FABRIC INSTALLÉ");
    println!("========================================");

    Ok(format!(
        "Fabric {} installé pour Minecraft {}.",
        profile.id,
        profile.inherits_from
    ))
}