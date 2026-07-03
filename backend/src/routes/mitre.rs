use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct MitreTechnique {
    id: String,
    name: String,
    tactic: Option<String>,
    url: Option<String>,
}

/// Unscoped reference lookup, like templates -- any authenticated user can
/// view it.
async fn list_mitre_techniques(
    State(state): State<AppState>,
) -> Result<Json<Vec<MitreTechnique>>, StatusCode> {
    let techniques = sqlx::query_as::<_, MitreTechnique>(
        "SELECT id, name, tactic, url FROM mitre_techniques ORDER BY id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(techniques))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/mitre-techniques", get(list_mitre_techniques))
}
