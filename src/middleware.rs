//! App middleware functions.

use axum::{extract::Request, middleware::Next, response::Response};
use log::{debug, warn};
use once_cell::sync::Lazy;
use std::collections::HashSet;

static IGNORE_PATHS: Lazy<HashSet<&str>> = Lazy::new(|| HashSet::from(["/favicon.ico"]));

/// Simple logging middleware.
///
/// Logs the method, path, and response code to debug
/// if processing returned a successful code, and to
/// warn otherwise.
pub async fn logging(request: Request, next: Next) -> Response {
    let uri = request.uri().clone();
    let path = uri.path();
    if !IGNORE_PATHS.contains(path) {
        let method = request.method().clone();
        let response = next.run(request).await;
        let s = format!("{} {} {}", method, path, response.status().as_u16());
        if response.status().is_success() || response.status().is_redirection() {
            debug!("{s}");
        } else {
            warn!("{s}");
        }
        response
    } else {
        next.run(request).await
    }
}
