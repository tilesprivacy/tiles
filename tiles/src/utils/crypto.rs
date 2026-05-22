//! Cryptographic utilities

use anyhow::{Result, anyhow};
use chacha20poly1305::{
    Key, KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use serde::{Deserialize, Serialize};

// All strings are base64 encoded (w/o padding)
#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedBase64Content {
    pub ciphertext: String,
    pub nonce: String,
    pub key: String,
}
pub fn encrypt_to_base64(content: &[u8]) -> Result<EncryptedBase64Content> {
    let mut key_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut key_bytes);

    let key = Key::from_slice(&key_bytes);
    let cipher = XChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, content.as_ref())
        .map_err(|_e| anyhow!("Encryption failed for ciphertext"))?;

    Ok(EncryptedBase64Content {
        ciphertext: data_encoding::BASE64.encode(&ciphertext),
        nonce: data_encoding::BASE64.encode(nonce),
        key: data_encoding::BASE64.encode(key),
    })
}

pub fn decrypt_from_base64(encrypted_content: EncryptedBase64Content) -> Result<Vec<u8>> {
    let nonce_bytes = data_encoding::BASE64.decode(encrypted_content.nonce.as_bytes())?;

    let key_bytes = data_encoding::BASE64.decode(encrypted_content.key.as_bytes())?;

    let ciphertext_bytes = data_encoding::BASE64.decode(encrypted_content.ciphertext.as_bytes())?;

    if key_bytes.len() != 32 {
        return Err(anyhow!(
            "Invalid key length: expected 32 bytes, got {}",
            key_bytes.len()
        ));
    }
    if nonce_bytes.len() != 24 {
        return Err(anyhow!(
            "Invalid nonce length: expected 24 bytes, got {}",
            nonce_bytes.len()
        ));
    }
    let key = Key::from_slice(&key_bytes);
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let content = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|_e| anyhow!("Failed to decrypt"))?;

    Ok(content)
}

#[cfg(test)]
mod tests {

    use atrium_api::types::string::Datetime;

    use crate::repl::SharedSession;

    use super::*;

    #[test]
    fn test_round_trip_string_content_encryption() {
        let content = "hello world";

        let enc_content = encrypt_to_base64(content.as_bytes());

        assert!(enc_content.is_ok());

        let d_content_b = decrypt_from_base64(enc_content.unwrap()).unwrap();
        let dec_content = String::from_utf8(d_content_b).unwrap();

        assert_eq!(content, dec_content);
    }

    #[test]
    fn test_round_trip_shared_session_encryption() {
        let shared_session = SharedSession {
            r#type: "run.tiles.session".to_string(),
            session_id: String::from("session_abc"),
            name: String::from("super_session"),
            contents: vec![],
            created_at: Datetime::now().as_str().to_string(),
            models_used: vec!["model".to_string()],
        };

        let enc_content = encrypt_to_base64(&serde_json::to_vec(&shared_session).unwrap());

        assert!(enc_content.is_ok());

        let d_content_b = decrypt_from_base64(enc_content.unwrap()).unwrap();
        let v = serde_json::from_slice(&d_content_b).unwrap();
        let dec_content: SharedSession = serde_json::from_value(v).unwrap();

        assert_eq!(shared_session.name, dec_content.name);
    }
}
