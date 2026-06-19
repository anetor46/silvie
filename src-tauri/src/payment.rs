use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

const KEYRING_SERVICE: &str = "com.silvie";
const KEYRING_ACCOUNT: &str = "payment";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPaymentMethod {
    pub customer_id: String,
    pub payment_method_id: String,
    pub last4: String,
    pub brand: String,
    pub exp_month: u32,
    pub exp_year: u32,
}

#[instrument]
pub fn store_payment_method(pm: &StoredPaymentMethod) -> Result<()> {
    let payload = serde_json::to_string(pm)?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))?
        .set_password(&payload)
        .map_err(|e| anyhow!("Failed to store payment method: {e}"))?;
    info!(last4 = %pm.last4, brand = %pm.brand, "payment method stored in keychain");
    Ok(())
}

pub fn load_payment_method() -> Option<StoredPaymentMethod> {
    let payload = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .ok()?
        .get_password()
        .ok()?;
    let pm: StoredPaymentMethod = serde_json::from_str(&payload).ok()?;
    debug!(last4 = %pm.last4, "loaded payment method from keychain");
    Some(pm)
}

#[instrument]
pub fn remove_payment_method() -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| anyhow!("Keyring unavailable: {e}"))?
        .delete_credential()
        .map_err(|e| anyhow!("Failed to remove payment method: {e}"))?;
    info!("payment method removed from keychain");
    Ok(())
}
