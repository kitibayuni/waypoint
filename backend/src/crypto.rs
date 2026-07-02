use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand::RngExt;
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;

/// AEAD encryption for credential secrets at rest. The key is derived by
/// hashing CRED_ENC_KEY (an arbitrary-length env var) down to 32 bytes, so
/// operators don't have to hand-generate a precisely-sized key.
pub struct CredentialCipher {
    cipher: Aes256Gcm,
}

impl CredentialCipher {
    pub fn from_env_key(env_key: &str) -> Self {
        let key_bytes = Sha256::digest(env_key.as_bytes());
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("digest is always 32 bytes");
        Self { cipher }
    }

    /// Returns nonce || ciphertext, stored as-is in the BYTEA column.
    pub fn encrypt(&self, plaintext: &str) -> Vec<u8> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::try_from(nonce_bytes).expect("nonce is always 12 bytes");
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .expect("encryption failure");

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        out
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<String, ()> {
        if data.len() < NONCE_LEN {
            return Err(());
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::try_from(nonce_bytes).map_err(|_| ())?;
        let plaintext = self.cipher.decrypt(&nonce, ciphertext).map_err(|_| ())?;
        String::from_utf8(plaintext).map_err(|_| ())
    }
}
