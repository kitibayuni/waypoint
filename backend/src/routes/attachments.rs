use axum::body::Bytes;
use axum::extract::{Extension, Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

const VALID_SUBJECT_TYPES: [&str; 6] =
    ["engagement", "host", "finding", "observation", "credential", "note"];

fn valid_subject_type(s: &str) -> bool {
    VALID_SUBJECT_TYPES.contains(&s)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Attachment {
    id: Uuid,
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
    filename: String,
    mime: Option<String>,
    size: Option<i64>,
    sha256: String,
    caption: Option<String>,
    created_at: DateTime<Utc>,
}

const ATTACHMENT_SELECT: &str = "SELECT id, engagement_id, subject_type::text AS subject_type,
    subject_id, filename, mime, size, sha256, caption, created_at FROM attachments";

#[derive(Deserialize)]
pub struct ListAttachmentsQuery {
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
}

async fn list_attachments(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<ListAttachmentsQuery>,
) -> Result<Json<Vec<Attachment>>, StatusCode> {
    if !valid_subject_type(&q.subject_type) {
        return Err(StatusCode::BAD_REQUEST);
    }
    require_role(&state.pool, &user, q.engagement_id, EngagementRole::Viewer).await?;

    let attachments = sqlx::query_as::<_, Attachment>(&format!(
        "{ATTACHMENT_SELECT} WHERE engagement_id = $1 AND subject_type = $2::attachment_subject_type
         AND subject_id = $3 ORDER BY created_at"
    ))
    .bind(q.engagement_id)
    .bind(&q.subject_type)
    .bind(q.subject_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(attachments))
}

async fn upload_attachment(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<Json<Attachment>, StatusCode> {
    let mut engagement_id: Option<Uuid> = None;
    let mut subject_type: Option<String> = None;
    let mut subject_id: Option<Uuid> = None;
    let mut caption: Option<String> = None;
    let mut file_bytes: Option<Bytes> = None;
    let mut filename: Option<String> = None;
    let mut mime: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        match field.name().unwrap_or_default() {
            "engagement_id" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                engagement_id = Uuid::parse_str(&text).ok();
            }
            "subject_type" => {
                subject_type = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "subject_id" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                subject_id = Uuid::parse_str(&text).ok();
            }
            "caption" => {
                caption = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "file" => {
                filename = field.file_name().map(|s| s.to_string());
                mime = field.content_type().map(|s| s.to_string());
                file_bytes = Some(field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {}
        }
    }

    let engagement_id = engagement_id.ok_or(StatusCode::BAD_REQUEST)?;
    let subject_type = subject_type.ok_or(StatusCode::BAD_REQUEST)?;
    let subject_id = subject_id.ok_or(StatusCode::BAD_REQUEST)?;
    let file_bytes = file_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let filename = filename.unwrap_or_else(|| "upload.bin".to_string());

    if !valid_subject_type(&subject_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let sha256 = to_hex(&Sha256::digest(&file_bytes));

    // Storage filename is a fresh UUID, decoupled from the user-supplied
    // filename, so there's no path-traversal or collision surface; the
    // original name is kept only in the `filename` DB column for display.
    let storage_name = Uuid::new_v4().to_string();
    let storage_path = state.attachments_dir.join(&storage_name);

    tokio::fs::create_dir_all(&state.attachments_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(&storage_path, &file_bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let size = file_bytes.len() as i64;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO attachments (engagement_id, subject_type, subject_id, filename, mime, size, storage_path, sha256, caption)
         VALUES ($1, $2::attachment_subject_type, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(engagement_id)
    .bind(&subject_type)
    .bind(subject_id)
    .bind(&filename)
    .bind(&mime)
    .bind(size)
    .bind(&storage_name)
    .bind(&sha256)
    .bind(&caption)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let attachment = sqlx::query_as::<_, Attachment>(&format!("{ATTACHMENT_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(attachment))
}

async fn download_attachment(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    #[derive(sqlx::FromRow)]
    struct Row {
        engagement_id: Uuid,
        filename: String,
        mime: Option<String>,
        storage_path: String,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT engagement_id, filename, mime, storage_path FROM attachments WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    require_role(&state.pool, &user, row.engagement_id, EngagementRole::Viewer).await?;

    let bytes = tokio::fs::read(state.attachments_dir.join(&row.storage_path))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mime = row.mime.unwrap_or_else(|| "application/octet-stream".to_string());
    let disposition = format!("attachment; filename=\"{}\"", row.filename.replace('"', ""));

    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response())
}

async fn delete_attachment(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    #[derive(sqlx::FromRow)]
    struct Row {
        engagement_id: Uuid,
        storage_path: String,
    }

    let row = sqlx::query_as::<_, Row>("SELECT engagement_id, storage_path FROM attachments WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    require_role(&state.pool, &user, row.engagement_id, EngagementRole::Tester).await?;

    sqlx::query("DELETE FROM attachments WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = tokio::fs::remove_file(state.attachments_dir.join(&row.storage_path)).await;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/attachments",
            axum::routing::get(list_attachments).post(upload_attachment),
        )
        .route(
            "/attachments/{id}",
            axum::routing::delete(delete_attachment),
        )
        .route("/attachments/{id}/download", axum::routing::get(download_attachment))
}
