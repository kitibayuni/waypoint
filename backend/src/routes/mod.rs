pub mod clients;
pub mod credential_usage;
pub mod credentials;
pub mod engagements;
pub mod hosts;
pub mod members;
pub mod observation_types;
pub mod observations;
pub mod scope;
pub mod services;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(clients::router())
        .merge(engagements::router())
        .merge(scope::router())
        .merge(members::router())
        .merge(hosts::router())
        .merge(services::router())
        .merge(observation_types::router())
        .merge(observations::router())
        .merge(credentials::router())
        .merge(credential_usage::router())
}
