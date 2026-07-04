use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct NodePosition {
    node_id: String,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
pub struct NodePositionUpsert {
    node_id: String,
    x: f64,
    y: f64,
}

async fn list_node_positions(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<NodePosition>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let rows = sqlx::query_as::<_, NodePosition>(
        "SELECT node_id, x, y FROM node_positions WHERE engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

async fn upsert_node_positions(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<Vec<NodePositionUpsert>>,
) -> Result<StatusCode, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for p in &payload {
        sqlx::query(
            "INSERT INTO node_positions (engagement_id, node_id, x, y)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (engagement_id, node_id)
             DO UPDATE SET x = excluded.x, y = excluded.y, updated_at = now()",
        )
        .bind(engagement_id)
        .bind(&p.node_id)
        .bind(p.x)
        .bind(p.y)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/engagements/{engagement_id}/node-positions",
        get(list_node_positions).put(upsert_node_positions),
    )
}
