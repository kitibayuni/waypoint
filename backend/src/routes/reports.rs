use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::report::render_html;
use crate::routes::common::{OptionExt, ResultExt};
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

    if q.format.as_deref() != Some("pdf") {
        return Ok((
            [(header::CONTENT_TYPE, "text/html; charset=utf-8".to_string())],
            html,
        )
            .into_response());
    }

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
    stdin
        .write_all(html.as_bytes())
        .await
        .internal()?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .internal()?;

    if !output.status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"report.pdf\"".to_string(),
            ),
        ],
        output.stdout,
    )
        .into_response())
}

pub fn router() -> Router<AppState> {
    Router::new().route("/reports/{engagement_id}", axum::routing::get(get_report))
}
