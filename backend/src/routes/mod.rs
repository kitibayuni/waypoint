pub mod attachments;
pub mod audit;
pub mod checklists;
pub mod clients;
pub mod credential_usage;
pub mod credentials;
pub mod dashboard;
pub mod engagements;
pub mod findings;
pub mod graph;
pub mod hosts;
pub mod import;
pub mod members;
pub mod mitre;
pub mod node_positions;
pub mod notes;
pub mod reports;
pub mod scope;
pub mod search;
pub mod services;
pub mod timeline;
pub mod trust_relationships;

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
        .merge(credentials::router())
        .merge(credential_usage::router())
        .merge(trust_relationships::router())
        .merge(graph::router())
        .merge(node_positions::router())
        .merge(checklists::router())
        .merge(notes::router())
        .merge(findings::router())
        .merge(attachments::router())
        .merge(search::router())
        .merge(dashboard::router())
        .merge(reports::router())
        .merge(import::router())
        .merge(audit::router())
        .merge(mitre::router())
        .merge(timeline::router())
}
