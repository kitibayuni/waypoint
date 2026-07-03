pub mod middleware;
pub mod password;
pub mod rate_limit;
pub mod routes;
pub mod session;

use serde::Serialize;
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "session_token";

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
}
