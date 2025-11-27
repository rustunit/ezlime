use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use tracing::info;

// Store your API keys
#[derive(Clone)]
pub struct ApiKeys {
    keys: Arc<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedKey(pub String);

impl ApiKeys {
    pub fn new(keys: &str) -> Self {
        let keys: Vec<String> = keys.split(',').map(|s| s.trim().to_string()).collect();

        info!("keys configured: {}", keys.len());

        Self {
            keys: Arc::new(keys),
        }
    }

    pub fn is_valid(&self, key: &str) -> bool {
        self.keys.contains(&key.to_string())
    }
}

pub async fn require_auth(
    State(api_keys): State<ApiKeys>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(key) if api_keys.is_valid(key) => {
            let key = key.to_string();
            request.extensions_mut().insert(AuthenticatedKey(key));
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
