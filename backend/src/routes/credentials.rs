use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::log_action;
use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

const VALID_SECRET_TYPES: [&str; 5] = ["plaintext", "ntlm", "kerb", "ssh_key", "hash_other"];
const VALID_ORIGINS: [&str; 5] = ["captured", "cracked", "sprayed", "default", "created"];

fn valid_secret_type(s: &str) -> bool {
    VALID_SECRET_TYPES.contains(&s)
}

fn valid_origin(s: &str) -> bool {
    VALID_ORIGINS.contains(&s)
}

fn default_origin() -> String {
    "captured".to_string()
}

pub(crate) async fn credential_engagement_id(
    pool: &PgPool,
    credential_id: Uuid,
) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>("SELECT engagement_id FROM credentials WHERE id = $1")
        .bind(credential_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|(id,)| id)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Redacted credential shape — never carries the secret. Every list/get
/// response uses this; revealing the plaintext is a separate, explicit
/// action (GET .../reveal).
#[derive(Serialize, sqlx::FromRow)]
pub struct Credential {
    id: Uuid,
    engagement_id: Uuid,
    username: String,
    domain: Option<String>,
    secret_type: String,
    source_host_id: Option<Uuid>,
    source_service_id: Option<Uuid>,
    origin: String,
    validated: bool,
    notes_md: String,
    created_at: DateTime<Utc>,
}

const CREDENTIAL_SELECT: &str = "SELECT id, engagement_id, username, domain,
    secret_type::text AS secret_type, source_host_id, source_service_id, origin::text AS origin,
    validated, notes_md, created_at FROM credentials";

#[derive(Deserialize)]
pub struct CreateCredentialRequest {
    username: String,
    domain: Option<String>,
    secret: String,
    secret_type: String,
    source_host_id: Option<Uuid>,
    /// Only set from the attack graph's "Add credential" action on a service node --
    /// drives a service->credential arrow. Not editable afterward.
    #[serde(default)]
    source_service_id: Option<Uuid>,
    #[serde(default = "default_origin")]
    origin: String,
    #[serde(default)]
    validated: bool,
    #[serde(default)]
    notes_md: String,
}

#[derive(Deserialize)]
pub struct UpdateCredentialRequest {
    username: String,
    domain: Option<String>,
    /// Only re-encrypted if present; omitting it keeps the existing secret,
    /// since we never return the plaintext to pre-fill a full-replace form.
    secret: Option<String>,
    secret_type: String,
    source_host_id: Option<Uuid>,
    origin: String,
    validated: bool,
    notes_md: String,
}

async fn list_credentials(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Credential>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let credentials = sqlx::query_as::<_, Credential>(&format!(
        "{CREDENTIAL_SELECT} WHERE engagement_id = $1 ORDER BY created_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(credentials))
}

async fn create_credential(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<CreateCredentialRequest>,
) -> Result<Json<Credential>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_secret_type(&payload.secret_type) || !valid_origin(&payload.origin) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let encrypted = state.cred_cipher.encrypt(&payload.secret);

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO credentials (engagement_id, username, domain, secret, secret_type, source_host_id, source_service_id, origin, validated, notes_md)
         VALUES ($1, $2, $3, $4, $5::credential_secret_type, $6, $7, $8::credential_origin, $9, $10)
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(&payload.username)
    .bind(&payload.domain)
    .bind(&encrypted)
    .bind(&payload.secret_type)
    .bind(payload.source_host_id)
    .bind(payload.source_service_id)
    .bind(&payload.origin)
    .bind(payload.validated)
    .bind(&payload.notes_md)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let credential = sqlx::query_as::<_, Credential>(&format!("{CREDENTIAL_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Credential audit snapshots reuse the already-redacted response type,
    // so the secret is never written to audit_log either.
    log_action(&state.pool, user.id, "create", "credential", id, None::<&Credential>, Some(&credential)).await;

    Ok(Json(credential))
}

async fn get_credential(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Credential>, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let credential = sqlx::query_as::<_, Credential>(&format!("{CREDENTIAL_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(credential))
}

async fn update_credential(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCredentialRequest>,
) -> Result<Json<Credential>, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_secret_type(&payload.secret_type) || !valid_origin(&payload.origin) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let before = sqlx::query_as::<_, Credential>(&format!("{CREDENTIAL_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = if let Some(secret) = &payload.secret {
        let encrypted = state.cred_cipher.encrypt(secret);
        sqlx::query(
            "UPDATE credentials SET username = $1, domain = $2, secret = $3, secret_type = $4::credential_secret_type,
             source_host_id = $5, origin = $6::credential_origin, validated = $7, notes_md = $8 WHERE id = $9",
        )
        .bind(&payload.username)
        .bind(&payload.domain)
        .bind(&encrypted)
        .bind(&payload.secret_type)
        .bind(payload.source_host_id)
        .bind(&payload.origin)
        .bind(payload.validated)
        .bind(&payload.notes_md)
        .bind(id)
        .execute(&state.pool)
        .await
    } else {
        sqlx::query(
            "UPDATE credentials SET username = $1, domain = $2, secret_type = $3::credential_secret_type,
             source_host_id = $4, origin = $5::credential_origin, validated = $6, notes_md = $7 WHERE id = $8",
        )
        .bind(&payload.username)
        .bind(&payload.domain)
        .bind(&payload.secret_type)
        .bind(payload.source_host_id)
        .bind(&payload.origin)
        .bind(payload.validated)
        .bind(&payload.notes_md)
        .bind(id)
        .execute(&state.pool)
        .await
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let credential = sqlx::query_as::<_, Credential>(&format!("{CREDENTIAL_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_action(&state.pool, user.id, "update", "credential", id, Some(&before), Some(&credential)).await;

    Ok(Json(credential))
}

async fn delete_credential(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let before = sqlx::query_as::<_, Credential>(&format!("{CREDENTIAL_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = sqlx::query("DELETE FROM credentials WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query("DELETE FROM node_positions WHERE engagement_id = $1 AND node_id = $2")
        .bind(engagement_id)
        .bind(format!("credential:{id}"))
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_action(&state.pool, user.id, "delete", "credential", id, Some(&before), None::<&Credential>).await;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct RevealResponse {
    secret: String,
}

async fn reveal_credential(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<RevealResponse>, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let (encrypted,): (Vec<u8>,) = sqlx::query_as("SELECT secret FROM credentials WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let secret = state
        .cred_cipher
        .decrypt(&encrypted)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RevealResponse { secret }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/engagements/{engagement_id}/credentials",
            get(list_credentials).post(create_credential),
        )
        .route(
            "/credentials/{id}",
            get(get_credential)
                .put(update_credential)
                .delete(delete_credential),
        )
        .route("/credentials/{id}/reveal", get(reveal_credential))
}
