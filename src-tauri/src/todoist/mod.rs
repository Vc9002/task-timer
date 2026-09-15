pub mod client;
pub mod sync;

const SERVICE_NAME: &str = "com.vincentc9002.tasktimer";
const TOKEN_KEY: &str = "todoist_api_token";

pub fn get_token() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_token(token: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn clear_token() -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_KEY).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
