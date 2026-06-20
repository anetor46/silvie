use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

const KEYRING_SERVICE: &str = "com.silvie";
const KEYRING_ACCOUNT: &str = "profile";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProfile {
    // Core — required by onboarding
    pub first_name: String,
    pub last_name: String,
    pub email: String,

    // Contact
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phone: Option<String>,

    // Travel documents
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nationality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub passport_number: Option<String>,
    /// ISO date string "YYYY-MM-DD"
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub passport_expiry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub country_of_residence: Option<String>,

    // Home address (for hotel billing)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub address_line1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub address_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub address_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub address_postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub address_country: Option<String>,
}

#[instrument(skip(p))]
pub fn store_profile(p: &StoredProfile) -> Result<()> {
    let payload = serde_json::to_string(p)?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))?
        .set_password(&payload)
        .map_err(|e| anyhow!("Failed to store profile: {e}"))?;
    info!(email_len = p.email.len(), "profile stored in keychain");
    Ok(())
}

pub fn load_profile() -> Option<StoredProfile> {
    let payload = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .ok()?
        .get_password()
        .ok()?;
    let p: StoredProfile = serde_json::from_str(&payload).ok()?;
    debug!("profile loaded from keychain");
    Some(p)
}
