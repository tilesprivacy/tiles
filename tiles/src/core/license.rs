//! License Management Module
//!
//! Handles Polar.sh license key activation, validation, and deactivation

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use keyring::Entry;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::config::get_app_name;

const POLAR_BASE_URL: &str = "https://api.polar.sh/v1";
const POLAR_ORG_ID: &str = "028ca25d-5316-46a1-8771-28c6403d8348";

// Product IDs (kept for reference and future use)
#[allow(dead_code)]
const BACKER_PRODUCT_ID: &str = "453d470f-9a16-4fb2-bbcd-09276dcf7e92";
#[allow(dead_code)]
const COMMERCIAL_PRODUCT_ID: &str = "6b9bae85-25c9-4f35-9d64-cbbbc6348d90";

/// License status in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub id: String,
    pub activation_id: String,
    pub benefit_id: String,
    pub product_type: ProductType,
    pub status: LicenseStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub activated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProductType {
    Backer,
    Commercial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LicenseStatus {
    Active,
    Expired,
    Revoked,
}

/// Polar API response for activation
#[derive(Debug, Deserialize)]
struct ActivationResponse {
    id: String,
    license_key: LicenseKeyDetails,
}

#[derive(Debug, Deserialize)]
struct LicenseKeyDetails {
    id: String,
    status: String,
    expires_at: Option<String>,
}

/// Polar API response for validation
#[derive(Debug, Deserialize)]
pub struct ValidationResponse {
    pub id: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub limit_activations: Option<i32>,
    pub usage: Option<i32>,
    pub activation: Option<ActivationDetails>,
}

#[derive(Debug, Deserialize)]
pub struct ActivationDetails {
    pub id: String,
}

/// Gets the current active license from the database
pub fn get_active_license(conn: &Connection) -> Result<Option<LicenseInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, activation_id, benefit_id, product_type, status, expires_at, activated_at
         FROM licenses
         WHERE status = 'Active'
         ORDER BY activated_at DESC
         LIMIT 1"
    )?;

    let license = stmt.query_row([], |row| {
        let product_type_str: String = row.get(3)?;
        let status_str: String = row.get(4)?;
        let expires_str: Option<String> = row.get(5)?;
        let activated_str: String = row.get(6)?;

        Ok(LicenseInfo {
            id: row.get(0)?,
            activation_id: row.get(1)?,
            benefit_id: row.get(2)?,
            product_type: match product_type_str.as_str() {
                "Backer" => ProductType::Backer,
                "Commercial" => ProductType::Commercial,
                _ => ProductType::Backer,
            },
            status: match status_str.as_str() {
                "Active" => LicenseStatus::Active,
                "Expired" => LicenseStatus::Expired,
                "Revoked" => LicenseStatus::Revoked,
                _ => LicenseStatus::Active,
            },
            expires_at: expires_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            activated_at: DateTime::parse_from_rfc3339(&activated_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        })
    });

    match license {
        Ok(lic) => Ok(Some(lic)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Generates a unique device identifier
pub fn get_device_id() -> Result<String> {
    let app_name = get_app_name();
    let entry = Entry::new(&app_name, "device_id")?;

    match entry.get_password() {
        Ok(device_id) => Ok(device_id),
        Err(_) => {
            // Generate new device ID using v4 (random UUID)
            let device_id = Uuid::new_v4().to_string();
            entry.set_password(&device_id)?;
            Ok(device_id)
        }
    }
}

/// Activates a license key with Polar
pub async fn activate_license(license_key: &str, conn: &Connection) -> Result<LicenseInfo> {
    let device_id = get_device_id()?;

    // Call Polar API to activate
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/customer-portal/license-keys/activate", POLAR_BASE_URL))
        .json(&serde_json::json!({
            "key": license_key,
            "organization_id": POLAR_ORG_ID,
            "label": format!("Tiles Device {}", device_id),
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await?;
        return Err(anyhow!("License activation failed ({}): {}", status, error_body));
    }

    let activation: ActivationResponse = response.json().await?;

    // Determine product type from benefit_id or key prefix
    let product_type = if license_key.starts_with("BACKER-") {
        ProductType::Backer
    } else if license_key.starts_with("COMMERCIAL-") {
        ProductType::Commercial
    } else {
        // Fallback: check benefit_id if available
        ProductType::Commercial
    };

    let expires_at = activation.license_key.expires_at
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let activation_id = activation.id.clone();

    let license_info = LicenseInfo {
        id: activation.license_key.id.clone(),
        activation_id: activation_id.clone(),
        benefit_id: activation.license_key.id.clone(),
        product_type: product_type.clone(),
        status: match activation.license_key.status.as_str() {
            "granted" => LicenseStatus::Active,
            "revoked" => LicenseStatus::Revoked,
            _ => LicenseStatus::Active,
        },
        expires_at,
        activated_at: Utc::now(),
    };

    // Store license key in keychain
    let app_name = get_app_name();
    let entry = Entry::new(&app_name, &format!("license_key_{}", activation_id))?;
    entry.set_password(license_key)?;

    // Save to database
    save_license(conn, &license_info)?;

    Ok(license_info)
}

/// Validates the current license with Polar
pub async fn validate_license(license_info: &LicenseInfo) -> Result<bool> {
    let validation = get_license_details(license_info).await?;

    // Check if status is granted
    if validation.status != "granted" {
        return Ok(false);
    }

    // For Commercial licenses, check expiration
    if license_info.product_type == ProductType::Commercial {
        if let Some(expires_at_str) = &validation.expires_at {
            let expires_at = DateTime::parse_from_rfc3339(expires_at_str)?
                .with_timezone(&Utc);
            if expires_at < Utc::now() {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Gets detailed license information from Polar API
pub async fn get_license_details(license_info: &LicenseInfo) -> Result<ValidationResponse> {
    // Retrieve license key from keychain
    let app_name = get_app_name();
    let entry = Entry::new(&app_name, &format!("license_key_{}", license_info.activation_id))?;
    let license_key = entry.get_password()
        .map_err(|_| anyhow!("License key not found in keychain"))?;

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/customer-portal/license-keys/validate", POLAR_BASE_URL))
        .json(&serde_json::json!({
            "key": license_key,
            "organization_id": POLAR_ORG_ID,
            "activation_id": license_info.activation_id,
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await?;
        return Err(anyhow!("License validation failed ({}): {}", status, error_body));
    }

    let validation: ValidationResponse = response.json().await?;
    Ok(validation)
}

/// Removes local license data (DB row + keychain entry).
pub fn purge_local_license(conn: &Connection, license_info: &LicenseInfo) -> Result<()> {
    conn.execute(
        "DELETE FROM licenses WHERE activation_id = ?1",
        [&license_info.activation_id],
    )?;
    let app_name = get_app_name();
    if let Ok(entry) = Entry::new(&app_name, &format!("license_key_{}", license_info.activation_id)) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

/// Deactivates the current license
pub async fn deactivate_license(license_info: &LicenseInfo, conn: &Connection) -> Result<()> {
    // Retrieve license key from keychain
    let app_name = get_app_name();
    let entry = Entry::new(&app_name, &format!("license_key_{}", license_info.activation_id))?;
    let license_key = entry.get_password()
        .map_err(|_| anyhow!("License key not found in keychain"))?;

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/customer-portal/license-keys/deactivate", POLAR_BASE_URL))
        .json(&serde_json::json!({
            "key": license_key,
            "organization_id": POLAR_ORG_ID,
            "activation_id": license_info.activation_id,
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await?;
        return Err(anyhow!("License deactivation failed ({}): {}", status, error_body));
    }

    purge_local_license(conn, license_info)?;

    Ok(())
}

/// Saves license info to database
fn save_license(conn: &Connection, license: &LicenseInfo) -> Result<()> {
    let product_type_str = match license.product_type {
        ProductType::Backer => "Backer",
        ProductType::Commercial => "Commercial",
    };

    let status_str = match license.status {
        LicenseStatus::Active => "Active",
        LicenseStatus::Expired => "Expired",
        LicenseStatus::Revoked => "Revoked",
    };

    let expires_at_str = license.expires_at.map(|dt| dt.to_rfc3339());
    let activated_at_str = license.activated_at.to_rfc3339();

    conn.execute(
        "INSERT OR REPLACE INTO licenses
         (id, activation_id, benefit_id, product_type, status, expires_at, activated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &license.id,
            &license.activation_id,
            &license.benefit_id,
            product_type_str,
            status_str,
            expires_at_str,
            activated_at_str,
        ),
    )?;

    Ok(())
}

/// Gets the license status display string
pub fn get_license_status_string(conn: &Connection) -> String {
    match get_active_license(conn) {
        Ok(Some(license)) => {
            // Validate that license hasn't expired
            if let Some(expires_at) = license.expires_at {
                if expires_at < Utc::now() {
                    return "Unlicensed".to_string();
                }
            }
            match license.product_type {
                ProductType::Backer => "Backer License".to_string(),
                ProductType::Commercial => "Commercial License".to_string(),
            }
        }
        _ => "Unlicensed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ProductType::Backer).unwrap(),
            "\"Backer\""
        );
        assert_eq!(
            serde_json::to_string(&ProductType::Commercial).unwrap(),
            "\"Commercial\""
        );
    }
}
