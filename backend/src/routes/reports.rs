use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::report::render_html;
use crate::routes::common::{scoped_engagement_id, OptionExt, ResultExt};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ReportQuery {
    format: Option<String>,
}

async fn get_report(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> Result<Response, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let html = render_html(&state.pool, engagement_id)
        .await
        .internal()?
        .or_404()?;

    respond_with(html, q.format.as_deref()).await
}

/// Shared by `get_report` and `get_snapshot`: serves HTML directly, or pipes
/// it through wkhtmltopdf when `?format=pdf` is requested.
async fn respond_with(html: String, format: Option<&str>) -> Result<Response, StatusCode> {
    if format != Some("pdf") {
        return Ok((
            [(header::CONTENT_TYPE, "text/html; charset=utf-8".to_string())],
            html,
        )
            .into_response());
    }

    let pdf = render_pdf(&html).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"report.pdf\"".to_string(),
            ),
        ],
        pdf,
    )
        .into_response())
}

async fn render_pdf(html: &str) -> Result<Vec<u8>, StatusCode> {
    // --disable-javascript: notes/findings content is rendered as HTML in
    // the report, so this keeps an embedded <script> in someone's markdown
    // from executing during PDF rendering.
    let mut child = Command::new("wkhtmltopdf")
        .args(["--disable-javascript", "--quiet", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .internal()?;

    let mut stdin = child.stdin.take().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    stdin.write_all(html.as_bytes()).await.internal()?;
    drop(stdin);

    let output = child.wait_with_output().await.internal()?;
    if !output.status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(output.stdout)
}

async fn snapshot_engagement_id(pool: &PgPool, id: Uuid) -> Result<Uuid, StatusCode> {
    scoped_engagement_id(pool, "SELECT engagement_id FROM report_snapshots WHERE id = $1", id).await
}

/// A point-in-time capture of a rendered report, so a tester can lock in
/// what was actually delivered to a client even as findings keep changing
/// afterward (e.g. during retest). `html_body` is deliberately excluded from
/// this list-shaped struct -- it can be tens of KB per snapshot and the list
/// view doesn't need it, only `get_snapshot` does.
#[derive(Serialize, sqlx::FromRow)]
pub struct ReportSnapshot {
    id: Uuid,
    engagement_id: Uuid,
    generated_at: DateTime<Utc>,
    status: String,
    generated_by_name: Option<String>,
}

const SNAPSHOT_SELECT: &str = "SELECT rs.id, rs.engagement_id, rs.generated_at, rs.status::text AS status,
    u.display_name AS generated_by_name
    FROM report_snapshots rs
    LEFT JOIN users u ON u.id = rs.generated_by";

async fn list_snapshots(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<ReportSnapshot>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let snapshots = sqlx::query_as::<_, ReportSnapshot>(&format!(
        "{SNAPSHOT_SELECT} WHERE rs.engagement_id = $1 ORDER BY rs.generated_at DESC"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(snapshots))
}

async fn create_snapshot(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<ReportSnapshot>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let html = render_html(&state.pool, engagement_id)
        .await
        .internal()?
        .or_404()?;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO report_snapshots (engagement_id, html_body, generated_by)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(engagement_id)
    .bind(&html)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .internal()?;

    let snapshot = sqlx::query_as::<_, ReportSnapshot>(&format!("{SNAPSHOT_SELECT} WHERE rs.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(snapshot))
}

async fn get_snapshot(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((_engagement_id, snapshot_id)): Path<(Uuid, Uuid)>,
    Query(q): Query<ReportQuery>,
) -> Result<Response, StatusCode> {
    let engagement_id = snapshot_engagement_id(&state.pool, snapshot_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let (html,): (String,) = sqlx::query_as("SELECT html_body FROM report_snapshots WHERE id = $1")
        .bind(snapshot_id)
        .fetch_optional(&state.pool)
        .await
        .internal()?
        .or_404()?;

    respond_with(html, q.format.as_deref()).await
}

async fn mark_snapshot_final(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((_engagement_id, snapshot_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReportSnapshot>, StatusCode> {
    let engagement_id = snapshot_engagement_id(&state.pool, snapshot_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    // The snapshot is confirmed to exist by `snapshot_engagement_id` above,
    // so 0 rows affected here unambiguously means it was already final.
    let result = sqlx::query(
        "UPDATE report_snapshots SET status = 'final'::report_snapshot_status
         WHERE id = $1 AND status = 'draft'",
    )
    .bind(snapshot_id)
    .execute(&state.pool)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::CONFLICT);
    }

    let snapshot = sqlx::query_as::<_, ReportSnapshot>(&format!("{SNAPSHOT_SELECT} WHERE rs.id = $1"))
        .bind(snapshot_id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(snapshot))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/reports/{engagement_id}", axum::routing::get(get_report))
        .route(
            "/reports/{engagement_id}/snapshots",
            axum::routing::get(list_snapshots).post(create_snapshot),
        )
        .route(
            "/reports/{engagement_id}/snapshots/{snapshot_id}",
            axum::routing::get(get_snapshot),
        )
        .route(
            "/reports/{engagement_id}/snapshots/{snapshot_id}/finalize",
            axum::routing::patch(mark_snapshot_final),
        )
}
