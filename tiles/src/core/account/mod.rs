pub mod atproto;
pub mod local;

use std::str::FromStr;

use anyhow::Result;
use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
use dialog_credentials::{
    Ed25519Signer, Ed25519Verifier, KeyExport,
    native::{SigningKey, VerifyingKey},
};
use dialog_varsig::Principal;
use keyring_core::Entry;
use log::info;

type Did = String;
type Identity = Did;

fn keyring_entry(app: &str, key: &str) -> Result<Entry> {
    ensure_keyring_store()?;
    Ok(Entry::new(app, key)?)
}

fn ensure_keyring_store() -> Result<()> {
    if keyring_core::get_default_store().is_some() {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        keyring_core::set_default_store(dbus_secret_service_keyring_store::Store::new()?);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        keyring_core::set_default_store(apple_native_keyring_store::keychain::Store::new()?);
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    anyhow::bail!("secure storage is not supported on this platform");
}

/// Creates an `Identity` for given application
/// The keypair generated will be stored in OS secure storage
///
/// # Arguments
///
/// - `app`: The service for which Identity is made (for ex: tiles)
pub async fn create_identity(app: &str) -> Result<Identity> {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let key_material = KeyExport::from(&signing_key.to_bytes());

    let signer = Ed25519Signer::import(key_material).await?;
    let did = signer.ed25519_did().did().to_string();
    let entry = keyring_entry(app, &did)?;
    info!("secure did {}", &did);
    entry.set_secret(&signing_key.to_keypair_bytes())?;
    Ok(did)
}

/// Returns the `SecretKey` (ed25519_dalek type, but Private Key)
///
/// # Arguments
///
/// - `app`- The service for which Identity is made (for ex: tiles)
/// - `did` - The `Identity` of the service
pub fn get_secret_key(app: &str, did: &str) -> Result<[u8; 32]> {
    let entry = keyring_entry(app, did)?;
    let mut bytes: [u8; 64] = [0u8; 64];
    let secret_pair = entry.get_secret()?;

    bytes[..64].copy_from_slice(secret_pair.as_slice());

    let signing_key = SigningKey::from_keypair_bytes(&bytes)?;
    Ok(signing_key.to_bytes())
}

/// Returns the `SigningKey` (ed25519_dalek SigningKey)
///
/// # Arguments
///
/// - `app`- The service for which Identity is made (for ex: tiles)
/// - `did` - The `Identity` of the service
pub fn get_signing_key(app: &str, did: &str) -> Result<SigningKey> {
    info!("secure did {}", &did);
    let entry = keyring_entry(app, did)?;
    let mut bytes: [u8; 64] = [0u8; 64];
    let secret_pair = entry.get_secret()?;

    bytes[..64].copy_from_slice(secret_pair.as_slice());

    let signing_key = SigningKey::from_keypair_bytes(&bytes)?;
    Ok(signing_key)
}
pub fn create_and_save_passkey(app: &str, key: &str) -> Result<String> {
    let rand_bytes = get_random_bytes_32()?;
    let rand_hex: String = rand_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let entry = keyring_entry(app, key)?;
    entry.set_secret(rand_bytes.as_slice())?;
    Ok(rand_hex)
}

pub fn get_passkey(app: &str, key: &str) -> Result<String> {
    let entry = keyring_entry(app, key)?;
    let secret = entry.get_secret()?;
    Ok(to_hex(secret.as_slice()))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn get_public_key_from_did(did: &str) -> Result<[u8; 32]> {
    let verifier = Ed25519Verifier::from_str(did)?;

    Ok(verifier.0.to_bytes())
}

pub fn get_did_from_public_key(publick_key: &[u8; 32]) -> Result<String> {
    let verifying_key = VerifyingKey::from_bytes(publick_key)?;

    let verifier = Ed25519Verifier::from(verifying_key);
    Ok(verifier.to_string())
}

pub fn get_random_bytes() -> [u8; 16] {
    let mut value = [0u8; 16];
    OsRng.fill_bytes(&mut value);
    value
}

pub fn get_random_bytes_32() -> Result<[u8; 32]> {
    let mut value = [0u8; 32];
    OsRng.try_fill_bytes(&mut value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn use_sample_keyring_store() -> Result<()> {
        keyring_core::set_default_store(keyring_core::sample::Store::new()?);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_success() -> Result<()> {
        use_sample_keyring_store()?;
        let did = create_identity("tiles").await?;
        assert!(did.starts_with("did:key"));
        Ok(())
    }

    #[test]
    fn test_roundtrip_public_key_did_conversion() {
        let did = "did:key:z6MkqkPYU3eUSs7Pg4NsSTNbm9hKZ4MU997wKFbBwt9gD5k5";

        let pub_key = get_public_key_from_did(did).unwrap();

        assert_eq!(get_did_from_public_key(&pub_key).unwrap(), did);
    }

    #[test]
    fn test_invalid_plc_roundtrip_public_key_did_conversion() {
        let did = "did:plc:mbk6wgmxiatotzy5b3q57naw";

        assert!(get_public_key_from_did(did).is_err());
    }
}
