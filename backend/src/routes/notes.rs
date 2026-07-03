use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

const VALID_SUBJECT_TYPES: [&str; 5] = ["engagement", "host", "finding", "observation", "credential"];

fn valid_subject_type(s: &str) -> bool {
    VALID_SUBJECT_TYPES.contains(&s)
}

async fn note_engagement_id(pool: &PgPool, id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>("SELECT engagement_id FROM notes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|(id,)| id)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Note {
    id: Uuid,
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
    title: Option<String>,
    body_md: String,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

const NOTE_SELECT: &str = "SELECT id, engagement_id, subject_type::text AS subject_type, subject_id,
    title, body_md, created_by, created_at FROM notes";

#[derive(Deserialize)]
pub struct ListNotesQuery {
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
    title: Option<String>,
    #[serde(default)]
    body_md: String,
}

#[derive(Deserialize)]
pub struct UpdateNoteRequest {
    title: Option<String>,
    body_md: String,
}

async fn list_notes(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<ListNotesQuery>,
) -> Result<Json<Vec<Note>>, StatusCode> {
    if !valid_subject_type(&q.subject_type) {
        return Err(StatusCode::BAD_REQUEST);
    }
    require_role(&state.pool, &user, q.engagement_id, EngagementRole::Viewer).await?;

    let notes = sqlx::query_as::<_, Note>(&format!(
        "{NOTE_SELECT} WHERE engagement_id = $1 AND subject_type = $2::note_subject_type AND subject_id = $3
         ORDER BY created_at"
    ))
    .bind(q.engagement_id)
    .bind(&q.subject_type)
    .bind(q.subject_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(notes))
}

async fn create_note(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<Json<Note>, StatusCode> {
    if !valid_subject_type(&payload.subject_type) {
        return Err(StatusCode::BAD_REQUEST);
    }
    require_role(&state.pool, &user, payload.engagement_id, EngagementRole::Tester).await?;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO notes (engagement_id, subject_type, subject_id, title, body_md, created_by)
         VALUES ($1, $2::note_subject_type, $3, $4, $5, $6) RETURNING id",
    )
    .bind(payload.engagement_id)
    .bind(&payload.subject_type)
    .bind(payload.subject_id)
    .bind(&payload.title)
    .bind(&payload.body_md)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let note = sqlx::query_as::<_, Note>(&format!("{NOTE_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(note))
}

async fn update_note(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateNoteRequest>,
) -> Result<Json<Note>, StatusCode> {
    let engagement_id = note_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("UPDATE notes SET title = $1, body_md = $2 WHERE id = $3")
        .bind(&payload.title)
        .bind(&payload.body_md)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let note = sqlx::query_as::<_, Note>(&format!("{NOTE_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(note))
}

async fn delete_note(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = note_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notes", axum::routing::get(list_notes).post(create_note))
        .route("/notes/{id}", axum::routing::put(update_note).delete(delete_note))
}
