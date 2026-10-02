// ============================================================
// MODULES DU LAUNCHER ETERNIA
// ============================================================

mod auth;
mod session;
mod launcher;
mod minecraft;
mod download;
mod minecraft_manifest;
mod fabric;
mod libraries;
mod java;
mod native;
mod mods;

// ============================================================
// IMPORTS
// ============================================================

use session::{
    load_accounts_from_disk,
    SessionState,
};

// ============================================================
// DÉTECTION DE LA RAM DU PC
// ============================================================

/// Retourne la quantité totale de RAM physique du PC.
///
/// La valeur retournée est exprimée en mégaoctets (Mo).
///
/// Exemples :
/// - PC avec 8 Go  -> environ 8192 Mo
/// - PC avec 16 Go -> environ 16384 Mo
/// - PC avec 32 Go -> environ 32768 Mo
/// - PC avec 64 Go -> environ 65536 Mo
///
/// Cette commande est appelée depuis React/TypeScript
/// afin d'adapter automatiquement les paramètres de RAM
/// dans le launcher.
#[tauri::command]
fn get_system_ram() -> Result<u64, String> {
    use sysinfo::System;

    // --------------------------------------------------------
    // Création du système
    // --------------------------------------------------------

    let mut system = System::new();

    // --------------------------------------------------------
    // Actualisation des informations mémoire
    // --------------------------------------------------------

    system.refresh_memory();

    // --------------------------------------------------------
    // Récupération de la RAM totale
    // --------------------------------------------------------

    // sysinfo retourne la mémoire en octets.
    //
    // Conversion :
    //
    // octets -> Ko -> Mo

    let total_memory_mb =
        system.total_memory() / 1024 / 1024;

    // --------------------------------------------------------
    // Vérification
    // --------------------------------------------------------

    if total_memory_mb == 0 {
        return Err(
            "Impossible de détecter la mémoire RAM du PC."
                .to_string(),
        );
    }

    Ok(total_memory_mb)
}

// ============================================================
// OUVRIR LE DOSSIER MINECRAFT
// ============================================================

/// Ouvre le dossier Minecraft utilisé par Eternia
/// directement dans l'Explorateur Windows.
#[tauri::command]
fn open_minecraft_dir() -> Result<(), String> {
    // --------------------------------------------------------
    // Récupération du dossier Minecraft
    // --------------------------------------------------------

    let minecraft_dir =
        minecraft::get_minecraft_dir()?;

    // --------------------------------------------------------
    // Ouverture de l'Explorateur Windows
    // --------------------------------------------------------

    std::process::Command::new("explorer.exe")
        .arg(&minecraft_dir)
        .spawn()
        .map_err(|error| {
            format!(
                "Impossible d'ouvrir le dossier Minecraft : {}",
                error
            )
        })?;

    Ok(())
}

// ============================================================
// POINT D'ENTRÉE PRINCIPAL DE TAURI
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    // ========================================================
    // CRÉATION DE L'ÉTAT DE SESSION
    // ========================================================

    let session_state =
        SessionState::new();

    // ========================================================
    // INITIALISATION DE L'ANNULATION DE CONNEXION
    // ========================================================

    // Cette variable permet d'annuler proprement
    // une connexion Microsoft en cours depuis le launcher.

    session_state
        .cancel_login
        .store(
            false,
            std::sync::atomic::Ordering::SeqCst,
        );

    // ========================================================
    // CHARGEMENT DES COMPTES SAUVEGARDÉS
    // ========================================================

    match load_accounts_from_disk() {

        // ----------------------------------------------------
        // COMPTES CHARGÉS AVEC SUCCÈS
        // ----------------------------------------------------

        Ok(accounts_file) => {

            println!();
            println!("========================================");
            println!("          COMPTES ETERNIA");
            println!("========================================");

            // ------------------------------------------------
            // AUCUN COMPTE SAUVEGARDÉ
            // ------------------------------------------------

            if accounts_file.accounts.is_empty() {

                println!(
                    "Aucun compte sauvegardé."
                );

                println!(
                    "Connexion Microsoft nécessaire."
                );

            }

            // ------------------------------------------------
            // DES COMPTES SONT DISPONIBLES
            // ------------------------------------------------

            else {

                println!(
                    "{} compte(s) sauvegardé(s).",
                    accounts_file.accounts.len()
                );

                // --------------------------------------------
                // AFFICHAGE DES COMPTES DISPONIBLES
                // --------------------------------------------

                for account in &accounts_file.accounts {

                    println!(
                        "- {} ({})",
                        account.minecraft_name,
                        account.minecraft_uuid
                    );
                }

                // --------------------------------------------
                // RECHERCHE DU COMPTE SÉLECTIONNÉ
                // --------------------------------------------

                // On utilise en priorité le UUID sauvegardé.
                //
                // Si aucun UUID sélectionné n'est disponible,
                // on prend automatiquement le premier compte.

                let selected =
                    accounts_file
                        .selected_uuid
                        .as_deref()
                        .and_then(|uuid| {

                            accounts_file
                                .accounts
                                .iter()
                                .find(|account| {
                                    account.minecraft_uuid
                                        == uuid
                                })
                        })
                        .or_else(|| {
                            accounts_file
                                .accounts
                                .first()
                        });

                // --------------------------------------------
                // CHARGEMENT DU COMPTE SÉLECTIONNÉ
                // --------------------------------------------

                if let Some(account) = selected {

                    println!(
                        "Compte sélectionné : {}",
                        account.minecraft_name
                    );

                    // ----------------------------------------
                    // PLACEMENT DU COMPTE DANS LA SESSION
                    // ----------------------------------------

                    if let Ok(mut session) =
                        session_state.session.lock()
                    {
                        *session =
                            Some(account.clone());
                    }
                }

                // --------------------------------------------
                // CHARGEMENT DE LA LISTE DES COMPTES
                // DANS L'ÉTAT GLOBAL DU LAUNCHER
                // --------------------------------------------

                if let Ok(mut accounts) =
                    session_state.accounts.lock()
                {
                    *accounts =
                        accounts_file.accounts;
                }
            }

            println!(
                "========================================"
            );

            println!();
        }

        // ----------------------------------------------------
        // ERREUR LORS DU CHARGEMENT DES COMPTES
        // ----------------------------------------------------

        Err(error) => {

            println!();
            println!("========================================");
            println!("       ERREUR COMPTES ETERNIA");
            println!("========================================");

            println!(
                "{}",
                error
            );

            println!(
                "========================================"
            );

            println!();
        }
    }

    // ========================================================
    // CONFIGURATION DE TAURI
    // ========================================================

    tauri::Builder::default()

        // ====================================================
        // ÉTAT GLOBAL DU LAUNCHER
        // ====================================================

        // Permet aux différentes commandes Tauri
        // d'accéder à la session et aux comptes.

        .manage(session_state)

        // ====================================================
        // COMMANDES ACCESSIBLES DEPUIS REACT / TYPESCRIPT
        // ====================================================

        .invoke_handler(
            tauri::generate_handler![

                // ============================================
                // SYSTÈME
                // ============================================

                // Détection automatique de la RAM physique.

                get_system_ram,

                // ============================================
                // AUTHENTIFICATION MICROSOFT
                // ============================================

                auth::microsoft_login,
                auth::cancel_microsoft_login,

                // ============================================
                // GESTION DES SESSIONS
                // ============================================

                session::get_session,
                session::get_accounts,
                session::select_account,
                session::remove_account,

                // ============================================
                // LANCEMENT DE MINECRAFT
                // ============================================

                launcher::launch_minecraft,
                launcher::prepare_and_launch,
                launcher::repair_installation,
                launcher::cancel_launch,
                launcher::stop_minecraft,

                // ============================================
                // MINECRAFT
                // ============================================

                minecraft::get_minecraft_dir,
                open_minecraft_dir,

                // ============================================
                // MINECRAFT MANIFEST
                // ============================================

                minecraft_manifest::check_minecraft_version,
                minecraft_manifest::install_minecraft,
                minecraft_manifest::install_minecraft_libraries,
                minecraft_manifest::install_minecraft_assets,

                // ============================================
                // FABRIC
                // ============================================

                fabric::install_fabric,

                // ============================================
                // LIBRAIRIES
                // ============================================

                libraries::install_libraries,

                // ============================================
                // JAVA
                // ============================================

                java::check_java,
                java::get_java_version,
                java::install_java,

                // ============================================
                // NATIVES WINDOWS
                // ============================================

                native::extract_natives,

                // ============================================
                // MODS ETERNIA
                // ============================================

                mods::install_eternia_mods,
            ],
        )

        // ====================================================
        // LANCEMENT DE L'APPLICATION TAURI
        // ====================================================

        .run(
            tauri::generate_context!()
        )

        // ====================================================
        // GESTION D'ERREUR AU DÉMARRAGE
        // ====================================================

        .expect(
            "error while running tauri application"
        );
}