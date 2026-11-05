use std::io::{self, BufRead};

use thriftp_core::error::{CoreError, CoreResult};

/// Read one line from stdin and return a zeroizing secret.
pub fn read_secret() -> CoreResult<thriftp_core::credentials::Secret> {
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|_error| CoreError::CredentialMissing)?;
    let trimmed = line.trim_end_matches(['\r', '\n']).to_owned();
    if trimmed.is_empty() {
        return Err(CoreError::CredentialMissing);
    }
    Ok(thriftp_core::credentials::Secret::new(trimmed.into_bytes()))
}
