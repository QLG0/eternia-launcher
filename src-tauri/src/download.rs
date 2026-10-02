use sha1::{Digest, Sha1};
use std::path::Path;

pub async fn download_file(url: &str, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Impossible de créer le dossier : {}", e))?;
    }

    println!("Téléchargement : {}", url);

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Erreur téléchargement : {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Erreur HTTP {} pour {}", response.status(), url));
    }

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Impossible de lire le fichier : {}", e))?;

    std::fs::write(destination, &data)
        .map_err(|e| format!("Impossible d'écrire le fichier : {}", e))?;

    println!("Téléchargement terminé : {}", destination.display());

    Ok(())
}

pub fn sha1_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Impossible de lire le fichier : {}", e))?;

    let mut hasher = Sha1::new();
    hasher.update(&data);

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_sha1(path: &Path, expected: &str) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }

    let actual = sha1_file(path)?;

    Ok(actual.eq_ignore_ascii_case(expected))
}
