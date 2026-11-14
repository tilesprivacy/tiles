//! Cryptographic utilities
use anyhow::{Result, anyhow};
use chacha20poly1305::{
    Key, KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use hpke::{
    Deserializable, Kem as KemTrait, Serializable, aead::ChaCha20Poly1305, kdf::HkdfSha256,
    kem::X25519HkdfSha256, setup_receiver,
};
use serde::{Deserialize, Serialize};

type Kem = X25519HkdfSha256;
type HpkeAead = ChaCha20Poly1305;
type Kdf = HkdfSha256;

// All strings are base64 encoded (w/o padding)
#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedBase64Content {
    pub ciphertext: String,
    pub nonce: String,
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ContentKey {
    pub key: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl Default for ContentKey {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentKey {
    pub fn new() -> Self {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);

        let key = Key::from_slice(&key_bytes);

        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        ContentKey {
            key: key.to_vec(),
            nonce: nonce.to_vec(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("Failed to convert content_key to bytes with postcard")
    }
}

pub fn encrypt_by_content_key(content: &[u8], content_key: &ContentKey) -> Result<Vec<u8>> {
    let key = content_key.key.as_slice().into();
    let nonce = content_key.nonce.as_slice().into();
    let cipher = XChaCha20Poly1305::new(key);

    cipher
        .encrypt(nonce, content.as_ref())
        .map_err(|_e| anyhow!("Encryption failed for ciphertext"))
}

pub fn decrypt_by_content_key(content: &[u8], content_key: &ContentKey) -> Result<Vec<u8>> {
    let key = Key::from_slice(&content_key.key);
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(&content_key.nonce);

    cipher
        .decrypt(nonce, content.as_ref())
        .map_err(|_e| anyhow!("Failed to decrypt"))
}

pub fn encrypt_to_base64(content: &[u8]) -> Result<EncryptedBase64Content> {
    let content_key = ContentKey::new();
    let enc_content = encrypt_by_content_key(content, &content_key)?;

    Ok(EncryptedBase64Content {
        ciphertext: data_encoding::BASE64.encode(&enc_content),
        nonce: data_encoding::BASE64.encode(content_key.nonce.as_slice()),
        key: data_encoding::BASE64.encode(content_key.key.as_slice()),
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
    let decrypted_content = decrypt_by_content_key(
        &ciphertext_bytes,
        &ContentKey {
            key: key_bytes,
            nonce: nonce_bytes,
        },
    )?;

    Ok(decrypted_content)
}

pub fn create_kem_pair() -> (Vec<u8>, Vec<u8>) {
    let (sec_key, pub_key) = Kem::gen_keypair();
    (sec_key.to_bytes().to_vec(), pub_key.to_bytes().to_vec())
}

pub fn encrypt_by_hpke(
    pub_key: Vec<u8>,
    domain: &str,
    content: Vec<u8>,
    aad: &str,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let pkey = <Kem as KemTrait>::PublicKey::from_bytes(&pub_key)?;

    let (encapped_key, mut encrypt_ctx) =
        hpke::setup_sender::<HpkeAead, Kdf, Kem>(&hpke::OpModeS::Base, &pkey, domain.as_bytes())?;

    let encrypted_content = encrypt_ctx.seal(&content, aad.as_bytes())?;
    Ok((encapped_key.to_bytes().to_vec(), encrypted_content))
}

pub fn decrypt_by_hpke(
    secret_key: Vec<u8>,
    encapped_key: Vec<u8>,
    enc_content: Vec<u8>,
    domain: &str,
    aad: &str,
) -> Result<Vec<u8>> {
    let sec_key = <Kem as KemTrait>::PrivateKey::from_bytes(&secret_key)?;

    let mut decrypt_ctx = setup_receiver::<HpkeAead, Kdf, Kem>(
        &hpke::OpModeR::Base,
        &sec_key,
        &<Kem as KemTrait>::EncappedKey::from_bytes(&encapped_key)?,
        domain.as_bytes(),
    )?;

    decrypt_ctx
        .open(&enc_content, aad.as_bytes())
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {

    use atrium_api::types::string::Datetime;
    use uuid::Uuid;

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

    #[test]
    fn test_round_trip_hpke_encryption() {
        // let (alice_sk, alice_pk) = create_kem_pair();
        let (bob_sk, bob_pk) = create_kem_pair();

        let alice_data = "hello world";
        let common_domain = "hpke_v2";
        let aad: String = Uuid::now_v7().into();

        // Alice encrypted her data with bob's public key
        let (encapped_key, enc_content) =
            encrypt_by_hpke(bob_pk, common_domain, alice_data.into(), &aad).unwrap();

        // Bob decrypting with his private key and the encapped_key
        let og_content =
            decrypt_by_hpke(bob_sk, encapped_key, enc_content, common_domain, &aad).unwrap();

        assert_eq!(alice_data, String::from_utf8_lossy(&og_content))
    }
}
