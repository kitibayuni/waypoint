use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::graph::build_graph;
use crate::routes::common::ResultExt;
use crate::state::AppState;

#[derive(Deserialize)]
struct GraphQuery {
    as_of: Option<DateTime<Utc>>,
}

async fn get_graph(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Query(query): Query<GraphQuery>,
) -> Result<Json<Value>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let graph = build_graph(&state.pool, engagement_id, query.as_of)
        .await
        .internal()?;

    Ok(Json(graph))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/engagements/{engagement_id}/graph", get(get_graph))
}
