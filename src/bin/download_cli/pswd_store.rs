use anyhow::{anyhow, Result};
use keyring::{Entry, Error};

/// Set password into keyring manager.
pub fn set(entry: &Entry, username: &str, pswd: &str) -> Result<()> {
    Ok(entry.set_password(pswd)?)
}

/// Get password from keyring manager.
pub fn get(entry: &Entry, username: &str) -> Result<String> {
    match entry.get_password() {
        Ok(s) => Ok(s),
        Err(Error::NoEntry) => Err(anyhow!("no password found for current user: {username}")),
        Err(err) => Err(anyhow!("fail to get password from user: {username}: {err}")),
    }
}
