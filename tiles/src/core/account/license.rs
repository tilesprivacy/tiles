//! Polar.sh license activation, deactivation, and status.
//!
//! Polar's customer-portal license-key endpoints are public (no auth needed).
//! Rate limit is 3 req/s; CLI invocations are single-shot, so no client-side throttling.

use anyhow::{Result, anyhow};
use owo_colors::OwoColorize;
use reqwest::Client;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::core::storage::db::Dbconn;

const POLAR_API_BASE: &str = "https://api.polar.sh/v1";
const POLAR_ORG_ID: &str = "028ca25d-5316-46a1-8771-28c6403d8348";
const PRODUCT_ID_BACKER: &str = "453d470f-9a16-4fb2-bbcd-09276dcf7e92";
const PRODUCT_ID_COMMERCIAL: &str = "6b9bae85-25c9-4f35-9d64-cbbbc6348d90";

const LICENSE_TYPE_BACKER: &str = "backer";
const LICENSE_TYPE_COMMERCIAL: &str = "commercial";

const LABEL_UNLICENSED: &str = "Unlicensed";
const LABEL_BACKER: &str = "Backer License";
const LABEL_COMMERCIAL: &str = "Commercial License";

const KEY_PREFIX_BACKER: &str = "BACKER-";
const KEY_PREFIX_COMMERCIAL: &str = "COMMERCIAL-";

#[derive(Debug, Clone)]
struct StoredLicense {
    license_key: String,
    activation_id: String,
    product_id: String,
    license_type: String,
    expires_at: Option<i64>,
    activations_used: Option<i64>,
    activations_limit: Option<i64>,
    customer_portal_url: Option<String>,
    device_id: String,
    activated_at: i64,
    updated_at: i64,
}

#[derive(Serialize)]
struct ActivateRequest<'a> {
    key: &'a str,
    organization_id: &'a str,
    label: &'a str,
}

#[derive(Deserialize, Debug)]
struct ActivateResponse {
    id: String,
    license_key: LicenseKeyPayload,
}

#[derive(Deserialize, Debug)]
struct LicenseKeyPayload {
    #[allow(dead_code)]
    #[serde(default)]
    id: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    organization_id: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    key: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit_activations: Option<i64>,
    #[serde(default)]
    expires_at: Option<String>,
}

#[derive(Serialize)]
struct ValidateRequest<'a> {
    key: &'a str,
    organization_id: &'a str,
    activation_id: &'a str,
}

#[derive(Serialize)]
struct DeactivateRequest<'a> {
    key: &'a str,
    organization_id: &'a str,
    activation_id: &'a str,
}

pub async fn activate(conn: &Dbconn, license_key: &str) -> Result<()> {
    let key = license_key.trim();
    if key.is_empty() {
        return Err(anyhow!("License key is empty"));
    }

    if let Some(existing) = fetch_license(&conn.common)? {
        return Err(anyhow!(
            "License already active ({}, key {}). Run `tiles license deactivate` first.",
            label_for_type(&existing.license_type, existing.expires_at),
            mask_key(&existing.license_key)
        ));
    }

    let (license_type, product_id) = classify_key(key)?;
    let device_id = Uuid::now_v7().to_string();
    let client = build_http_client()?;

    let activate_resp = polar_activate(&client, key, &device_id).await?;
    let activation_id = activate_resp.id.clone();

    let validate_payload = match polar_validate(&client, key, &activation_id).await {
        Ok(v) => v,
        Err(validate_err) => {
            let _ = polar_deactivate(&client, key, &activation_id).await;
            return Err(anyhow!(
                "License activated but validation failed; activation has been released. Reason: {}",
                validate_err
            ));
        }
    };

    let expires_at = validate_payload
        .expires_at
        .as_deref()
        .and_then(parse_rfc3339_to_unix_seconds);
    let activations_limit = validate_payload
        .limit_activations
        .or(activate_resp.license_key.limit_activations);
    let activations_used: Option<i64> = None;
    let customer_portal_url: Option<String> = None;

    let now = unix_now_seconds();
    let row = StoredLicense {
        license_key: key.to_owned(),
        activation_id,
        product_id: product_id.to_owned(),
        license_type: license_type.to_owned(),
        expires_at,
        activations_used,
        activations_limit,
        customer_portal_url: customer_portal_url.clone(),
        device_id,
        activated_at: now,
        updated_at: now,
    };
    if let Err(upsert_err) = upsert_license(&conn.common, &row) {
        let _ = polar_deactivate(&client, key, &row.activation_id).await;
        return Err(anyhow!(
            "License activated but persistence failed; activation has been released. Reason: {}",
            upsert_err
        ));
    }

    let label = label_for_type(license_type, expires_at);
    println!(
        "{} {} (key {}).",
        "Activated".green(),
        label,
        mask_key(key)
    );
    if let Some(url) = customer_portal_url {
        println!("Manage at {}", url);
    }
    Ok(())
}

pub async fn deactivate(conn: &Dbconn) -> Result<()> {
    let Some(row) = fetch_license(&conn.common)? else {
        println!("No license active.");
        return Ok(());
    };

    let client = build_http_client()?;
    polar_deactivate(&client, &row.license_key, &row.activation_id).await?;
    delete_license(&conn.common)?;
    println!(
        "Deactivated {} (key {}).",
        label_for_type(&row.license_type, row.expires_at),
        mask_key(&row.license_key)
    );
    Ok(())
}

pub async fn status(conn: &Dbconn) -> Result<()> {
    let Some(mut row) = fetch_license(&conn.common)? else {
        println!("No license active.");
        println!("Activate one with: tiles license activate <KEY>");
        return Ok(());
    };

    let mut offline = false;
    let client = build_http_client()?;
    match polar_validate(&client, &row.license_key, &row.activation_id).await {
        Ok(payload) => {
            row.expires_at = payload
                .expires_at
                .as_deref()
                .and_then(parse_rfc3339_to_unix_seconds)
                .or(row.expires_at);
            row.activations_limit = payload.limit_activations.or(row.activations_limit);
            row.updated_at = unix_now_seconds();
            let _ = upsert_license(&conn.common, &row);
        }
        Err(_) => offline = true,
    }

    let label = label_for_type(&row.license_type, row.expires_at);
    println!("License: {}", label);
    println!("Key: {}", mask_key(&row.license_key));
    println!("Activated: {}", format_unix_date(row.activated_at));
    match (row.license_type.as_str(), row.expires_at) {
        (LICENSE_TYPE_BACKER, _) | (_, None) => println!("Expires: Never"),
        (_, Some(exp)) => {
            let now = unix_now_seconds();
            let days = (exp - now) / 86_400;
            let suffix = if exp <= now {
                "EXPIRED".to_owned()
            } else {
                format!("({} days remaining)", days)
            };
            println!("Expires: {} {}", format_unix_date(exp), suffix);
        }
    }
    if let Some(limit) = row.activations_limit {
        println!("Activations allowed: {}", limit);
    }
    if let Some(url) = row.customer_portal_url.as_ref() {
        println!("Manage: {}", url);
    }
    if offline {
        println!("(offline; showing cached values)");
    }
    Ok(())
}

pub fn current_license_label(conn: &Dbconn) -> String {
    let row = fetch_license(&conn.common).ok().flatten();
    match row {
        None => LABEL_UNLICENSED.to_owned(),
        Some(r) => label_for_type(&r.license_type, r.expires_at).to_owned(),
    }
}

fn classify_key(key: &str) -> Result<(&'static str, &'static str)> {
    if key.starts_with(KEY_PREFIX_BACKER) {
        Ok((LICENSE_TYPE_BACKER, PRODUCT_ID_BACKER))
    } else if key.starts_with(KEY_PREFIX_COMMERCIAL) {
        Ok((LICENSE_TYPE_COMMERCIAL, PRODUCT_ID_COMMERCIAL))
    } else {
        Err(anyhow!(
            "Unrecognized license key prefix. Expected BACKER- or COMMERCIAL-."
        ))
    }
}

fn label_for_type(license_type: &str, expires_at: Option<i64>) -> &'static str {
    match license_type {
        LICENSE_TYPE_BACKER => LABEL_BACKER,
        LICENSE_TYPE_COMMERCIAL => match expires_at {
            Some(exp) if exp <= unix_now_seconds() => LABEL_UNLICENSED,
            _ => LABEL_COMMERCIAL,
        },
        _ => LABEL_UNLICENSED,
    }
}

fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    let len = trimmed.chars().count();
    if len <= 4 {
        return "****".to_owned();
    }
    let tail: String = trimmed.chars().skip(len - 4).collect();
    format!("****-{}", tail)
}

fn build_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))
}

async fn polar_activate(
    client: &Client,
    key: &str,
    device_id: &str,
) -> Result<ActivateResponse> {
    let url = format!("{}/customer-portal/license-keys/activate", POLAR_API_BASE);
    let body = ActivateRequest {
        key,
        organization_id: POLAR_ORG_ID,
        label: device_id,
    };
    let res = client.post(url).json(&body).send().await;
    match res {
        Err(err) if err.is_timeout() => Err(anyhow!("Activate request timed out")),
        Err(err) => Err(anyhow!("Activate request failed: {}", err)),
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                res.json::<ActivateResponse>()
                    .await
                    .map_err(|e| anyhow!("Failed to parse activate response: {}", e))
            } else {
                let body = res.text().await.unwrap_or_default();
                Err(anyhow!("Polar activate failed ({}): {}", status, body))
            }
        }
    }
}

async fn polar_validate(
    client: &Client,
    key: &str,
    activation_id: &str,
) -> Result<LicenseKeyPayload> {
    let url = format!("{}/customer-portal/license-keys/validate", POLAR_API_BASE);
    let body = ValidateRequest {
        key,
        organization_id: POLAR_ORG_ID,
        activation_id,
    };
    let res = client.post(url).json(&body).send().await;
    match res {
        Err(err) if err.is_timeout() => Err(anyhow!("Validate request timed out")),
        Err(err) => Err(anyhow!("Validate request failed: {}", err)),
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                res.json::<LicenseKeyPayload>()
                    .await
                    .map_err(|e| anyhow!("Failed to parse validate response: {}", e))
            } else {
                let body = res.text().await.unwrap_or_default();
                Err(anyhow!("Polar validate failed ({}): {}", status, body))
            }
        }
    }
}

async fn polar_deactivate(client: &Client, key: &str, activation_id: &str) -> Result<()> {
    let url = format!("{}/customer-portal/license-keys/deactivate", POLAR_API_BASE);
    let body = DeactivateRequest {
        key,
        organization_id: POLAR_ORG_ID,
        activation_id,
    };
    let res = client.post(url).json(&body).send().await;
    match res {
        Err(err) if err.is_timeout() => Err(anyhow!("Deactivate request timed out")),
        Err(err) => Err(anyhow!("Deactivate request failed: {}", err)),
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                Ok(())
            } else {
                let body = res.text().await.unwrap_or_default();
                Err(anyhow!("Polar deactivate failed ({}): {}", status, body))
            }
        }
    }
}

fn upsert_license(conn: &Connection, row: &StoredLicense) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO polar_license(
            id, license_key, activation_id, product_id, license_type,
            expires_at, activations_used, activations_limit, customer_portal_url,
            device_id, activated_at, updated_at
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            license_key = ?1,
            activation_id = ?2,
            product_id = ?3,
            license_type = ?4,
            expires_at = ?5,
            activations_used = ?6,
            activations_limit = ?7,
            customer_portal_url = ?8,
            device_id = ?9,
            activated_at = ?10,
            updated_at = ?11",
    )?;
    stmt.execute(params![
        row.license_key,
        row.activation_id,
        row.product_id,
        row.license_type,
        row.expires_at,
        row.activations_used,
        row.activations_limit,
        row.customer_portal_url,
        row.device_id,
        row.activated_at,
        row.updated_at,
    ])
    .map_err(|e| anyhow!("Failed to write license row: {}", e))?;
    Ok(())
}

fn fetch_license(conn: &Connection) -> Result<Option<StoredLicense>> {
    conn.query_row(
        "SELECT license_key, activation_id, product_id, license_type, expires_at,
                activations_used, activations_limit, customer_portal_url, device_id,
                activated_at, updated_at
         FROM polar_license WHERE id = 1",
        [],
        |row| {
            Ok(StoredLicense {
                license_key: row.get(0)?,
                activation_id: row.get(1)?,
                product_id: row.get(2)?,
                license_type: row.get(3)?,
                expires_at: row.get(4)?,
                activations_used: row.get(5)?,
                activations_limit: row.get(6)?,
                customer_portal_url: row.get(7)?,
                device_id: row.get(8)?,
                activated_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .optional()
    .map_err(Into::<anyhow::Error>::into)
}

fn delete_license(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM polar_license WHERE id = 1", [])
        .map_err(|e| anyhow!("Failed to delete license row: {}", e))?;
    Ok(())
}

fn unix_now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// Minimal RFC3339 parser for the form `YYYY-MM-DDThh:mm:ss[.fff][Z|±hh:mm]`.
// Returns unix seconds (UTC). Only supports the formats Polar emits.
fn parse_rfc3339_to_unix_seconds(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' || bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    let second: u32 = s.get(17..19)?.parse().ok()?;

    // Skip fractional seconds.
    let mut idx = 19;
    if bytes.get(idx) == Some(&b'.') {
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
    }

    // Timezone: Z, +hh:mm, -hh:mm, or empty (assume UTC).
    let mut offset_seconds: i64 = 0;
    if idx < bytes.len() {
        match bytes[idx] {
            b'Z' | b'z' => {}
            b'+' | b'-' => {
                let sign: i64 = if bytes[idx] == b'-' { -1 } else { 1 };
                let off = s.get(idx + 1..)?;
                let (oh, om): (i64, i64) = if off.len() >= 5 && off.as_bytes().get(2) == Some(&b':') {
                    (off.get(0..2)?.parse().ok()?, off.get(3..5)?.parse().ok()?)
                } else if off.len() >= 4 {
                    (off.get(0..2)?.parse().ok()?, off.get(2..4)?.parse().ok()?)
                } else {
                    return None;
                };
                offset_seconds = sign * (oh * 3600 + om * 60);
            }
            _ => return None,
        }
    }

    let days = days_from_civil(year, month, day);
    let secs_of_day = (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64);
    Some(days * 86_400 + secs_of_day - offset_seconds)
}

// Howard Hinnant's "days_from_civil" — converts (year, month, day) to days since 1970-01-01.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn format_unix_date(secs: i64) -> String {
    // YYYY-MM-DD UTC
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS polar_license (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                license_key TEXT NOT NULL,
                activation_id TEXT NOT NULL,
                product_id TEXT NOT NULL,
                license_type TEXT NOT NULL,
                expires_at INTEGER,
                activations_used INTEGER,
                activations_limit INTEGER,
                customer_portal_url TEXT,
                device_id TEXT NOT NULL,
                activated_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn sample(license_type: &str, expires_at: Option<i64>) -> StoredLicense {
        StoredLicense {
            license_key: "BACKER-AAAA-BBBB-CCCC-1234".to_owned(),
            activation_id: "act-1".to_owned(),
            product_id: PRODUCT_ID_BACKER.to_owned(),
            license_type: license_type.to_owned(),
            expires_at,
            activations_used: Some(1),
            activations_limit: Some(5),
            customer_portal_url: Some("https://polar.sh/portal".to_owned()),
            device_id: "device-1".to_owned(),
            activated_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        }
    }

    #[test]
    fn upsert_and_fetch_roundtrip() {
        let conn = setup_db_schema();
        let row = sample(LICENSE_TYPE_BACKER, None);
        upsert_license(&conn, &row).unwrap();
        let got = fetch_license(&conn).unwrap().unwrap();
        assert_eq!(got.license_key, row.license_key);
        assert_eq!(got.license_type, LICENSE_TYPE_BACKER);
        assert_eq!(got.expires_at, None);
    }

    #[test]
    fn upsert_overwrites_existing_row() {
        let conn = setup_db_schema();
        upsert_license(&conn, &sample(LICENSE_TYPE_BACKER, None)).unwrap();

        let mut row2 = sample(LICENSE_TYPE_COMMERCIAL, Some(2_000_000_000));
        row2.license_key = "COMMERCIAL-XXXX-YYYY-ZZZZ-9999".to_owned();
        row2.product_id = PRODUCT_ID_COMMERCIAL.to_owned();
        upsert_license(&conn, &row2).unwrap();

        let got = fetch_license(&conn).unwrap().unwrap();
        assert_eq!(got.license_type, LICENSE_TYPE_COMMERCIAL);
        assert_eq!(got.expires_at, Some(2_000_000_000));
        assert!(got.license_key.starts_with("COMMERCIAL-"));
    }

    #[test]
    fn delete_removes_row() {
        let conn = setup_db_schema();
        upsert_license(&conn, &sample(LICENSE_TYPE_BACKER, None)).unwrap();
        delete_license(&conn).unwrap();
        assert!(fetch_license(&conn).unwrap().is_none());
    }

    #[test]
    fn fetch_empty_returns_none() {
        let conn = setup_db_schema();
        assert!(fetch_license(&conn).unwrap().is_none());
    }

    #[test]
    fn classify_key_recognizes_prefixes() {
        assert_eq!(
            classify_key("BACKER-1234").unwrap(),
            (LICENSE_TYPE_BACKER, PRODUCT_ID_BACKER)
        );
        assert_eq!(
            classify_key("COMMERCIAL-1234").unwrap(),
            (LICENSE_TYPE_COMMERCIAL, PRODUCT_ID_COMMERCIAL)
        );
        assert!(classify_key("UNKNOWN-1234").is_err());
    }

    #[test]
    fn label_for_type_handles_expiry() {
        assert_eq!(label_for_type(LICENSE_TYPE_BACKER, None), LABEL_BACKER);
        let future = unix_now_seconds() + 86_400;
        let past = unix_now_seconds() - 1;
        assert_eq!(
            label_for_type(LICENSE_TYPE_COMMERCIAL, Some(future)),
            LABEL_COMMERCIAL
        );
        assert_eq!(
            label_for_type(LICENSE_TYPE_COMMERCIAL, Some(past)),
            LABEL_UNLICENSED
        );
        assert_eq!(label_for_type("unknown", None), LABEL_UNLICENSED);
    }

    #[test]
    fn mask_key_redacts_all_but_last_four() {
        assert_eq!(mask_key("BACKER-AAAA-BBBB-CCCC-1234"), "****-1234");
        assert_eq!(mask_key("ABCD"), "****");
    }

    #[test]
    fn parse_rfc3339_basic() {
        // 2027-04-28T00:00:00Z
        let expected = 1_808_870_400_i64;
        assert_eq!(
            parse_rfc3339_to_unix_seconds("2027-04-28T00:00:00Z"),
            Some(expected)
        );
        // With fractional seconds
        assert_eq!(
            parse_rfc3339_to_unix_seconds("2027-04-28T00:00:00.123Z"),
            Some(expected)
        );
        // Negative offset -> earlier UTC
        assert_eq!(
            parse_rfc3339_to_unix_seconds("2027-04-28T00:00:00-05:00"),
            Some(expected + 5 * 3600)
        );
        // Garbage
        assert_eq!(parse_rfc3339_to_unix_seconds("not-a-date"), None);
    }

    #[test]
    fn format_unix_date_round_trip() {
        let secs = parse_rfc3339_to_unix_seconds("2027-04-28T12:34:56Z").unwrap();
        assert_eq!(format_unix_date(secs), "2027-04-28");
    }
}
