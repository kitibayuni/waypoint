use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::search::{search, SearchResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    types: Option<String>,
}

async fn search_engagement(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    if query.q.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let types: Vec<String> = query
        .types
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let results = search(&state.pool, engagement_id, &query.q, &types)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/engagements/{engagement_id}/search",
        axum::routing::get(search_engagement),
    )
}
