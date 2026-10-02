use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::Emitter;

use crate::session::{
    save_accounts_to_disk,
    MinecraftSession,
    SessionState,
};

const CLIENT_ID: &str =
    "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";

const TOKEN_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

const MICROSOFT_REFRESH_GRANT: &str = "refresh_token";

const XBOX_AUTH_URL: &str =
    "https://user.auth.xboxlive.com/user/authenticate";

const XSTS_URL: &str =
    "https://xsts.auth.xboxlive.com/xsts/authorize";

const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";

const MINECRAFT_PROFILE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,

    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MicrosoftToken {
    pub access_token: String,

    #[serde(default)]
    pub refresh_token: Option<String>,

    #[serde(default)]
    pub token_type: Option<String>,

    #[serde(default)]
    pub expires_in: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResult {
    pub microsoft_access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub xbox_user_token: String,
    pub xbox_user_hash: String,
    pub xsts_token: String,
    pub xsts_user_hash: String,
    pub minecraft_access_token: String,
    pub minecraft_uuid: String,
    pub minecraft_name: String,
}

#[derive(Debug, Deserialize)]
struct XboxUserTokenResponse {
    #[serde(rename = "IssueInstant")]
    _issue_instant: String,

    #[serde(rename = "NotAfter")]
    _not_after: String,

    #[serde(rename = "Token")]
    token: String,

    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
    xui: Vec<XboxUserClaim>,
}

#[derive(Debug, Deserialize)]
struct XboxUserClaim {
    uhs: String,
}

#[derive(Debug, Serialize)]
struct XboxUserTokenRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XboxUserTokenProperties<'a>,

    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,

    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
struct XboxUserTokenProperties<'a> {
    #[serde(rename = "AuthMethod")]
    auth_method: &'a str,

    #[serde(rename = "SiteName")]
    site_name: &'a str,

    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Deserialize)]
struct XstsResponse {
    #[serde(rename = "IssueInstant")]
    _issue_instant: String,

    #[serde(rename = "NotAfter")]
    _not_after: String,

    #[serde(rename = "Token")]
    token: String,

    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, Serialize)]
struct XstsRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XstsProperties<'a>,

    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,

    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
struct XstsProperties<'a> {
    #[serde(rename = "SandboxId")]
    sandbox_id: &'a str,

    #[serde(rename = "UserTokens")]
    user_tokens: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
struct MinecraftLoginRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,

    #[serde(rename = "ensureLegacyEnabled")]
    ensure_legacy_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

// ============================================================
// MICROSOFT REFRESH TOKEN
// ============================================================

async fn refresh_microsoft_token(
    client: &Client,
    refresh_token: &str,
) -> Result<MicrosoftToken, String> {
    println!("========================================");
    println!("REFRESH MICROSOFT");
    println!("========================================");
    println!("Renouvellement de la session Microsoft...");

    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", MICROSOFT_REFRESH_GRANT),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| {
            format!(
                "Impossible de contacter Microsoft pour renouveler la session : {}",
                e
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| {
            format!(
                "Impossible de lire la réponse Microsoft : {}",
                e
            )
        })?;

    if !status.is_success() {
        return Err(format!(
            "Microsoft a refusé le refresh ({}): {}",
            status,
            body
        ));
    }

    let token: MicrosoftToken =
        serde_json::from_str(&body)
            .map_err(|e| {
                format!(
                    "Réponse Microsoft invalide lors du refresh : {}",
                    e
                )
            })?;

    println!("Microsoft : refresh réussi.");

    Ok(token)
}

// ============================================================
// MICROSOFT DEVICE CODE
// ============================================================

async fn microsoft_device_login(
    client: &Client,
    app: &tauri::AppHandle,
    state: &SessionState,
) -> Result<MicrosoftToken, String> {
    println!("========================================");
    println!("Connexion Microsoft");
    println!("========================================");

    // Nouvelle connexion : on remet l'annulation à false
    state
        .cancel_login
        .store(false, Ordering::SeqCst);

    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", CLIENT_ID),
            (
                "scope",
                "XboxLive.SignIn XboxLive.offline_access",
            ),
        ])
        .send()
        .await
        .map_err(|e| {
            format!("Erreur réseau Microsoft : {e}")
        })?;

    if !response.status().is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_default();

        return Err(format!(
            "Microsoft a refusé la demande de connexion : {body}"
        ));
    }

    let device: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| {
            format!(
                "Réponse Device Code Microsoft invalide : {e}"
            )
        })?;

    // Envoie le code à l'interface React
    app.emit("microsoft-device-code", &device)
        .map_err(|e| {
            format!(
                "Impossible d'afficher le code Microsoft : {e}"
            )
        })?;

    println!(
        "Code Microsoft : {}",
        device.user_code
    );

    println!(
        "Adresse de connexion : {}",
        device.verification_uri
    );

    println!("========================================");

    open::that(&device.verification_uri)
        .map_err(|e| {
            format!(
                "Impossible d'ouvrir le navigateur : {e}"
            )
        })?;

    let interval_seconds =
        if device.interval == 0 {
            5
        } else {
            device.interval
        };

    let interval =
        Duration::from_secs(interval_seconds);

    let max_attempts = std::cmp::max(
        1,
        device.expires_in / interval_seconds,
    );

    println!(
        "En attente de la connexion Microsoft..."
    );

    let mut microsoft_token: Option<MicrosoftToken> = None;

    for _ in 0..max_attempts {
        // Vérifie si l'utilisateur a demandé l'annulation
        if state
            .cancel_login
            .load(Ordering::SeqCst)
        {
            println!(
                "Connexion Microsoft annulée par l'utilisateur."
            );

            return Err(
                "Connexion Microsoft annulée.".to_string()
            );
        }

        tokio::time::sleep(interval).await;

        // Vérifie également après l'attente
        if state
            .cancel_login
            .load(Ordering::SeqCst)
        {
            println!(
                "Connexion Microsoft annulée par l'utilisateur."
            );

            return Err(
                "Connexion Microsoft annulée.".to_string()
            );
        }

        let response = client
            .post(TOKEN_URL)
            .form(&[
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code",
                ),
                (
                    "client_id",
                    CLIENT_ID,
                ),
                (
                    "device_code",
                    &device.device_code,
                ),
            ])
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Erreur réseau pendant la connexion : {e}"
                )
            })?;

        let status = response.status();

        let body = response
            .text()
            .await
            .map_err(|e| {
                format!(
                    "Erreur lecture réponse Microsoft : {e}"
                )
            })?;

        if status.is_success() {
            let token: MicrosoftToken =
                serde_json::from_str(&body)
                    .map_err(|e| {
                        format!(
                            "Token Microsoft invalide : {e}"
                        )
                    })?;

            microsoft_token = Some(token);
            break;
        }

        if body.contains("authorization_pending") {
            continue;
        }

        if body.contains("slow_down") {
            tokio::time::sleep(
                Duration::from_secs(5),
            )
            .await;

            continue;
        }

        return Err(format!(
            "Erreur authentification Microsoft : {body}"
        ));
    }

    let microsoft_token =
        microsoft_token.ok_or_else(|| {
            "Le code de connexion Microsoft a expiré."
                .to_string()
        })?;

    println!(
        "Microsoft : connexion réussie."
    );

    Ok(microsoft_token)
}

// ============================================================
// MICROSOFT → XBOX LIVE → XSTS → MINECRAFT
// ============================================================

async fn authenticate_minecraft(
    client: &Client,
    microsoft_token: &MicrosoftToken,
) -> Result<
    (
        XboxUserTokenResponse,
        String,
        XstsResponse,
        String,
        MinecraftLoginResponse,
        MinecraftProfile,
    ),
    String,
> {
    // ========================================================
    // 1. MICROSOFT → XBOX LIVE
    // ========================================================

    println!(
        "Xbox Live : authentification..."
    );

    let xbox_request =
        XboxUserTokenRequest {
            properties:
                XboxUserTokenProperties {
                    auth_method: "RPS",

                    site_name:
                        "user.auth.xboxlive.com",

                    rps_ticket: format!(
                        "d={}",
                        microsoft_token.access_token
                    ),
                },

            relying_party:
                "http://auth.xboxlive.com",

            token_type: "JWT",
        };

    let response = client
        .post(XBOX_AUTH_URL)
        .header(
            "Content-Type",
            "application/json",
        )
        .header(
            "Accept",
            "application/json",
        )
        .json(&xbox_request)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Erreur réseau Xbox Live : {e}"
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| {
            format!(
                "Erreur lecture réponse Xbox Live : {e}"
            )
        })?;

    if !status.is_success() {
        return Err(format!(
            "Xbox Live a refusé l'authentification \
             (HTTP {}): {}",
            status,
            body
        ));
    }

    let xbox_response:
        XboxUserTokenResponse =
        serde_json::from_str(&body)
            .map_err(|e| {
                format!(
                    "Réponse Xbox Live invalide : {e}"
                )
            })?;

    let xbox_user_hash =
        xbox_response
            .display_claims
            .xui
            .first()
            .ok_or_else(|| {
                "Xbox Live n'a retourné aucun utilisateur."
                    .to_string()
            })?
            .uhs
            .clone();

    println!(
        "Xbox Live : connexion réussie."
    );

    // ========================================================
    // 2. XBOX LIVE → XSTS
    // ========================================================

    println!(
        "XSTS : authentification..."
    );

    let xsts_request = XstsRequest {
        properties: XstsProperties {
            sandbox_id: "RETAIL",

            user_tokens:
                vec![&xbox_response.token],
        },

        relying_party:
            "rp://api.minecraftservices.com/",

        token_type: "JWT",
    };

    let response = client
        .post(XSTS_URL)
        .header(
            "Content-Type",
            "application/json",
        )
        .header(
            "Accept",
            "application/json",
        )
        .json(&xsts_request)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Erreur réseau XSTS : {e}"
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| {
            format!(
                "Erreur lecture réponse XSTS : {e}"
            )
        })?;

    if !status.is_success() {
        return Err(format!(
            "XSTS a refusé l'authentification \
             (HTTP {}): {}",
            status,
            body
        ));
    }

    let xsts_response:
        XstsResponse =
        serde_json::from_str(&body)
            .map_err(|e| {
                format!(
                    "Réponse XSTS invalide : {e}"
                )
            })?;

    let xsts_user_hash =
        xsts_response
            .display_claims
            .xui
            .first()
            .ok_or_else(|| {
                "XSTS n'a retourné aucun utilisateur."
                    .to_string()
            })?
            .uhs
            .clone();

    println!(
        "XSTS : connexion réussie."
    );

    // ========================================================
    // 3. XSTS → MINECRAFT ACCESS TOKEN
    // ========================================================

    println!(
        "Minecraft Authentication : authentification..."
    );

    let identity_token = format!(
        "XBL3.0 x={};{}",
        xsts_user_hash,
        xsts_response.token
    );

    let minecraft_login_request =
        MinecraftLoginRequest {
            identity_token,

            ensure_legacy_enabled: true,
        };

    let response = client
        .post(MINECRAFT_LOGIN_URL)
        .header(
            "Content-Type",
            "application/json",
        )
        .header(
            "Accept",
            "application/json",
        )
        .json(&minecraft_login_request)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Erreur réseau Minecraft Authentication : {e}"
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| {
            format!(
                "Erreur lecture réponse Minecraft Authentication : {e}"
            )
        })?;

    if !status.is_success() {
        return Err(format!(
            "Minecraft Authentication a refusé la connexion \
             (HTTP {}): {}",
            status,
            body
        ));
    }

    let minecraft_auth:
        MinecraftLoginResponse =
        serde_json::from_str(&body)
            .map_err(|e| {
                format!(
                    "Réponse Minecraft Authentication invalide : {e}"
                )
            })?;

    println!(
        "Minecraft Authentication : connexion réussie."
    );

    // ========================================================
    // 4. MINECRAFT ACCESS TOKEN → PROFILE
    // ========================================================

    println!(
        "Minecraft Services : récupération du profil..."
    );

    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(
            &minecraft_auth.access_token
        )
        .header(
            "Accept",
            "application/json",
        )
        .send()
        .await
        .map_err(|e| {
            format!(
                "Erreur réseau Minecraft Services : {e}"
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| {
            format!(
                "Erreur lecture réponse Minecraft Services : {e}"
            )
        })?;

    if !status.is_success() {
        return Err(format!(
            "Minecraft Services a refusé la connexion \
             (HTTP {}): {}",
            status,
            body
        ));
    }

    let profile:
        MinecraftProfile =
        serde_json::from_str(&body)
            .map_err(|e| {
                format!(
                    "Profil Minecraft invalide : {e}"
                )
            })?;

    println!(
        "Minecraft : connexion réussie."
    );

    println!(
        "Pseudo : {}",
        profile.name
    );

    println!(
        "UUID : {}",
        profile.id
    );

    Ok((
        xbox_response,
        xbox_user_hash,
        xsts_response,
        xsts_user_hash,
        minecraft_auth,
        profile,
    ))
}

// ============================================================
// COMMANDE TAURI : CONNEXION MICROSOFT
// ============================================================

#[tauri::command]
pub async fn microsoft_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, SessionState>,
    force_new_account: bool,
) -> Result<LoginResult, String> {
    let client = Client::new();

    
    // ========================================================
    // 1. TENTATIVE DE REFRESH AUTOMATIQUE
    // ========================================================

    let saved_session = {
        let session = state
            .session
            .lock()
            .map_err(|_| {
                "Impossible d'accéder à la session Eternia."
                    .to_string()
            })?;

        session.clone()
    };

    let microsoft_token: MicrosoftToken;

    if force_new_account {
        println!(
            "Nouvelle connexion Microsoft demandée."
        );

        microsoft_token =
            microsoft_device_login(
                &client,
                &app,
                &state,
            )
            .await?;
    } else if let Some(session) = saved_session {
        if let Some(refresh_token) =
            session.refresh_token.as_deref()
        {
            println!(
                "Session Microsoft existante trouvée."
            );

            println!(
                "Tentative de renouvellement automatique..."
            );

            match refresh_microsoft_token(
                &client,
                refresh_token,
            )
            .await
            {
                Ok(mut token) => {
                    println!(
                        "Refresh Microsoft réussi."
                    );

                    // Microsoft peut ne pas renvoyer de nouveau
                    // refresh_token. Dans ce cas, on conserve
                    // l'ancien.
                    if token.refresh_token.is_none() {
                        token.refresh_token =
                            Some(refresh_token.to_string());
                    }

                    microsoft_token = token;
                }

                Err(error) => {
                    println!(
                        "Refresh Microsoft échoué : {}",
                        error
                    );

                    println!(
                        "Nouvelle connexion Microsoft nécessaire."
                    );

                    microsoft_token =
                        microsoft_device_login(
                            &client,
                            &app,
                            &state,
                        )
                        .await?;
                }
            }
        } else {
            println!(
                "Aucun refresh token disponible."
            );

            println!(
                "Nouvelle connexion Microsoft nécessaire."
            );

            microsoft_token =
                microsoft_device_login(
                    &client,
                    &app,
                    &state,
                )
                .await?;
        }
    } else {
        println!(
            "Aucune session Microsoft sauvegardée."
        );

        println!(
            "Nouvelle connexion Microsoft nécessaire."
        );

        microsoft_token =
            microsoft_device_login(
                &client,
                &app,
                &state,
            )
            .await?;
    }

    // ========================================================
    // 2. MICROSOFT → XBOX → XSTS → MINECRAFT
    // ========================================================

    let (
        xbox_response,
        xbox_user_hash,
        xsts_response,
        xsts_user_hash,
        minecraft_auth,
        profile,
    ) = authenticate_minecraft(
        &client,
        &microsoft_token,
    )
    .await?;

    // ========================================================
    // 3. SAUVEGARDE DE LA SESSION
    // ========================================================

    let session = MinecraftSession {
        minecraft_uuid:
            profile.id.clone(),

        minecraft_name:
            profile.name.clone(),

        minecraft_access_token:
            minecraft_auth.access_token.clone(),

        refresh_token:
            microsoft_token
                .refresh_token
                .clone(),

        expires_in:
            Some(minecraft_auth.expires_in),
    };

   {
    let mut accounts = state
        .accounts
        .lock()
        .map_err(|_| {
            "Impossible d'accéder aux comptes Eternia.".to_string()
        })?;

    if let Some(existing) = accounts
        .iter_mut()
        .find(|account| {
            account.minecraft_uuid == session.minecraft_uuid
        })
    {
        *existing = session.clone();
    } else {
        accounts.push(session.clone());
    }

    save_accounts_to_disk(
        &accounts,
        Some(&session.minecraft_uuid),
    )?;
}

let mut current_session = state
    .session
    .lock()
    .map_err(|_| {
        "Impossible d'accéder à la session Eternia.".to_string()
    })?;

*current_session = Some(session);

    println!(
        "Session Eternia : sauvegardée."
    );

    // ========================================================
    // 4. FIN
    // ========================================================

    println!(
        "========================================"
    );

    println!(
        "AUTHENTIFICATION TERMINÉE"
    );

    println!(
        "========================================"
    );

    println!(
        "Minecraft : {}",
        profile.name
    );

    println!(
        "UUID      : {}",
        profile.id
    );

    println!(
        "Session   : sauvegardée"
    );

    println!(
        "========================================"
    );

    Ok(LoginResult {
        microsoft_access_token:
            microsoft_token
                .access_token,

        refresh_token:
            microsoft_token
                .refresh_token,

        expires_in:
            microsoft_token
                .expires_in,

        xbox_user_token:
            xbox_response.token,

        xbox_user_hash,

        xsts_token:
            xsts_response.token,

        xsts_user_hash,

        minecraft_access_token:
            minecraft_auth
                .access_token,

        minecraft_uuid:
            profile.id,

        minecraft_name:
            profile.name,
    })
}
#[tauri::command]
pub fn cancel_microsoft_login(
    state: tauri::State<'_, SessionState>,
) -> Result<(), String> {
    state
        .cancel_login
        .store(true, Ordering::SeqCst);

    println!(
        "Demande d'annulation de la connexion Microsoft."
    );

    Ok(())
}