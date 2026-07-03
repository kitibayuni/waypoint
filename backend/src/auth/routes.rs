use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use time::Duration as CookieDuration;

use crate::state::AppState;

use super::password::verify_password;
use super::session::{create_session, delete_session};
use super::{CurrentUser, SESSION_COOKIE};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: uuid::Uuid,
    email: String,
    display_name: String,
    password_hash: String,
    is_admin: bool,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<CurrentUser>), StatusCode> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, display_name, password_hash, is_admin FROM users WHERE email = $1",
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_password(&payload.password, &row.password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let session = create_session(&state.pool, row.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let max_age =
        CookieDuration::seconds((session.expires_at - chrono::Utc::now()).num_seconds().max(0));
    // Strict is sufficient CSRF protection here (no cross-site login flows to support)
    // and avoids needing a separate CSRF token scheme.
    let cookie = Cookie::build((SESSION_COOKIE, session.raw_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(max_age)
        .build();

    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(CurrentUser {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            is_admin: row.is_admin,
        }),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> CookieJar {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        delete_session(&state.pool, cookie.value()).await;
    }
    jar.remove(Cookie::build(SESSION_COOKIE).path("/").build())
}

pub async fn me(Extension(user): Extension<CurrentUser>) -> Json<CurrentUser> {
    Json(user)
}
