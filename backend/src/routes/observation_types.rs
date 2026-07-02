use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::state::AppState;

fn normalize_json_array(v: Value) -> Value {
    if v.is_null() {
        serde_json::json!([])
    } else {
        v
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ObservationType {
    id: Uuid,
    key: String,
    title: String,
    category: String,
    default_severity: String,
    description_md: String,
    references_json: Value,
    mitre_technique_ids: Value,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ObservationTypeRequest {
    key: String,
    title: String,
    category: String,
    default_severity: String,
    #[serde(default)]
    description_md: String,
    #[serde(default)]
    references_json: Value,
    #[serde(default)]
    mitre_technique_ids: Value,
}

const OBSERVATION_TYPE_SELECT: &str = "SELECT id, key, title, category, default_severity,
    description_md, references_json, mitre_technique_ids, created_at FROM observation_types";

async fn list_observation_types(
    State(state): State<AppState>,
) -> Result<Json<Vec<ObservationType>>, StatusCode> {
    let types = sqlx::query_as::<_, ObservationType>(&format!(
        "{OBSERVATION_TYPE_SELECT} ORDER BY category, title"
    ))
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(types))
}

async fn create_observation_type(
    State(state): State<AppState>,
    Json(payload): Json<ObservationTypeRequest>,
) -> Result<Json<ObservationType>, StatusCode> {
    let references_json = normalize_json_array(payload.references_json);
    let mitre_technique_ids = normalize_json_array(payload.mitre_technique_ids);

    let obs_type = sqlx::query_as::<_, ObservationType>(
        "INSERT INTO observation_types (key, title, category, default_severity, description_md, references_json, mitre_technique_ids)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, key, title, category, default_severity, description_md, references_json, mitre_technique_ids, created_at",
    )
    .bind(&payload.key)
    .bind(&payload.title)
    .bind(&payload.category)
    .bind(&payload.default_severity)
    .bind(&payload.description_md)
    .bind(&references_json)
    .bind(&mitre_technique_ids)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    Ok(Json(obs_type))
}

async fn get_observation_type(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ObservationType>, StatusCode> {
    let obs_type = sqlx::query_as::<_, ObservationType>(&format!(
        "{OBSERVATION_TYPE_SELECT} WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(obs_type))
}

async fn update_observation_type(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ObservationTypeRequest>,
) -> Result<Json<ObservationType>, StatusCode> {
    let references_json = normalize_json_array(payload.references_json);
    let mitre_technique_ids = normalize_json_array(payload.mitre_technique_ids);

    let obs_type = sqlx::query_as::<_, ObservationType>(
        "UPDATE observation_types SET key = $1, title = $2, category = $3, default_severity = $4,
         description_md = $5, references_json = $6, mitre_technique_ids = $7
         WHERE id = $8
         RETURNING id, key, title, category, default_severity, description_md, references_json, mitre_technique_ids, created_at",
    )
    .bind(&payload.key)
    .bind(&payload.title)
    .bind(&payload.category)
    .bind(&payload.default_severity)
    .bind(&payload.description_md)
    .bind(&references_json)
    .bind(&mitre_technique_ids)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(obs_type))
}

async fn delete_observation_type(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM observation_types WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_foreign_key_violation() => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/observation-types",
            get(list_observation_types).post(create_observation_type),
        )
        .route(
            "/observation-types/{id}",
            get(get_observation_type)
                .put(update_observation_type)
                .delete(delete_observation_type),
        )
}
