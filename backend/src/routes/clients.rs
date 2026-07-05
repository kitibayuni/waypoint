use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::routes::common::{OptionExt, ResultExt};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Client {
    id: Uuid,
    name: String,
    contacts: Value,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ClientRequest {
    name: String,
    #[serde(default)]
    contacts: Value,
    notes: Option<String>,
}

fn normalize_contacts(v: Value) -> Value {
    if v.is_null() {
        serde_json::json!([])
    } else {
        v
    }
}

async fn list_clients(State(state): State<AppState>) -> Result<Json<Vec<Client>>, StatusCode> {
    let clients = sqlx::query_as::<_, Client>(
        "SELECT id, name, contacts, notes, created_at FROM clients ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await
    .internal()?;
    Ok(Json(clients))
}

async fn create_client(
    State(state): State<AppState>,
    Json(payload): Json<ClientRequest>,
) -> Result<Json<Client>, StatusCode> {
    let contacts = normalize_contacts(payload.contacts);
    let client = sqlx::query_as::<_, Client>(
        "INSERT INTO clients (name, contacts, notes) VALUES ($1, $2, $3)
         RETURNING id, name, contacts, notes, created_at",
    )
    .bind(&payload.name)
    .bind(&contacts)
    .bind(&payload.notes)
    .fetch_one(&state.pool)
    .await
    .internal()?;
    Ok(Json(client))
}

async fn get_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Client>, StatusCode> {
    let client = sqlx::query_as::<_, Client>(
        "SELECT id, name, contacts, notes, created_at FROM clients WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .internal()?
    .or_404()?;
    Ok(Json(client))
}

async fn update_client(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ClientRequest>,
) -> Result<Json<Client>, StatusCode> {
    let contacts = normalize_contacts(payload.contacts);
    let client = sqlx::query_as::<_, Client>(
        "UPDATE clients SET name = $1, contacts = $2, notes = $3 WHERE id = $4
         RETURNING id, name, contacts, notes, created_at",
    )
    .bind(&payload.name)
    .bind(&contacts)
    .bind(&payload.notes)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .internal()?
    .or_404()?;
    Ok(Json(client))
}

async fn delete_client(State(state): State<AppState>, Path(id): Path<Uuid>) -> StatusCode {
    let result = sqlx::query("DELETE FROM clients WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await;
    match result {
        Ok(r) if r.rows_affected() == 0 => StatusCode::NOT_FOUND,
        Ok(_) => StatusCode::NO_CONTENT,
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => StatusCode::CONFLICT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/clients", get(list_clients).post(create_client))
        .route(
            "/clients/{id}",
            get(get_client).put(update_client).delete(delete_client),
        )
}
