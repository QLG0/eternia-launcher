use std::path::PathBuf;

#[tauri::command]
pub fn get_minecraft_dir() -> Result<String, String> {
    let app_data =
        dirs::data_dir().ok_or_else(|| "Impossible de trouver le dossier AppData.".to_string())?;

    let minecraft_dir: PathBuf = app_data.join(".eternia");

    std::fs::create_dir_all(&minecraft_dir)
        .map_err(|e| format!("Impossible de créer le dossier Minecraft : {}", e))?;

    println!("Dossier Minecraft Eternia : {}", minecraft_dir.display());

    Ok(minecraft_dir.to_string_lossy().to_string())
}
