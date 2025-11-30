use crate::{app::App, auth::AuthenticatedKey};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_turnstile::VerifiedTurnstile;
use ezlime_rs::{CreateLinkRequest, CreatedLinkResponse};
use std::sync::Arc;
use tracing::info;
use x402_rs::{network::Network, types::PaymentPayload};

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub async fn handle_health() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

pub async fn handle_redirect(
    Path(id): Path<String>,
    State(app): State<Arc<App>>,
) -> Result<impl IntoResponse, AppError> {
    info!("handle_redirect: {}", id);

    let url = app.redirect(&id).await?;

    Ok(Redirect::temporary(&url))
}

pub async fn handle_create(
    Extension(AuthenticatedKey(api_key)): Extension<AuthenticatedKey>,
    State(app): State<Arc<App>>,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(api_key, "handle_create: '{}'", create.url);

    Ok(Json(app.create_link(api_key, create).await?).into_response())
}

pub async fn handle_public_create(
    _verified: VerifiedTurnstile,
    State(app): State<Arc<App>>,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("handle_public_create: '{}'", create.url);

    Ok(Json(app.create_link("public".to_string(), create).await?).into_response())
}

/// x402-powered link shortening endpoint
/// Payment enforcement is handled by x402-axum middleware
/// Supports Base mainnet and Base Sepolia USDC payments
pub async fn handle_x402_create(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(url = %create.url, "x402 payment received, creating link");

    // Check if payment was made on Sepolia by inspecting the X-Payment header
    if let Some(payment_header) = headers.get("x-payment") {
        if let Ok(payment_str) = payment_header.to_str() {
            // Decode the base64-encoded payment payload
            let base64_bytes =
                x402_rs::types::Base64Bytes(std::borrow::Cow::Borrowed(payment_str.as_bytes()));
            if let Ok(payment) = PaymentPayload::try_from(base64_bytes) {
                // Check if the network is BaseSepolia
                if payment.network == Network::BaseSepolia {
                    info!("Detected Sepolia testnet payment, returning demo response");
                    let demo_response = CreatedLinkResponse::new(
                        "demo123".to_string(),
                        "https://example.com",
                        create.url.clone(),
                    );
                    return Ok(Json(demo_response).into_response());
                }
            }
        }
    }

    let response = app.create_link("x402".to_string(), create).await?;
    Ok(Json(response).into_response())
}
