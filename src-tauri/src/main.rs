// ============================================================
// CONFIGURATION WINDOWS
// ============================================================
//
// Empêche Windows d'ouvrir une deuxième fenêtre console
// lorsque le launcher Eternia est lancé en mode Release.
//
// IMPORTANT : ne pas supprimer cette ligne.
//

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// ============================================================
// POINT D'ENTRÉE DE L'APPLICATION
// ============================================================
//
// Le vrai fonctionnement du launcher se trouve dans
// `src-tauri/src/lib.rs`.
//
// `main.rs` sert uniquement à démarrer la bibliothèque
// Tauri du launcher.
//

fn main() {
    // --------------------------------------------------------
    // Démarrage du launcher Eternia
    // --------------------------------------------------------
    //
    // La fonction `run()` est définie dans lib.rs.
    //

    app_lib::run();
}
