// Stuff related to Tiles license billing and management

// For gateway verification, we send the DID, activationID  and a signed message to a serveless api
// which verifies the signature and then calls the validate api

use std::time::Duration;

use anyhow::{Result, anyhow};
use axum::http::HeaderMap;
use reqwest::Client;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{core::account::local::get_current_user, utils::get_unix_time_now};

const HEADER_PARSING_ERROR: &str = "Failed to parse header";

const BASE_URL_DEV: &str = "https://sandbox-api.polar.sh/v1";
const BASE_URL: &str = "https://api.polar.sh/v1";
const POLAR_ORGANIZATION_ID_DEV: &str = "71cde5b6-b910-46e9-9b04-53bb3ec63b7f";
const POLAR_ORGANIZATION_ID: &str = "028ca25d-5316-46a1-8771-28c6403d8348";

#[derive(Serialize, Deserialize, Debug)]
pub struct License {
    // license key from Polar
    pub key: String,
    // If this key is used currently
    pub active: bool,
    // Activation Id from polar, which will be used for validation
    activation_id: String,
    // License status from Polar
    pub polar_status: String,
    created_at: u64,
    updated_at: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct LicenseKey {
    status: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct ActivateResponse {
    id: String,
    license_key_id: String,
    label: String,
    license_key: LicenseKey,
}

// We will replace Polar api with custom api for more validation server-side
pub async fn validate_license(license_key: &str, base_url: Option<String>) -> Result<()> {
    let client = get_client()?;
    let body = json!({
       "key": license_key,
       "organization_id": get_org_id()
    });
    let response = client
        .post(format!(
            "{}/{}",
            get_base_url(base_url),
            "customer-portal/license-keys/validate"
        ))
        .json(&body)
        .send()
        .await;
    match response {
        Ok(result) if result.status() == 200 => Ok(()),
        Ok(result) if result.status() == 404 => Err(anyhow!("License key not found")),
        Ok(result) => {
            let err_msg = format!("Error with code {} occured", result.status());
            Err(anyhow!(err_msg))
        }
        Err(err) => Err(anyhow!(err.to_string())),
    }
}

pub async fn activate_license(
    license_key: &str,
    common_db_conn: &mut Connection,
    base_url: Option<String>,
) -> Result<License> {
    //TODO: Prevent hitting activation, if its done in DB
    let client = get_client()?;
    let user = get_current_user(common_db_conn)?;
    if let Ok(license) = fetch_license(license_key, common_db_conn)
        && !license.activation_id.is_empty()
    {
        return Err(anyhow!("License already activated for this device"));
    }
    let body = json!({
       "key": license_key,
       "organization_id": get_org_id(),
       "label": user.user_id
    });
    let response = client
        .post(format!(
            "{}/{}",
            get_base_url(base_url),
            "customer-portal/license-keys/activate"
        ))
        .json(&body)
        .send()
        .await;
    match response {
        Ok(result) if result.status() == 200 => {
            let activate_resp = result.json::<ActivateResponse>().await?;
            log::info!("{:?}", activate_resp);

            do_activate_license(
                common_db_conn,
                &License {
                    key: license_key.to_string(),
                    active: true,
                    activation_id: activate_resp.id,
                    polar_status: activate_resp.license_key.status,
                    created_at: get_unix_time_now(),
                    updated_at: get_unix_time_now(),
                },
            )
        }
        Ok(result) if result.status() == 403 => Err(anyhow!(
            "License key activation not supported or limit reached."
        )),
        Ok(result) if result.status() == 404 => Err(anyhow!("License key not found")),
        Ok(result) => {
            let err_msg = format!("Error with code {} occured", result.status());
            Err(anyhow!(err_msg))
        }
        Err(err) => Err(anyhow!(err.to_string())),
    }
}
pub async fn deactivate_license(
    license_key: &str,
    common_db_conn: &Connection,
    base_url: Option<String>,
) -> Result<License> {
    let client = get_client()?;
    let license = fetch_license(license_key, common_db_conn)?;
    let body = json!({
       "key": license_key,
       "organization_id": get_org_id(),
       "activation_id": license.activation_id
    });
    let response = client
        .post(format!(
            "{}/{}",
            get_base_url(base_url),
            "customer-portal/license-keys/deactivate"
        ))
        .json(&body)
        .send()
        .await;
    match response {
        Ok(result) if result.status() == 204 => {
            // let activate_resp = result.json::<serde_json::Value>().await?;
            // log::info!("{:?}", activate_resp);
            do_deactivate_license(license_key, common_db_conn)
        }
        Ok(result) if result.status() == 404 => Err(anyhow!("License key not found")),
        Ok(result) => {
            let err_msg = format!("Error with code {} occured", result.status());
            Err(anyhow!(err_msg))
        }
        Err(err) => Err(anyhow!(err.to_string())),
    }
}
fn get_client() -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Accept",
        "application/json"
            .parse()
            .map_err(|_e| anyhow!(HEADER_PARSING_ERROR))?,
    );
    headers.insert("user-agent", "Tiles".parse().expect(HEADER_PARSING_ERROR));
    let client_builder = Client::builder()
        .timeout(Duration::from_secs(60))
        .default_headers(headers);

    let client = client_builder.build()?;
    Ok(client)
}

fn get_base_url(url: Option<String>) -> String {
    if let Some(base) = url {
        return base;
    }
    if cfg!(debug_assertions) {
        BASE_URL_DEV.to_owned()
    } else {
        BASE_URL.to_owned()
    }
}

fn get_org_id() -> &'static str {
    if cfg!(debug_assertions) {
        POLAR_ORGANIZATION_ID_DEV
    } else {
        POLAR_ORGANIZATION_ID
    }
}

fn do_activate_license(conn: &mut Connection, data: &License) -> Result<License> {
    let txn = conn.transaction()?;
    {
        let mut stmt = txn.prepare(
        "insert into licenses(key, polar_status, active, activation_id, created_at, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6)
         on conflict(key)
         do update set polar_status= ?2, updated_at = ?6, activation_id= ?4, active = ?3
         ",
    )?;

        match stmt.execute(params![
            data.key.to_owned(),
            data.polar_status.to_owned(),
            data.active.to_owned(),
            data.activation_id.to_string(),
            data.created_at as f64,
            data.updated_at as f64,
        ]) {
            Ok(_res) => Ok(()),
            Err(err) => Err(anyhow!("Err inserting due to {}", err)),
        }?;

        let mut stmt = txn.prepare("update licenses set active = false where key != ?1")?;

        match stmt.execute(params![data.key.to_owned()]) {
            Ok(_res) => Ok(()),
            Err(err) => Err(anyhow!("Err updating due to {}", err)),
        }?;
    }
    txn.commit()?;

    fetch_license(&data.key, conn)
}

fn fetch_license(key: &str, conn: &Connection) -> Result<License> {
    let license = conn.query_row("select key, polar_status, active, activation_id, created_at, updated_at from licenses where key = ?1",
    [key],
    |row| {
        Ok(License{
            key: row.get(0)?,
            polar_status: row.get(1)?,
            active: row.get(2)?,
            activation_id: row.get(3)?,
            created_at: row.get::<usize, f64>(4)? as u64,
            updated_at: row.get::<usize, f64>(5)? as u64
        })
    }
    )?;

    Ok(license)
}

pub fn fetch_licenses(conn: &Connection) -> Result<Vec<License>> {
    let query = "select key, polar_status, active, activation_id, created_at, updated_at from licenses order by updated_at desc";

    let mut stmt = conn.prepare(query)?;
    let license_rows = stmt.query_map([], |row| {
        Ok(License {
            key: row.get(0)?,
            polar_status: row.get(1)?,
            active: row.get(2)?,
            activation_id: row.get(3)?,
            created_at: row.get::<usize, f64>(4)? as u64,
            updated_at: row.get::<usize, f64>(5)? as u64,
        })
    })?;

    let mut licenses: Vec<License> = vec![];

    for license in license_rows {
        licenses.push(license?);
    }
    Ok(licenses)
}

fn do_deactivate_license(license_key: &str, db_conn: &Connection) -> Result<License> {
    let mut stmt = db_conn.prepare(
        "update licenses set polar_status = ?1, active = ?2, updated_at = ?4 where key = ?3",
    )?;

    if let Err(err) = stmt.execute(params![
        "revoked".to_string(),
        false,
        license_key.to_string(),
        get_unix_time_now() as f64
    ]) {
        return Err(anyhow!("Error updating license info due to {}", err));
    }
    fetch_license(license_key, db_conn)
}

#[cfg(test)]
mod tests {

    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use crate::core::{
        account::local::{create_dummy_user, tests::setup_db_conn_v2},
        license::{activate_license, deactivate_license, fetch_license, validate_license},
    };

    #[tokio::test]
    async fn test_activate_valid_license() {
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d473",
                    "label": "did:key",
                    "license_key": {
                        "status": "granted"
                    }
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(license.active);
        assert_eq!(license.polar_status, "granted")
    }

    #[tokio::test]
    async fn test_activate_invalid_license() {
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
        .and(path("customer-portal/license-keys/activate"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!(
            {
                "error": "NotPermitted",
                "detail": "License key activation not supported or limit reached. Use /validate endpoint for licenses without activations."
            }
        )))
        .mount(&mock_server).await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        if let Err(err) = res.await {
            assert_eq!(
                err.to_string(),
                "License key activation not supported or limit reached.".to_owned()
            );
        }
    }

    #[tokio::test]
    async fn test_activate_invalid_license_2() {
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!(
                {
                    "error": "ResourceNotFound",
                    "detail": "license not found"
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        if let Err(err) = res.await {
            assert_eq!(err.to_string(), "License key not found".to_owned());
        }
    }

    #[tokio::test]
    async fn test_activate_valid_multiple_license() {
        // the latest activated will be the active license used in Tiles
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d473",
                    "label": "did:key",
                    "license_key": {
                        "status": "granted"
                    }
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(license.active);
        assert_eq!(license.polar_status, "granted");
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d469",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d469",
                    "label": "did:key",
                    "license_key": {
                        "status": "granted"
                    }
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d469",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(license.active);
        assert_eq!(license.polar_status, "granted");
        assert_eq!(
            license.activation_id,
            "f47ac10b-58cc-4372-a567-0e02b2c3d469"
        );

        let first_license =
            fetch_license("f47ac10b-58cc-4372-a567-0e02b2c3d473", &db_conn.common).unwrap();

        assert!(!first_license.active);
    }

    #[tokio::test]
    async fn test_validate_valid_license() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/validate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d473",
                    "label": "did:key"
                }
            )))
            .mount(&mock_server)
            .await;

        let res = validate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            Some(mock_server.uri()),
        );

        assert!(res.await.is_ok())
    }

    #[tokio::test]
    async fn test_validate_invalid_license_2() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/validate"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!(
                {
                    "error": "ResourceNotFound",
                    "detail": "license not found"
                }
            )))
            .mount(&mock_server)
            .await;

        let res = validate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            Some(mock_server.uri()),
        );

        if let Err(err) = res.await {
            assert_eq!(err.to_string(), "License key not found".to_owned());
        }
    }

    #[tokio::test]
    async fn test_deactivate_valid_license() {
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d473",
                    "label": "did:key",
                    "license_key": {
                        "status": "granted"
                    }
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(license.active);
        assert_eq!(license.polar_status, "granted");

        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/deactivate"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let res = deactivate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(!license.active);
        assert_eq!(license.polar_status, "revoked");
    }

    #[tokio::test]
    async fn test_activating_valid_license_multiple_times() {
        let mock_server = MockServer::start().await;
        let mut db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, None);
        Mock::given(method("POST"))
            .and(path("customer-portal/license-keys/activate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
                    "license_key_id": "f47ac10b-58cc-4372-a567-0e02b2c3d473",
                    "label": "did:key",
                    "license_key": {
                        "status": "granted"
                    }
                }
            )))
            .mount(&mock_server)
            .await;

        let res = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        );

        let license = res.await.unwrap();

        assert!(license.active);
        assert_eq!(license.polar_status, "granted");
        if let Err(err) = activate_license(
            "f47ac10b-58cc-4372-a567-0e02b2c3d473",
            &mut db_conn.common,
            Some(mock_server.uri()),
        )
        .await
        {
            assert_eq!(err.to_string(), "License already activated for this device")
        }
    }
}
