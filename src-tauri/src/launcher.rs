// ============================================================
// ETERNIA LAUNCHER
// Lancement et préparation complète de Minecraft
// ============================================================

use serde::Deserialize;

use std::io::{BufRead, BufReader, Read};

use std::path::PathBuf;

use std::process::{Command, Stdio};

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use std::thread;

use tauri::{AppHandle, Emitter, State};

use crate::fabric::install_fabric;

use crate::java::install_java;

use crate::libraries::install_libraries;

use crate::minecraft::get_minecraft_dir;

use crate::minecraft_manifest::{
    install_minecraft,
    install_minecraft_assets,
    install_minecraft_libraries,
};

use crate::mods::install_eternia_mods;

use crate::native::extract_natives;

use crate::session::SessionState;


// ============================================================
// CONFIGURATION MINECRAFT
// ============================================================

/// Version de Minecraft utilisée par Eternia.
const MINECRAFT_VERSION: &str = "1.21.11";

/// Version de Fabric Loader utilisée par Eternia.
const FABRIC_VERSION: &str = "0.19.3";

/// Classe principale utilisée pour lancer Fabric.
const FABRIC_MAIN_CLASS: &str =
    "net.fabricmc.loader.impl.launch.knot.KnotClient";

/// Nombre total d'étapes de préparation.
const PREPARATION_TOTAL_STEPS: u32 = 9;

/// RAM minimum imposée à Minecraft.
///
/// 2048 MB = 2 Go.
///
/// Cette valeur est fixe et ne peut pas être modifiée
/// depuis le launcher.
const MIN_RAM_MB: u64 = 2048;


// ============================================================
// ÉTAT GLOBAL DU PROCESSUS MINECRAFT
// ============================================================

/// PID actuel de Minecraft.
///
/// 0 = aucun processus Minecraft lancé.
static MINECRAFT_PID: AtomicU32 = AtomicU32::new(0);

/// Indique si une préparation doit être annulée.
static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);


// ============================================================
// PROGRESSION DU LAUNCHER
// ============================================================

#[derive(Clone, serde::Serialize)]
struct PreparationProgress {
    step: u32,
    total: u32,
    message: String,
}

/// Envoie la progression au frontend React.
fn emit_progress(
    app: &AppHandle,
    step: u32,
    message: &str,
) {
    let progress = PreparationProgress {
        step,
        total: PREPARATION_TOTAL_STEPS,
        message: message.to_string(),
    };

    if let Err(error) = app.emit(
        "launcher-progress",
        progress,
    ) {
        eprintln!(
            "Impossible d'envoyer la progression : {}",
            error
        );
    }
}


// ============================================================
// CONSOLE MINECRAFT
// ============================================================

#[derive(Clone, serde::Serialize)]
struct ConsoleLine {
    stream: String,
    message: String,
}

#[derive(Clone, serde::Serialize)]
struct MinecraftStarted {
    pid: u32,
}

#[derive(Clone, serde::Serialize)]
struct MinecraftStopped {
    pid: u32,
    exit_code: Option<i32>,
    success: bool,
}


// ============================================================
// TRANSMISSION DE LA CONSOLE
// ============================================================

/// Transmet stdout/stderr de Minecraft vers le frontend.
///
/// Cela permet d'avoir la console Minecraft directement
/// dans l'interface Eternia.
fn forward_output<R: Read + Send + 'static>(
    reader: R,
    app: AppHandle,
    stream: &'static str,
) {
    thread::spawn(move || {
        let reader = BufReader::new(reader);

        for line in reader.lines() {
            match line {
                Ok(message) => {
                    let event = ConsoleLine {
                        stream: stream.to_string(),
                        message,
                    };

                    if let Err(error) =
                        app.emit("launcher-console", event)
                    {
                        eprintln!(
                            "Erreur d'envoi de la console : {}",
                            error
                        );
                    }
                }

                Err(error) => {
                    eprintln!(
                        "Erreur de lecture de la sortie Minecraft : {}",
                        error
                    );

                    break;
                }
            }
        }
    });
}


// ============================================================
// VÉRIFICATION DE L'ANNULATION
// ============================================================

/// Vérifie si l'utilisateur a demandé l'annulation
/// de la préparation de Minecraft.
fn check_cancelled() -> Result<(), String> {
    if CANCEL_REQUESTED.load(Ordering::SeqCst) {
        Err(
            "Préparation de Minecraft annulée."
                .to_string()
        )
    } else {
        Ok(())
    }
}


// ============================================================
// VALIDATION DE LA RAM
// ============================================================

/// Vérifie et normalise la RAM maximum demandée.
///
/// RAM minimum fixe : 2 Go.
///
/// Le maximum doit être supérieur ou égal à 2 Go.
///
/// Le launcher utilise des pas de 512 MB.
fn validate_max_ram(
    max_ram_mb: u64,
) -> Result<u64, String> {

    // --------------------------------------------------------
    // RAM MINIMUM
    // --------------------------------------------------------

    if max_ram_mb < MIN_RAM_MB {
        return Err(
            "La RAM maximum doit être d'au moins 2 Go."
                .to_string()
        );
    }

    // --------------------------------------------------------
    // NORMALISATION SUR 512 MB
    // --------------------------------------------------------

    let normalized_ram =
        (max_ram_mb / 512) * 512;

    if normalized_ram < MIN_RAM_MB {
        return Err(
            "La RAM maximum calculée est inférieure à 2 Go."
                .to_string()
        );
    }

    Ok(normalized_ram)
}


// ============================================================
// STRUCTURES MINIMALES DU JSON MINECRAFT
// ============================================================

#[derive(Debug, Deserialize)]
struct LauncherVersionJson {
    #[serde(rename = "assetIndex")]
    asset_index: LauncherAssetIndex,
}

#[derive(Debug, Deserialize)]
struct LauncherAssetIndex {
    id: String,
}


// ============================================================
// PRÉPARATION AUTOMATIQUE + LANCEMENT
// ============================================================

#[tauri::command]
pub async fn prepare_and_launch(
    app: AppHandle,
    state: State<'_, SessionState>,
    max_ram_mb: u64,
) -> Result<String, String> {

    // --------------------------------------------------------
    // RÉINITIALISATION DE L'ANNULATION
    // --------------------------------------------------------

    CANCEL_REQUESTED.store(
        false,
        Ordering::SeqCst,
    );

    // --------------------------------------------------------
    // VALIDATION DE LA RAM
    // --------------------------------------------------------

    let max_ram_mb =
        validate_max_ram(max_ram_mb)?;

    println!();

    println!("========================================");
    println!("       ETERNIA - CONFIGURATION");
    println!("========================================");

    println!(
        "RAM minimum : {} MB (2 Go)",
        MIN_RAM_MB
    );

    println!(
        "RAM maximum : {} MB ({:.1} Go)",
        max_ram_mb,
        max_ram_mb as f64 / 1024.0
    );

    // --------------------------------------------------------
    // VÉRIFICATION DU PROCESSUS EXISTANT
    // --------------------------------------------------------

    let existing_pid =
        MINECRAFT_PID.load(Ordering::SeqCst);

    if existing_pid != 0 {
        return Err(format!(
            "Minecraft est déjà lancé. PID : {}",
            existing_pid
        ));
    }

    // --------------------------------------------------------
    // EN-TÊTE
    // --------------------------------------------------------

    println!();

    println!("========================================");
    println!("       ETERNIA - PRÉPARATION");
    println!("========================================");


    // ========================================================
    // 1/9 - SESSION
    // ========================================================

    {
        let session = state
            .session
            .lock()
            .map_err(|_| {
                "Impossible d'accéder à la session."
                    .to_string()
            })?;

        if session.is_none() {
            return Err(
                "Aucun compte Minecraft connecté."
                    .to_string()
            );
        }

        if let Some(session) = &*session {
            println!(
                "Compte Minecraft : {}",
                session.minecraft_name
            );

            println!(
                "UUID              : {}",
                session.minecraft_uuid
            );
        }
    }


    // ========================================================
    // 2/9 - JAVA
    // ========================================================

    println!();

    println!("========================================");
    println!("1/9 - JAVA");
    println!("========================================");

    emit_progress(
        &app,
        1,
        "Installation et vérification de Java 21...",
    );

    install_java().await?;

    check_cancelled()?;

    println!("Java prêt.");


    // ========================================================
    // 3/9 - MINECRAFT
    // ========================================================

    println!();

    println!("========================================");

    println!(
        "2/9 - MINECRAFT {}",
        MINECRAFT_VERSION
    );

    println!("========================================");

    emit_progress(
        &app,
        2,
        "Installation et vérification de Minecraft 1.21.11...",
    );

    install_minecraft(
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Minecraft prêt.");


    // ========================================================
    // 4/9 - LIBRARIES MINECRAFT
    // ========================================================

    println!();

    println!("========================================");
    println!("3/9 - LIBRARIES MINECRAFT");
    println!("========================================");

    emit_progress(
        &app,
        3,
        "Installation et vérification des libraries Minecraft...",
    );

    install_minecraft_libraries(
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Libraries Minecraft prêtes.");


    // ========================================================
    // 5/9 - ASSETS
    // ========================================================

    println!();

    println!("========================================");
    println!("4/9 - ASSETS");
    println!("========================================");

    emit_progress(
        &app,
        4,
        "Installation et vérification des assets...",
    );

    install_minecraft_assets(
        app.clone(),
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Assets prêts.");


    // ========================================================
    // 6/9 - FABRIC
    // ========================================================

    println!();

    println!("========================================");

    println!(
        "5/9 - FABRIC {}",
        FABRIC_VERSION
    );

    println!("========================================");

    emit_progress(
        &app,
        5,
        "Installation et vérification de Fabric 0.19.3...",
    );

    install_fabric().await?;

    check_cancelled()?;

    println!("Fabric prêt.");


    // ========================================================
    // 7/9 - LIBRARIES FABRIC
    // ========================================================

    println!();

    println!("========================================");
    println!("6/9 - LIBRARIES FABRIC");
    println!("========================================");

    emit_progress(
        &app,
        6,
        "Installation et vérification des libraries Fabric...",
    );

    install_libraries().await?;

    check_cancelled()?;

    println!("Libraries Fabric prêtes.");


    // ========================================================
    // 8/9 - MODS ETERNIA
    // ========================================================

    println!();

    println!("========================================");
    println!("7/9 - MODS ETERNIA");
    println!("========================================");

    emit_progress(
        &app,
        7,
        "Installation et vérification des mods Eternia...",
    );

    install_eternia_mods(
        app.clone(),
    )
    .await?;

    check_cancelled()?;

    println!("Mods Eternia prêts.");


    // ========================================================
    // 9/9 - NATIVES WINDOWS
    // ========================================================

    println!();

    println!("========================================");
    println!("8/9 - NATIVES WINDOWS");
    println!("========================================");

    emit_progress(
        &app,
        8,
        "Extraction des natives Windows...",
    );

    extract_natives()?;

    check_cancelled()?;

    println!("Natives prêtes.");


    // ========================================================
    // LANCEMENT DE MINECRAFT
    // ========================================================

    println!();

    println!("========================================");
    println!("9/9 - LANCEMENT MINECRAFT");
    println!("========================================");

    emit_progress(
        &app,
        9,
        "Lancement de Minecraft...",
    );

    let result =
        launch_minecraft_internal(
            state.inner(),
            app.clone(),
            max_ram_mb,
        )
        .await?;

    // --------------------------------------------------------
    // FIN
    // --------------------------------------------------------

    println!();

    println!("========================================");
    println!("       ETERNIA - TERMINÉ");
    println!("========================================");

    println!("{}", result);

    Ok(result)
}


// ============================================================
// RÉPARER L'INSTALLATION COMPLÈTE
// ============================================================

/// Vérifie et réinstalle tous les composants nécessaires
/// au fonctionnement de Minecraft Eternia.
///
/// Cette commande NE lance PAS Minecraft.
///
/// Elle effectue uniquement la réparation de l'installation.
#[tauri::command]
pub async fn repair_installation(
    app: AppHandle,
) -> Result<String, String> {

    // --------------------------------------------------------
    // RÉINITIALISATION DE L'ANNULATION
    // --------------------------------------------------------

    CANCEL_REQUESTED.store(
        false,
        Ordering::SeqCst,
    );

    check_cancelled()?;

    // --------------------------------------------------------
    // VÉRIFICATION D'UN MINECRAFT DÉJÀ LANCÉ
    // --------------------------------------------------------

    let existing_pid =
        MINECRAFT_PID.load(Ordering::SeqCst);

    if existing_pid != 0 {
        return Err(format!(
            "Impossible de réparer l'installation pendant que Minecraft est lancé. PID : {}",
            existing_pid
        ));
    }

    // --------------------------------------------------------
    // EN-TÊTE
    // --------------------------------------------------------

    println!();

    println!("========================================");
    println!("ETERNIA - RÉPARATION DE L'INSTALLATION");
    println!("========================================");

    println!(
        "Minecraft : {}",
        MINECRAFT_VERSION
    );

    println!(
        "Fabric    : {}",
        FABRIC_VERSION
    );


    // ========================================================
    // 1/9 - JAVA
    // ========================================================

    println!();

    println!("========================================");
    println!("1/9 - JAVA");
    println!("========================================");

    emit_progress(
        &app,
        1,
        "Vérification et réparation de Java 21...",
    );

    install_java().await?;

    check_cancelled()?;

    println!("Java prêt.");


    // ========================================================
    // 2/9 - MINECRAFT
    // ========================================================

    println!();

    println!("========================================");

    println!(
        "2/9 - MINECRAFT {}",
        MINECRAFT_VERSION
    );

    println!("========================================");

    emit_progress(
        &app,
        2,
        "Vérification et réparation de Minecraft 1.21.11...",
    );

    install_minecraft(
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Minecraft prêt.");


    // ========================================================
    // 3/9 - LIBRARIES MINECRAFT
    // ========================================================

    println!();

    println!("========================================");
    println!("3/9 - LIBRARIES MINECRAFT");
    println!("========================================");

    emit_progress(
        &app,
        3,
        "Vérification et réparation des libraries Minecraft...",
    );

    install_minecraft_libraries(
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Libraries Minecraft prêtes.");


    // ========================================================
    // 4/9 - ASSETS
    // ========================================================

    println!();

    println!("========================================");
    println!("4/9 - ASSETS");
    println!("========================================");

    emit_progress(
        &app,
        4,
        "Vérification et réparation des assets Minecraft...",
    );

    install_minecraft_assets(
        app.clone(),
        MINECRAFT_VERSION.to_string(),
    )
    .await?;

    check_cancelled()?;

    println!("Assets prêts.");


    // ========================================================
    // 5/9 - FABRIC
    // ========================================================

    println!();

    println!("========================================");

    println!(
        "5/9 - FABRIC {}",
        FABRIC_VERSION
    );

    println!("========================================");

    emit_progress(
        &app,
        5,
        "Vérification et réparation de Fabric 0.19.3...",
    );

    install_fabric().await?;

    check_cancelled()?;

    println!("Fabric prêt.");


    // ========================================================
    // 6/9 - LIBRARIES FABRIC
    // ========================================================

    println!();

    println!("========================================");
    println!("6/9 - LIBRARIES FABRIC");
    println!("========================================");

    emit_progress(
        &app,
        6,
        "Vérification et réparation des libraries Fabric...",
    );

    install_libraries().await?;

    check_cancelled()?;

    println!("Libraries Fabric prêtes.");


    // ========================================================
    // 7/9 - MODS ETERNIA
    // ========================================================

    println!();

    println!("========================================");
    println!("7/9 - MODS ETERNIA");
    println!("========================================");

    emit_progress(
        &app,
        7,
        "Vérification et réparation des mods Eternia...",
    );

    install_eternia_mods(
        app.clone(),
    )
    .await?;

    check_cancelled()?;

    println!("Mods Eternia prêts.");


    // ========================================================
    // 8/9 - NATIVES WINDOWS
    // ========================================================

    println!();

    println!("========================================");
    println!("8/9 - NATIVES WINDOWS");
    println!("========================================");

    emit_progress(
        &app,
        8,
        "Vérification et réparation des natives Windows...",
    );

    extract_natives()?;

    check_cancelled()?;

    println!("Natives Windows prêtes.");


    // ========================================================
    // 9/9 - FIN DE LA RÉPARATION
    // ========================================================

    println!();

    println!("========================================");
    println!("9/9 - RÉPARATION TERMINÉE");
    println!("========================================");

    emit_progress(
        &app,
        9,
        "Installation réparée avec succès.",
    );

    println!();

    println!("========================================");
    println!("ETERNIA - INSTALLATION RÉPARÉE");
    println!("========================================");

    println!(
        "Minecraft {} est prêt.",
        MINECRAFT_VERSION
    );

    println!(
        "Fabric {} est prêt.",
        FABRIC_VERSION
    );

    println!("Java est prêt.");
    println!("Libraries Minecraft vérifiées.");
    println!("Assets vérifiés.");
    println!("Libraries Fabric vérifiées.");
    println!("Mods Eternia vérifiés.");
    println!("Natives Windows vérifiées.");

    println!("========================================");

    Ok(
        "L'installation Minecraft d'Eternia a été réparée avec succès."
            .to_string()
    )
}


// ============================================================
// COMMANDE PUBLIQUE DE LANCEMENT
// ============================================================

/// Lance directement Minecraft.
///
/// Cette commande est également capable de recevoir
/// la RAM maximum définie dans les paramètres du launcher.
#[tauri::command]
pub async fn launch_minecraft(
    app: AppHandle,
    state: State<'_, SessionState>,
    max_ram_mb: u64,
) -> Result<String, String> {

    launch_minecraft_internal(
        state.inner(),
        app,
        max_ram_mb,
    )
    .await
}


// ============================================================
// LANCEMENT MINECRAFT INTERNE
// ============================================================

async fn launch_minecraft_internal(
    state: &SessionState,
    app: AppHandle,
    max_ram_mb: u64,
) -> Result<String, String> {

    println!("========================================");
    println!("PRÉPARATION DU LANCEMENT MINECRAFT");
    println!("========================================");


    // ========================================================
    // CONFIGURATION RAM JAVA
    // ========================================================

    let min_ram_arg = "-Xms2G";

    let max_ram_mb =
        validate_max_ram(max_ram_mb)?;

    let max_ram_arg =
        format!("-Xmx{}M", max_ram_mb);


    // --------------------------------------------------------
    // AFFICHAGE RAM
    // --------------------------------------------------------

    println!(
        "RAM minimum JVM : 2G"
    );

    println!(
        "RAM maximum JVM : {} MB ({:.1} Go)",
        max_ram_mb,
        max_ram_mb as f64 / 1024.0
    );


    // ========================================================
    // SESSION
    // ========================================================

    let session = state
        .session
        .lock()
        .map_err(|_| {
            "Impossible d'accéder à la session."
                .to_string()
        })?
        .clone();

    let session = match session {
        Some(session) => session,

        None => {
            return Err(
                "Aucun compte Minecraft connecté."
                    .to_string()
            );
        }
    };

    println!(
        "Joueur : {}",
        session.minecraft_name
    );

    println!(
        "UUID   : {}",
        session.minecraft_uuid
    );


    // ========================================================
    // DOSSIERS
    // ========================================================

    let minecraft_dir =
        PathBuf::from(get_minecraft_dir()?);

    let java_path = minecraft_dir
        .join("runtime")
        .join("java")
        .join("bin")
        .join("java.exe");

    let natives_dir =
        minecraft_dir.join("natives");

    let libraries_dir =
        minecraft_dir.join("libraries");

    let assets_dir =
        minecraft_dir.join("assets");

    let version_dir = minecraft_dir
        .join("versions")
        .join(MINECRAFT_VERSION);

    let client_jar = version_dir.join(
        format!(
            "{}.jar",
            MINECRAFT_VERSION
        )
    );

    let version_json_path = version_dir.join(
        format!(
            "{}.json",
            MINECRAFT_VERSION
        )
    );


    // ========================================================
    // AFFICHAGE DES CHEMINS
    // ========================================================

    println!(
        "Java      : {}",
        java_path.display()
    );

    println!(
        "Natives   : {}",
        natives_dir.display()
    );

    println!(
        "Libraries : {}",
        libraries_dir.display()
    );

    println!(
        "Assets    : {}",
        assets_dir.display()
    );

    println!(
        "Client    : {}",
        client_jar.display()
    );

    println!(
        "JSON      : {}",
        version_json_path.display()
    );


    // ========================================================
    // VÉRIFICATIONS DES FICHIERS
    // ========================================================

    if !java_path.exists() {
        return Err(format!(
            "Java introuvable : {}",
            java_path.display()
        ));
    }

    if !natives_dir.exists() {
        return Err(format!(
            "Natives introuvables : {}",
            natives_dir.display()
        ));
    }

    if !libraries_dir.exists() {
        return Err(format!(
            "Libraries introuvables : {}",
            libraries_dir.display()
        ));
    }

    if !assets_dir.exists() {
        return Err(format!(
            "Assets introuvables : {}",
            assets_dir.display()
        ));
    }

    if !client_jar.exists() {
        return Err(format!(
            "Minecraft client introuvable : {}",
            client_jar.display()
        ));
    }

    if !version_json_path.exists() {
        return Err(format!(
            "JSON Minecraft introuvable : {}",
            version_json_path.display()
        ));
    }


    // ========================================================
    // LECTURE DU JSON MINECRAFT
    // ========================================================

    println!("========================================");
    println!("LECTURE DU JSON MINECRAFT");
    println!("========================================");

    let version_json_data =
        std::fs::read_to_string(
            &version_json_path
        )
        .map_err(|e| {
            format!(
                "Impossible de lire le JSON Minecraft : {}",
                e
            )
        })?;

    let version_json: LauncherVersionJson =
        serde_json::from_str(
            &version_json_data
        )
        .map_err(|e| {
            format!(
                "Impossible de lire la structure du JSON Minecraft : {}",
                e
            )
        })?;

    let asset_index_id =
        version_json.asset_index.id;

    println!(
        "Asset Index : {}",
        asset_index_id
    );


    // ========================================================
    // CONSTRUCTION DU CLASSPATH
    // ========================================================

    println!("========================================");
    println!("CONSTRUCTION DU CLASSPATH");
    println!("========================================");

    let mut classpath: Vec<String> =
        Vec::new();

    collect_jars(
        &libraries_dir,
        &mut classpath,
    )?;


    // --------------------------------------------------------
    // CLIENT MINECRAFT
    // --------------------------------------------------------

    classpath.push(
        client_jar
            .to_string_lossy()
            .to_string()
    );

    println!(
        "JAR dans le classpath : {}",
        classpath.len()
    );

    let classpath_string =
        classpath.join(";");


    // ========================================================
    // INFORMATIONS DE LANCEMENT
    // ========================================================

    println!("========================================");
    println!("COMMANDE MINECRAFT");
    println!("========================================");

    println!(
        "Java executable : {}",
        java_path.display()
    );

    println!(
        "Main class      : {}",
        FABRIC_MAIN_CLASS
    );

    println!(
        "Minecraft       : {}",
        MINECRAFT_VERSION
    );

    println!(
        "Fabric          : {}",
        FABRIC_VERSION
    );

    println!(
        "RAM minimum     : 2048 MB (2 Go)"
    );

    println!(
        "RAM maximum     : {} MB ({:.1} Go)",
        max_ram_mb,
        max_ram_mb as f64 / 1024.0
    );

    println!(
        "Classpath       : {} JAR",
        classpath.len()
    );

    println!(
        "Natives         : {}",
        natives_dir.display()
    );

    println!(
        "Asset Index     : {}",
        asset_index_id
    );


    // ========================================================
    // CONSTRUCTION DU PROCESS JAVA
    // ========================================================

    let mut command =
        Command::new(&java_path);

    command
        .current_dir(&minecraft_dir)

        // ----------------------------------------------------
        // JVM / RAM
        // ----------------------------------------------------

        .arg(min_ram_arg)
        .arg(&max_ram_arg)

        // ----------------------------------------------------
        // NATIVES
        // ----------------------------------------------------

        .arg(format!(
            "-Djava.library.path={}",
            natives_dir.display()
        ))

        // ----------------------------------------------------
        // CLASSPATH
        // ----------------------------------------------------

        .arg("-cp")
        .arg(&classpath_string)

        // ----------------------------------------------------
        // FABRIC
        // ----------------------------------------------------

        .arg(FABRIC_MAIN_CLASS)

        // ----------------------------------------------------
        // COMPTE MINECRAFT
        // ----------------------------------------------------

        .arg("--username")
        .arg(&session.minecraft_name)

        .arg("--uuid")
        .arg(&session.minecraft_uuid)

        .arg("--accessToken")
        .arg(&session.minecraft_access_token)

        .arg("--userType")
        .arg("msa")

        // ----------------------------------------------------
        // VERSION
        // ----------------------------------------------------

        .arg("--version")
        .arg("1.21.11-fabric")

        .arg("--versionType")
        .arg("release")

        // ----------------------------------------------------
        // DOSSIERS
        // ----------------------------------------------------

        .arg("--gameDir")
        .arg(&minecraft_dir)

        .arg("--assetsDir")
        .arg(&assets_dir)

        // ----------------------------------------------------
        // ASSET INDEX
        // ----------------------------------------------------

        .arg("--assetIndex")
        .arg(&asset_index_id);


    // ========================================================
    // AFFICHAGE DES ARGUMENTS
    // ========================================================

    println!("----------------------------------------");
    println!("Arguments JVM :");

    println!(
        "  {}",
        min_ram_arg
    );

    println!(
        "  {}",
        max_ram_arg
    );

    println!("----------------------------------------");

    println!("Arguments Minecraft :");

    println!(
        "  --username {}",
        session.minecraft_name
    );

    println!(
        "  --uuid {}",
        session.minecraft_uuid
    );


    // --------------------------------------------------------
    // SÉCURITÉ
    // --------------------------------------------------------

    // Ne jamais afficher le vrai access token.

    println!(
        "  --accessToken ********"
    );

    println!(
        "  --gameDir {}",
        minecraft_dir.display()
    );

    println!(
        "  --assetsDir {}",
        assets_dir.display()
    );

    println!(
        "  --assetIndex {}",
        asset_index_id
    );

    println!("----------------------------------------");


    // ========================================================
    // LANCEMENT DE MINECRAFT
    // ========================================================

    println!(
        "Démarrage de Minecraft..."
    );

    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child =
        command
            .spawn()
            .map_err(|error| {
                format!(
                    "Impossible de lancer Minecraft : {}",
                    error
                )
            })?;

    let pid = child.id();


    // --------------------------------------------------------
    // SAUVEGARDE DU PID GLOBAL
    // --------------------------------------------------------

    MINECRAFT_PID.store(
        pid,
        Ordering::SeqCst,
    );


    // ========================================================
    // NOTIFICATION FRONTEND
    // ========================================================

    if let Err(error) = app.emit(
        "minecraft-started",
        MinecraftStarted { pid },
    ) {
        eprintln!(
            "Impossible d'envoyer minecraft-started : {}",
            error
        );
    }


    // ========================================================
    // SORTIE STANDARD
    // ========================================================

    if let Some(stdout) =
        child.stdout.take()
    {
        forward_output(
            stdout,
            app.clone(),
            "stdout",
        );
    }


    // ========================================================
    // SORTIE ERREUR
    // ========================================================

    if let Some(stderr) =
        child.stderr.take()
    {
        forward_output(
            stderr,
            app.clone(),
            "stderr",
        );
    }


    println!(
        "Minecraft lancé. PID : {}",
        pid
    );


    // ========================================================
    // SURVEILLANCE DU PROCESSUS
    // ========================================================

    let wait_app =
        app.clone();

    thread::spawn(move || {

        match child.wait() {

            // ------------------------------------------------
            // MINECRAFT S'EST ARRÊTÉ
            // ------------------------------------------------

            Ok(status) => {

                let exit_code =
                    status.code();

                let success =
                    status.success();


                // ------------------------------------------------
                // LE PID REPASSE À 0
                // ------------------------------------------------

                MINECRAFT_PID.store(
                    0,
                    Ordering::SeqCst,
                );


                // ------------------------------------------------
                // NOTIFICATION FRONTEND
                // ------------------------------------------------

                if let Err(error) =
                    wait_app.emit(
                        "minecraft-stopped",
                        MinecraftStopped {
                            pid,
                            exit_code,
                            success,
                        },
                    )
                {
                    eprintln!(
                        "Impossible d'envoyer minecraft-stopped : {}",
                        error
                    );
                }


                println!(
                    "Minecraft arrêté. PID : {} | code : {:?}",
                    pid,
                    exit_code
                );
            }


            // ------------------------------------------------
            // ERREUR DE SURVEILLANCE
            // ------------------------------------------------

            Err(error) => {

                MINECRAFT_PID.store(
                    0,
                    Ordering::SeqCst,
                );

                eprintln!(
                    "Erreur lors de l'attente de Minecraft : {}",
                    error
                );

                let _ =
                    wait_app.emit(
                        "minecraft-stopped",
                        MinecraftStopped {
                            pid,
                            exit_code: None,
                            success: false,
                        },
                    );
            }
        }
    });


    // ========================================================
    // FIN DU LANCEMENT
    // ========================================================

    println!("========================================");
    println!("MINECRAFT LANCÉ");
    println!("========================================");

    Ok(format!(
        "Minecraft {} lancé pour {} avec {} MB de RAM maximum.",
        MINECRAFT_VERSION,
        session.minecraft_name,
        max_ram_mb
    ))
}


// ============================================================
// ANNULER LA PRÉPARATION
// ============================================================

#[tauri::command]
pub fn cancel_launch() -> Result<String, String> {

    CANCEL_REQUESTED.store(
        true,
        Ordering::SeqCst,
    );

    println!(
        "Annulation de la préparation demandée."
    );

    Ok(
        "Annulation demandée.".to_string()
    )
}


// ============================================================
// ARRÊTER MINECRAFT
// ============================================================

#[tauri::command]
pub fn stop_minecraft() -> Result<String, String> {

    let pid =
        MINECRAFT_PID.load(
            Ordering::SeqCst
        );

    if pid == 0 {
        return Err(
            "Minecraft n'est pas actuellement lancé."
                .to_string()
        );
    }

    println!(
        "Arrêt de Minecraft demandé. PID : {}",
        pid
    );


    // ========================================================
    // WINDOWS
    // ========================================================

    #[cfg(target_os = "windows")]
    {
        let status =
            Command::new("taskkill")
                .args([
                    "/PID",
                    &pid.to_string(),
                    "/T",
                    "/F",
                ])
                .status()
                .map_err(|error| {
                    format!(
                        "Impossible d'arrêter Minecraft : {}",
                        error
                    )
                })?;

        if !status.success() {
            return Err(format!(
                "taskkill a échoué pour le PID {}.",
                pid
            ));
        }
    }


    // ========================================================
    // LINUX / MACOS
    // ========================================================

    #[cfg(not(target_os = "windows"))]
    {
        let status =
            Command::new("kill")
                .arg(pid.to_string())
                .status()
                .map_err(|error| {
                    format!(
                        "Impossible d'arrêter le processus {}.",
                        error
                    )
                })?;

        if !status.success() {
            return Err(format!(
                "Impossible d'arrêter le processus {}.",
                pid
            ));
        }
    }


    Ok(format!(
        "Arrêt de Minecraft demandé. PID : {}",
        pid
    ))
}


// ============================================================
// RECHERCHE RÉCURSIVE DES JAR
// ============================================================

/// Recherche tous les fichiers JAR dans le dossier libraries.
///
/// Les JAR contenant "-natives-" sont volontairement exclus
/// car ils sont déjà extraits dans le dossier natives.
fn collect_jars(
    directory: &PathBuf,
    result: &mut Vec<String>,
) -> Result<(), String> {

    let entries =
        std::fs::read_dir(directory)
            .map_err(|e| {
                format!(
                    "Impossible de lire {} : {}",
                    directory.display(),
                    e
                )
            })?;

    for entry in entries {

        let entry =
            entry.map_err(|e| {
                format!(
                    "Erreur lecture bibliothèque : {}",
                    e
                )
            })?;

        let path =
            entry.path();


        // ====================================================
        // SOUS-DOSSIER
        // ====================================================

        if path.is_dir() {

            collect_jars(
                &path,
                result,
            )?;

            continue;
        }


        // ====================================================
        // FICHIER JAR
        // ====================================================

        if !path.is_file() {
            continue;
        }

        let is_jar =
            path.extension()
                .and_then(|ext| ext.to_str())
                == Some("jar");

        if !is_jar {
            continue;
        }


        // ====================================================
        // NOM DU FICHIER
        // ====================================================

        let filename =
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");


        // ====================================================
        // EXCLUSION DES NATIVES
        // ====================================================

        if filename.contains("-natives-") {

            println!(
                "Native exclue du classpath : {}",
                filename
            );

            continue;
        }


        // ====================================================
        // AJOUT AU CLASSPATH
        // ====================================================

        result.push(
            path.to_string_lossy()
                .to_string()
        );
    }

    Ok(())
}