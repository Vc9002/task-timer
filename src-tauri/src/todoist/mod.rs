pub mod client;
pub mod sync;

use std::sync::{Mutex, OnceLock};

const SERVICE_NAME: &str = "com.vincentc9002.tasktimer";
const TOKEN_KEY: &str = "todoist_api_token";

// Keychain reads can trigger an OS authorization prompt. Cache the result for
// the lifetime of this process and invalidate it only when the credential is
// explicitly changed or removed.
type CachedToken = Option<Result<Option<String>, String>>;
static TOKEN_CACHE: OnceLock<Mutex<CachedToken>> = OnceLock::new();

pub fn get_token() -> Result<Option<String>, String> {
    let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(None));
    let mut cached = cache
        .lock()
        .map_err(|_| "Todoist credential cache unavailable".to_string())?;
    if let Some(value) = cached.clone() {
        return value;
    }
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    let result = match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    };
    *cached = Some(result.clone());
    result
}

pub fn set_token(token: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())?;
    *TOKEN_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Todoist credential cache unavailable".to_string())? =
        Some(Ok(Some(token.to_string())));
    Ok(())
}

pub fn clear_token() -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {
            *TOKEN_CACHE
                .get_or_init(|| Mutex::new(None))
                .lock()
                .map_err(|_| "Todoist credential cache unavailable".to_string())? = Some(Ok(None));
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}
