use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;

use crate::state::AppState;

use super::session::validate_session;
use super::{CurrentUser, SESSION_COOKIE};

pub async fn require_auth(
    State(state): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = jar
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = validate_session(&state.pool, &token)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(CurrentUser {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        is_admin: user.is_admin,
    });

    Ok(next.run(req).await)
}
