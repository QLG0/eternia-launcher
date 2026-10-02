use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::minecraft::get_minecraft_dir;

fn is_native_jar(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    name.contains("natives-windows")
}

fn extract_native_jar(
    jar_path: &Path,
    natives_dir: &Path,
) -> Result<usize, String> {
    println!(
        "Extraction native : {}",
        jar_path.display()
    );

    let file = fs::File::open(jar_path).map_err(|e| {
        format!(
            "Impossible d'ouvrir le JAR natif {} : {}",
            jar_path.display(),
            e
        )
    })?;

    let mut archive = ZipArchive::new(file).map_err(|e| {
        format!(
            "Impossible de lire le JAR natif {} : {}",
            jar_path.display(),
            e
        )
    })?;

    let mut extracted = 0usize;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| {
            format!(
                "Impossible de lire l'entrée {} dans {} : {}",
                index,
                jar_path.display(),
                e
            )
        })?;

        let entry_name = entry.name().replace('\\', "/");

        // On ne veut que les DLL Windows.
        if !entry_name.to_lowercase().ends_with(".dll") {
            continue;
        }

        let file_name = Path::new(&entry_name)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                format!(
                    "Nom de DLL invalide dans {}",
                    jar_path.display()
                )
            })?;

        // Protection contre les chemins ../ ou absolus.
        if file_name.contains("..")
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err(format!(
                "Chemin natif dangereux détecté : {}",
                entry_name
            ));
        }

        let destination = natives_dir.join(file_name);

        let mut data = Vec::new();

        entry.read_to_end(&mut data).map_err(|e| {
            format!(
                "Impossible de lire {} : {}",
                entry_name, e
            )
        })?;

        fs::write(&destination, &data).map_err(|e| {
            format!(
                "Impossible d'écrire {} : {}",
                destination.display(),
                e
            )
        })?;

        println!(
            "  DLL extraite : {}",
            destination.display()
        );

        extracted += 1;
    }

    Ok(extracted)
}

fn find_native_jars(
    libraries_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();

    fn scan_dir(
        directory: &Path,
        result: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(directory).map_err(|e| {
            format!(
                "Impossible de lire {} : {}",
                directory.display(),
                e
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                format!(
                    "Erreur lecture dossier {} : {}",
                    directory.display(),
                    e
                )
            })?;

            let path = entry.path();

            if path.is_dir() {
                scan_dir(&path, result)?;
                continue;
            }

            if path.is_file() && is_native_jar(&path) {
                result.push(path);
            }
        }

        Ok(())
    }

    scan_dir(libraries_dir, &mut result)?;

    Ok(result)
}

#[tauri::command]
pub fn extract_natives() -> Result<String, String> {
    println!("========================================");
    println!("EXTRACTION DES NATIVES WINDOWS");
    println!("========================================");

    let minecraft_dir = PathBuf::from(get_minecraft_dir()?);

    let libraries_dir = minecraft_dir.join("libraries");
    let natives_dir = minecraft_dir.join("natives");

    if !libraries_dir.exists() {
        return Err(format!(
            "Dossier libraries introuvable : {}",
            libraries_dir.display()
        ));
    }

    fs::create_dir_all(&natives_dir).map_err(|e| {
        format!(
            "Impossible de créer le dossier natives : {}",
            e
        )
    })?;

    println!(
        "Libraries : {}",
        libraries_dir.display()
    );

    println!(
        "Natives   : {}",
        natives_dir.display()
    );

    let native_jars = find_native_jars(&libraries_dir)?;

    println!(
        "JAR natives Windows trouvés : {}",
        native_jars.len()
    );

    if native_jars.is_empty() {
        return Err(
            "Aucun JAR natives-windows trouvé.".to_string()
        );
    }

    let mut total_dll = 0usize;

    for jar in native_jars {
        total_dll += extract_native_jar(
            &jar,
            &natives_dir,
        )?;
    }

    println!("----------------------------------------");
    println!(
        "DLL extraites : {}",
        total_dll
    );
    println!("========================================");
    println!("NATIVES WINDOWS PRÊTES");
    println!("========================================");

    Ok(natives_dir.to_string_lossy().to_string())
}