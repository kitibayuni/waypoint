use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

const SESSION_TTL_DAYS: i64 = 7;

pub struct CreatedSession {
    pub raw_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    to_hex(&bytes)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    to_hex(&hasher.finalize())
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<CreatedSession, sqlx::Error> {
    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);

    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token_hash)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;

    Ok(CreatedSession {
        raw_token,
        expires_at,
    })
}

pub async fn validate_session(pool: &PgPool, raw_token: &str) -> Option<SessionUser> {
    let token_hash = hash_token(raw_token);
    sqlx::query_as::<_, SessionUser>(
        "SELECT u.id, u.email, u.display_name, u.is_admin
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

pub async fn delete_session(pool: &PgPool, raw_token: &str) {
    let token_hash = hash_token(raw_token);
    let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(pool)
        .await;
}
