//! Tiny error-mapping helpers used everywhere a DB/IO call needs to become
//! an HTTP status: collapsing an error into a 500, or a missing row into a
//! 404, without repeating the same closure at every call site.

use axum::http::StatusCode;

/// Collapses any error into a 500 -- every call site already discards the
/// underlying error (`|_| StatusCode::INTERNAL_SERVER_ERROR`), so this just
/// gives that pattern a name instead of repeating the closure everywhere.
pub(crate) trait ResultExt<T> {
    fn internal(self) -> Result<T, StatusCode>;
}

impl<T, E> ResultExt<T> for Result<T, E> {
    fn internal(self) -> Result<T, StatusCode> {
        self.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// `Option<T> -> Result<T, StatusCode>` for the equally common "this row
/// doesn't exist" case.
pub(crate) trait OptionExt<T> {
    fn or_404(self) -> Result<T, StatusCode>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_404(self) -> Result<T, StatusCode> {
        self.ok_or(StatusCode::NOT_FOUND)
    }
}
