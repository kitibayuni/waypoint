use std::path::PathBuf;
use std::sync::Arc;

use sqlx::PgPool;

use crate::crypto::CredentialCipher;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cred_cipher: Arc<CredentialCipher>,
    pub attachments_dir: PathBuf,
}
