//! App middleware functions.

use axum::{extract::Request, middleware::Next, response::Response};
use log::{debug, warn};

/// Simple logging middleware.
///
/// Logs the method, path, and response code to debug
/// if processing returned a successful code, and to
/// warn otherwise.
pub async fn logging(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let s = format!("{} {} {}", method, uri.path(), response.status().as_u16());
    if response.status().is_success() {
        debug!("{s}");
    } else {
        warn!("{s}");
    }
    response
}
