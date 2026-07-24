use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use clap::ValueEnum;
use colored::Colorize;
use ed25519_dalek::{Signer, SigningKey};
use keyring::Entry;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use stellar_strkey::{ed25519::PublicKey, Strkey};
use uuid::Uuid;

const AUTH_SERVICE: &str = "soroban-registry";
const AUTH_DIR_NAME: &str = ".soroban-registry";
const AUTH_FILE_NAME: &str = "auth.json";
const REFRESH_MARGIN_SECS: i64 = 300;
const DEFAULT_STELLAR_EXPIRES_SECS: u64 = 86_400;
const MIN_EXPIRES_SECS: u64 = 300;
const MAX_EXPIRES_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    Github,
    Stellar,
    ApiKey,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Github => write!(f, "github"),
            AuthMethod::Stellar => write!(f, "stellar"),
            AuthMethod::ApiKey => write!(f, "api-key"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    method: AuthMethod,
    identity: String,
    scopes: Vec<String>,
    preferred_expires_seconds: Option<u64>,
    access_token_expires_at: Option<DateTime<Utc>>,
    refreshable: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct LoadedSession {
    record: SessionRecord,
    access_token: String,
    refresh_secret: Option<String>,
    legacy: bool,
}

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    nonce: String,
    #[allow(dead_code)]
    expires_in_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in_seconds: u64,
}

#[derive(Debug, Clone)]
struct IssuedToken {
    token: String,
    expires_at: Option<DateTime<Utc>>,
}

pub async fn login(
    api_url: &str,
    method: AuthMethod,
    identity: Option<&str>,
    secret: Option<&str>,
    scopes: Vec<String>,
    expires: Option<&str>,
) -> Result<()> {
    let identity = match identity {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => prompt_identity(method)?,
    };
    let secret = match secret {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => prompt_secret(method)?,
    };
    let scopes = normalize_scopes(scopes);
    let expires_in_seconds = parse_expires(expires)?.unwrap_or(DEFAULT_STELLAR_EXPIRES_SECS);

    let now = Utc::now();
    let previous_record = load_session_record()?;
    let session_id = Uuid::new_v4().to_string();
    let (access_token, access_expires_at, refresh_secret, refreshable) = match method {
        AuthMethod::Stellar => {
            let issued = issue_stellar_token(
                api_url,
                &identity,
                &secret,
                scopes.clone(),
                expires_in_seconds,
            )
            .await?;
            (issued.token, issued.expires_at, Some(secret.clone()), true)
        }
        AuthMethod::Github | AuthMethod::ApiKey => {
            let expires_at = now + Duration::seconds(expires_in_seconds as i64);
            (secret.clone(), Some(expires_at), None, false)
        }
    };

    store_secret(&session_id, SecretKind::Access, &access_token)?;
    if let Some(refresh_secret) = &refresh_secret {
        store_secret(&session_id, SecretKind::Refresh, refresh_secret)?;
    }

    save_session(&SessionRecord {
        session_id: session_id.clone(),
        method,
        identity: identity.clone(),
        scopes: scopes.clone(),
        preferred_expires_seconds: Some(expires_in_seconds),
        access_token_expires_at: access_expires_at,
        refreshable,
        created_at: now,
        updated_at: now,
    })?;

    if let Some(previous_record) = previous_record {
        clear_session_by_id(&previous_record.session_id)?;
    }

    println!(
        "{} {} ({})",
        "Signed in as".green().bold(),
        identity.bright_blue(),
        method.to_string().bright_black()
    );
    if let Some(expires_at) = access_expires_at {
        println!(
            "{} {}",
            "Token expires".bold(),
            format_expiry(expires_at).bright_black()
        );
    }
    println!(
        "{} {}",
        "Scopes".bold(),
        if scopes.is_empty() {
            "none".bright_black()
        } else {
            scopes.join(", ").bright_black()
        }
    );
    println!(
        "{} {}",
        "Storage".bold(),
        "OS keychain / credential manager".bright_black()
    );
    Ok(())
}

pub fn logout() -> Result<()> {
    clear_active_session()?;
    println!("{}", "Signed out and removed stored credentials.".green());
    Ok(())
}

pub async fn status(api_url: &str) -> Result<()> {
    let session = load_active_session().await?;
    match session {
        Some(session) => print_status(api_url, &session).await,
        None => {
            println!("{}", "Not signed in.".yellow());
            Ok(())
        }
    }
}

pub async fn token(api_url: &str, scopes: Vec<String>, expires: Option<&str>) -> Result<()> {
    let mut session = load_active_session().await?.ok_or_else(|| {
        anyhow::anyhow!("No active auth session. Run `soroban-registry auth login` first.")
    })?;

    let requested_scopes = if scopes.is_empty() {
        session.record.scopes.clone()
    } else {
        normalize_scopes(scopes)
    };
    let expires_in_seconds = parse_expires(expires)?.or(session.record.preferred_expires_seconds);

    let token = match session.record.method {
        AuthMethod::Stellar => {
            let refresh_secret = session.refresh_secret.clone().ok_or_else(|| {
                anyhow::anyhow!("Missing refresh secret. Run `soroban-registry auth login` again.")
            })?;
            let expires_in_seconds = expires_in_seconds.unwrap_or(DEFAULT_STELLAR_EXPIRES_SECS);
            let issued = issue_stellar_token(
                api_url,
                &session.record.identity,
                &refresh_secret,
                requested_scopes.clone(),
                expires_in_seconds,
            )
            .await?;
            session.access_token = issued.token.clone();
            session.record.scopes = requested_scopes;
            session.record.preferred_expires_seconds = Some(expires_in_seconds);
            session.record.access_token_expires_at = issued.expires_at;
            session.record.updated_at = Utc::now();
            store_secret(
                &session.record.session_id,
                SecretKind::Access,
                &session.access_token,
            )?;
            save_session(&session.record)?;
            issued.token
        }
        _ => {
            session.record.scopes = requested_scopes;
            if let Some(expires_in_seconds) = expires_in_seconds {
                session.record.preferred_expires_seconds = Some(expires_in_seconds);
                session.record.access_token_expires_at =
                    Some(Utc::now() + Duration::seconds(expires_in_seconds as i64));
            }
            session.record.updated_at = Utc::now();
            save_session(&session.record)?;
            session.access_token
        }
    };

    println!("{}", token);
    Ok(())
}

pub async fn access_token_for_requests(api_url: &str) -> Result<Option<String>> {
    let Some(mut session) = load_active_session().await? else {
        return Ok(load_legacy_api_key()?.map(|value| value.0));
    };

    if let Some(expires_at) = session.record.access_token_expires_at {
        let refresh_due = expires_at - Utc::now() <= Duration::seconds(REFRESH_MARGIN_SECS);
        if refresh_due {
            if session.record.method == AuthMethod::Stellar {
                refresh_stellar_session(api_url, &mut session).await?;
            } else if Utc::now() >= expires_at {
                anyhow::bail!(
                    "Authentication expired. Run `soroban-registry auth login` to refresh it."
                );
            }
        }
    }

    Ok(Some(session.access_token))
}

async fn refresh_stellar_session(api_url: &str, session: &mut LoadedSession) -> Result<()> {
    let refresh_secret = session.refresh_secret.clone().ok_or_else(|| {
        anyhow::anyhow!("Missing refresh secret. Run `soroban-registry auth login` again.")
    })?;
    let expires_in_seconds = session
        .record
        .preferred_expires_seconds
        .unwrap_or(DEFAULT_STELLAR_EXPIRES_SECS);
    let issued = issue_stellar_token(
        api_url,
        &session.record.identity,
        &refresh_secret,
        session.record.scopes.clone(),
        expires_in_seconds,
    )
    .await?;

    session.access_token = issued.token.clone();
    session.record.access_token_expires_at = issued.expires_at;
    session.record.updated_at = Utc::now();
    store_secret(
        &session.record.session_id,
        SecretKind::Access,
        &session.access_token,
    )?;
    save_session(&session.record)?;
    Ok(())
}

async fn print_status(api_url: &str, session: &LoadedSession) -> Result<()> {
    println!("\n{}", "Authentication Status".bold().cyan());
    println!("{}", "=".repeat(72).cyan());
    println!(
        "{} {}",
        "Method".bold(),
        session.record.method.to_string().bright_blue()
    );
    println!(
        "{} {}",
        "Identity".bold(),
        session.record.identity.bright_black()
    );
    println!(
        "{} {}",
        "Scopes".bold(),
        if session.record.scopes.is_empty() {
            "none".bright_black()
        } else {
            session.record.scopes.join(", ").bright_black()
        }
    );
    println!(
        "{} {}",
        "Storage".bold(),
        if session.legacy {
            "legacy config fallback".yellow()
        } else {
            "OS keychain / credential manager".bright_black()
        }
    );
    println!(
        "{} {}",
        "Refresh".bold(),
        if session.record.refreshable {
            "enabled".green()
        } else {
            "disabled".bright_black()
        }
    );

    if let Some(expires_at) = session.record.access_token_expires_at {
        let state = if Utc::now() >= expires_at {
            "expired".red()
        } else if expires_at - Utc::now() <= Duration::seconds(REFRESH_MARGIN_SECS) {
            "refreshing soon".yellow()
        } else {
            "valid".green()
        };
        println!(
            "{} {} ({})",
            "Token".bold(),
            format_expiry(expires_at),
            state
        );
    } else {
        println!("{} {}", "Token".bold(), "no expiry".bright_black());
    }

    if session.record.refreshable {
        println!("{} {}", "Auto-refresh".bold(), "enabled".green());
    } else {
        println!("{} {}", "Auto-refresh".bold(), "disabled".bright_black());
    }

    println!(
        "{} {}",
        "Token endpoint".bold(),
        format!("{}/api/auth/verify", api_url.trim_end_matches('/')).bright_black()
    );
    println!("{}", "=".repeat(72).cyan());
    Ok(())
}

async fn load_active_session() -> Result<Option<LoadedSession>> {
    if let Some(record) = load_session_record()? {
        let access_token = read_secret(&record.session_id, SecretKind::Access)?;
        let refresh_secret = if record.refreshable {
            read_secret(&record.session_id, SecretKind::Refresh).ok()
        } else {
            None
        };
        return Ok(Some(LoadedSession {
            record,
            access_token,
            refresh_secret,
            legacy: false,
        }));
    }

    if let Some((access_token, scopes)) = load_legacy_api_key()? {
        return Ok(Some(LoadedSession {
            record: SessionRecord {
                session_id: "legacy-api-key".to_string(),
                method: AuthMethod::ApiKey,
                identity: "legacy user config".to_string(),
                scopes,
                preferred_expires_seconds: None,
                access_token_expires_at: None,
                refreshable: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            access_token,
            refresh_secret: None,
            legacy: true,
        }));
    }

    Ok(None)
}

async fn issue_stellar_token(
    api_url: &str,
    address: &str,
    secret_seed: &str,
    scopes: Vec<String>,
    expires_in_seconds: u64,
) -> Result<IssuedToken> {
    let signing_key = parse_signing_key(secret_seed)?;
    let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let expected_address = parse_stellar_address(address)?;
    let actual_address = signing_key.verifying_key().to_bytes();
    if expected_address != actual_address {
        anyhow::bail!("The provided secret seed does not match the Stellar address.");
    }

    let client = crate::net::client();
    let challenge_url = Url::parse_with_params(
        &format!("{}/api/auth/challenge", api_url.trim_end_matches('/')),
        &[("address", address)],
    )?;
    let challenge: ChallengeResponse = client
        .get(challenge_url)
        .send()
        .await
        .context("Failed to request authentication challenge")?
        .error_for_status()
        .context("Authentication challenge failed")?
        .json()
        .await
        .context("Failed to parse authentication challenge")?;

    let signature = signing_key.sign(challenge.nonce.as_bytes());
    let payload = json!({
        "address": address,
        "public_key": public_key_hex,
        "signature": hex::encode(signature.to_bytes()),
        "scopes": scopes,
        "expires_in_seconds": expires_in_seconds,
    });
    let response: VerifyResponse = client
        .post(format!("{}/api/auth/verify", api_url.trim_end_matches('/')))
        .json(&payload)
        .send()
        .await
        .context("Failed to submit authentication proof")?
        .error_for_status()
        .context("Authentication failed")?
        .json()
        .await
        .context("Failed to parse authentication response")?;

    Ok(IssuedToken {
        token: response.token,
        expires_at: Some(Utc::now() + Duration::seconds(response.expires_in_seconds as i64)),
    })
}

fn prompt_identity(method: AuthMethod) -> Result<String> {
    let label = match method {
        AuthMethod::Stellar => "Stellar address",
        AuthMethod::Github => "GitHub username",
        AuthMethod::ApiKey => "API key label",
    };
    let value = crate::wizard::prompt_with_validation(
        label,
        None::<String>,
        |value| match method {
            AuthMethod::Stellar => parse_stellar_address(value).is_ok(),
            _ => !value.trim().is_empty(),
        },
        "Enter a non-empty value.",
    )?;
    Ok(value)
}

fn prompt_secret(method: AuthMethod) -> Result<String> {
    let label = match method {
        AuthMethod::Stellar => "Stellar secret seed",
        AuthMethod::Github => "GitHub token",
        AuthMethod::ApiKey => "API key",
    };
    crate::wizard::prompt_with_validation(
        label,
        None::<String>,
        |value| !value.trim().is_empty(),
        "Enter a non-empty secret.",
    )
}

fn parse_stellar_address(address: &str) -> Result<[u8; 32]> {
    match Strkey::from_string(address.trim()).context("Invalid Stellar address")? {
        Strkey::PublicKeyEd25519(PublicKey(bytes)) => Ok(bytes),
        _ => anyhow::bail!("Expected a Stellar address starting with G"),
    }
}

fn parse_signing_key(secret_seed: &str) -> Result<SigningKey> {
    let private_key =
        match Strkey::from_string(secret_seed.trim()).context("Invalid Stellar secret seed")? {
            Strkey::PrivateKeyEd25519(private_key) => private_key,
            _ => anyhow::bail!("Expected a Stellar secret seed starting with S"),
        };
    Ok(SigningKey::from_bytes(&private_key.0))
}

fn normalize_scopes(scopes: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for scope in scopes {
        let scope = scope.trim().to_ascii_lowercase();
        if scope.is_empty() || !is_valid_scope(&scope) {
            continue;
        }
        if seen.insert(scope.clone()) {
            normalized.push(scope);
        }
    }

    if normalized.is_empty() {
        vec!["read".to_string()]
    } else {
        normalized
    }
}

fn is_valid_scope(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-' | '.'))
}

fn parse_expires(input: Option<&str>) -> Result<Option<u64>> {
    let Some(raw) = input else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let expires = if let Some(number) = raw.strip_suffix('s') {
        number.parse::<u64>()?
    } else if let Some(number) = raw.strip_suffix('m') {
        number.parse::<u64>()? * 60
    } else if let Some(number) = raw.strip_suffix('h') {
        number.parse::<u64>()? * 60 * 60
    } else if let Some(number) = raw.strip_suffix('d') {
        number.parse::<u64>()? * 60 * 60 * 24
    } else {
        raw.parse::<u64>()?
    };

    Ok(Some(expires.clamp(MIN_EXPIRES_SECS, MAX_EXPIRES_SECS)))
}

fn format_expiry(expires_at: DateTime<Utc>) -> String {
    let now = Utc::now();
    if expires_at <= now {
        return "expired".to_string();
    }
    let remaining = expires_at - now;
    let days = remaining.num_days();
    let hours = (remaining - Duration::days(days)).num_hours();
    let minutes = (remaining - Duration::days(days) - Duration::hours(hours)).num_minutes();
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 && days == 0 {
        parts.push(format!("{}m", minutes));
    }
    if parts.is_empty() {
        parts.push("soon".to_string());
    }
    format!("{} (in {})", expires_at.to_rfc3339(), parts.join(" "))
}

fn session_file_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;
    Ok(home.join(AUTH_DIR_NAME).join(AUTH_FILE_NAME))
}

fn load_session_record() -> Result<Option<SessionRecord>> {
    let path = match session_file_path() {
        Ok(path) => path,
        Err(_) => return Ok(None),
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read auth session: {}", path.display()))?;
    let record = serde_json::from_str::<SessionRecord>(&raw)
        .with_context(|| format!("Failed to parse auth session: {}", path.display()))?;
    Ok(Some(record))
}

fn save_session(record: &SessionRecord) -> Result<()> {
    let path = session_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create auth directory: {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(record)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write auth session: {}", path.display()))?;
    Ok(())
}

fn clear_active_session() -> Result<()> {
    let Some(record) = load_session_record()? else {
        return Ok(());
    };
    clear_session_by_id(&record.session_id)?;
    if let Ok(path) = session_file_path() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn clear_session_by_id(session_id: &str) -> Result<()> {
    if let Ok(entry) = secret_entry(session_id, SecretKind::Access) {
        let _ = entry.delete_credential();
    }
    if let Ok(entry) = secret_entry(session_id, SecretKind::Refresh) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

fn store_secret(session_id: &str, kind: SecretKind, secret: &str) -> Result<()> {
    let entry = secret_entry(session_id, kind)?;
    entry
        .set_secret(secret.as_bytes())
        .context("Failed to store secret securely")?;
    Ok(())
}

fn read_secret(session_id: &str, kind: SecretKind) -> Result<String> {
    let entry = secret_entry(session_id, kind)?;
    let secret = entry
        .get_secret()
        .context("Failed to read secret from secure storage")?;
    String::from_utf8(secret).context("Stored secret is not valid UTF-8")
}

fn secret_entry(session_id: &str, kind: SecretKind) -> Result<Entry> {
    let account = format!("{}:{}", kind.as_str(), session_id);
    Entry::new(AUTH_SERVICE, &account).context("Failed to access system credential store")
}

fn load_legacy_api_key() -> Result<Option<(String, Vec<String>)>> {
    let config = crate::user_config::load().context("Failed to read user config")?;
    Ok(config.api_key.map(|key| (key, vec!["read".to_string()])))
}

fn format_scope_list(scopes: &[String]) -> String {
    if scopes.is_empty() {
        "none".to_string()
    } else {
        scopes.join(", ")
    }
}

#[derive(Debug, Clone, Copy)]
enum SecretKind {
    Access,
    Refresh,
}

impl SecretKind {
    fn as_str(&self) -> &'static str {
        match self {
            SecretKind::Access => "access",
            SecretKind::Refresh => "refresh",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_suffixes() {
        assert_eq!(parse_expires(Some("30")).unwrap(), Some(300));
        assert_eq!(parse_expires(Some("10m")).unwrap(), Some(600));
        assert_eq!(parse_expires(Some("2h")).unwrap(), Some(7_200));
        assert_eq!(parse_expires(Some("3d")).unwrap(), Some(259_200));
    }

    #[test]
    fn normalizes_scopes() {
        let scopes = normalize_scopes(vec![
            " Read ".to_string(),
            "deploy".to_string(),
            "deploy".to_string(),
            "".to_string(),
        ]);
        assert_eq!(scopes, vec!["read", "deploy"]);
    }

    #[test]
    fn validates_scope_characters() {
        assert!(is_valid_scope("deploy:contracts"));
        assert!(!is_valid_scope("deploy*"));
    }

    #[test]
    fn formats_scope_list() {
        assert_eq!(format_scope_list(&[]), "none");
        assert_eq!(
            format_scope_list(&["read".into(), "write".into()]),
            "read, write"
        );
    }
}
