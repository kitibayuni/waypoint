use std::sync::Arc;

use axum::routing::{get, post};
use axum::{middleware, Router};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;

use backend::auth;
use backend::crypto::CredentialCipher;
use backend::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    tracing::info!("migrations applied");

    let cred_enc_key = std::env::var("CRED_ENC_KEY").expect("CRED_ENC_KEY must be set");
    let cred_cipher = Arc::new(CredentialCipher::from_env_key(&cred_enc_key));

    let attachments_dir: std::path::PathBuf = std::env::var("ATTACHMENTS_DIR")
        .unwrap_or_else(|_| "./data/attachments".into())
        .into();
    tokio::fs::create_dir_all(&attachments_dir)
        .await
        .expect("failed to create attachments directory");

    let state = AppState {
        pool,
        cred_cipher,
        attachments_dir,
    };

    let login_limiter = auth::rate_limit::LoginRateLimiter::new();
    let login_route = Router::new()
        .route("/auth/login", post(auth::routes::login))
        .layer(middleware::from_fn(move |req, next| {
            let limiter = login_limiter.clone();
            async move { auth::rate_limit::rate_limit_login(limiter, req, next).await }
        }))
        .with_state(state.clone());

    let public_routes = Router::new()
        .route("/healthz", get(healthz))
        .merge(login_route);

    let protected_routes = Router::new()
        .route("/auth/logout", post(auth::routes::logout))
        .route("/auth/me", get(auth::routes::me))
        .merge(backend::routes::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::require_auth,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("BACKEND_PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn healthz() -> &'static str {
    "ok"
}
