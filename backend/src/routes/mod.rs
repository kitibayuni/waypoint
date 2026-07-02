pub mod clients;
pub mod engagements;
pub mod members;
pub mod scope;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(clients::router())
        .merge(engagements::router())
        .merge(scope::router())
        .merge(members::router())
}
