
use serde::Deserialize;
use std::path::PathBuf;

use crate::download::{download_file, verify_sha1};
use crate::minecraft::get_minecraft_dir;

const DEFAULT_MAVEN_URL: &str = "https://maven.fabricmc.net/";

#[derive(Debug, Deserialize)]
pub struct Library {
    pub name: String,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub url: String,

    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FabricProfile {
    pub libraries: Vec<Library>,
}

struct MavenArtifact {
    path: String,
    url: String,
}

fn resolve_maven_artifact(
    name: &str,
    repository: &str,
) -> Result<MavenArtifact, String> {
    let (coordinates, extension) = match name.split_once('@') {
        Some((coordinates, extension)) => {
            (coordinates, extension)
        }
        None => (name, "jar"),
    };

    let parts: Vec<&str> = coordinates.split(':').collect();

    if parts.len() < 3 || parts.len() > 4 {
        return Err(format!(
            "Coordonnées Maven invalides : {}",
            name
        ));
    }

    let group = parts[0];
    let artifact = parts[1];
    let version = parts[2];

    let classifier = parts.get(3).copied();

    let group_path = group.replace('.', "/");

    let filename = match classifier {
        Some(classifier) => format!(
            "{}-{}-{}.{}",
            artifact, version, classifier, extension
        ),
        None => format!(
            "{}-{}.{}",
            artifact, version, extension
        ),
    };

    let path = format!(
        "{}/{}/{}/{}",
        group_path, artifact, version, filename
    );

    let base_url = if repository.ends_with('/') {
        repository.to_string()
    } else {
        format!("{}/", repository)
    };

    let url = format!("{}{}", base_url, path);

    Ok(MavenArtifact { path, url })
}

#[tauri::command]
pub async fn install_libraries() -> Result<String, String> {
    println!("========================================");
    println!("INSTALLATION DES LIBRARIES");
    println!("========================================");

    let minecraft_dir =
        PathBuf::from(get_minecraft_dir()?);

    let fabric_json = minecraft_dir
        .join("versions")
        .join("1.21.11-fabric")
        .join("1.21.11-fabric.json");

    if !fabric_json.exists() {
        return Err(
            "Le profil Fabric n'existe pas. Installe Fabric d'abord."
                .to_string(),
        );
    }

    let json = std::fs::read(&fabric_json)
        .map_err(|e| {
            format!(
                "Impossible de lire le profil Fabric : {}",
                e
            )
        })?;

    let profile: FabricProfile =
        serde_json::from_slice(&json)
            .map_err(|e| {
                format!(
                    "Impossible de lire les libraries Fabric : {}",
                    e
                )
            })?;

    println!(
        "{} libraries trouvées dans le profil Fabric.",
        profile.libraries.len()
    );

    let libraries_dir = minecraft_dir.join("libraries");

    std::fs::create_dir_all(&libraries_dir)
        .map_err(|e| {
            format!(
                "Impossible de créer le dossier libraries : {}",
                e
            )
        })?;

    let mut downloaded = 0;
    let mut already_present = 0;

    for library in profile.libraries {
        let (relative_path, url, sha1) =
            match library.downloads.and_then(|d| d.artifact) {
                Some(artifact) => (
                    artifact.path,
                    artifact.url,
                    artifact.sha1,
                ),

                None => {
                    let repository = library
                        .url
                        .as_deref()
                        .unwrap_or(DEFAULT_MAVEN_URL);

                    let artifact =
                        resolve_maven_artifact(
                            &library.name,
                            repository,
                        )?;

                    (
                        artifact.path,
                        artifact.url,
                        None,
                    )
                }
            };

        let destination =
            libraries_dir.join(&relative_path);

        let valid = match &sha1 {
            Some(expected) => {
                verify_sha1(&destination, expected)?
            }
            None => destination.exists(),
        };

        if valid {
            println!(
                "Library déjà présente : {}",
                library.name
            );

            already_present += 1;
            continue;
        }

        println!(
            "Téléchargement library : {}",
            library.name
        );

        download_file(&url, &destination).await?;

        if let Some(expected_sha1) = &sha1 {
            if !verify_sha1(
                &destination,
                expected_sha1,
            )? {
                let _ = std::fs::remove_file(&destination);

                return Err(format!(
                    "SHA-1 incorrect pour la library : {}",
                    library.name
                ));
            }
        }

        downloaded += 1;
    }

    println!("========================================");
    println!("LIBRARIES TERMINÉES");
    println!("========================================");
    println!("Téléchargées : {}", downloaded);
    println!("Déjà présentes : {}", already_present);

    Ok(format!(
        "{} libraries téléchargées, {} déjà présentes.",
        downloaded,
        already_present
    ))
}