use crate::{
    app::App,
    auth::AuthenticatedKey,
    x402::{
        FacilitatorClient, FacilitatorPaymentRequirement, PaymentRequiredResponse,
        PaymentRequirement, parse_payment_header,
    },
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_turnstile::VerifiedTurnstile;
use ezlime_rs::CreateLinkRequest;
use std::sync::Arc;
use tracing::{error, info};

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
/// Requires payment via X-PAYMENT header
pub async fn handle_x402_create(
    State(app): State<Arc<App>>,
    Extension(facilitator): Extension<Arc<FacilitatorClient>>,
    Extension(x402_config): Extension<Arc<X402Config>>,
    headers: HeaderMap,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Check if payment header was provided
    let payment_header = headers.get("X-PAYMENT");

    let payment_payload = match payment_header {
        Some(header_value) => {
            // Parse the payment header
            let header_str = header_value
                .to_str()
                .map_err(|_| anyhow::anyhow!("Invalid X-PAYMENT header format"))?;

            parse_payment_header(header_str)
                .map_err(|_| anyhow::anyhow!("Failed to parse payment payload"))?
        }
        None => {
            // No payment provided, return 402 with payment requirements
            info!(url = %create.url, "x402: payment required");

            let payment_required = PaymentRequiredResponse {
                x402_version: 1,
                accepts: vec![PaymentRequirement {
                    scheme: "exact".to_string(),
                    network: x402_config.network.clone(),
                    max_amount_required: x402_config.price_per_link.clone(),
                    asset: x402_config.asset_address.clone(),
                    pay_to: x402_config.merchant_wallet.clone(),
                    resource: Some("/x402/shorten".to_string()),
                    description: Some("Link shortening service".to_string()),
                    max_timeout_seconds: Some(60),
                }],
            };

            return Ok((StatusCode::PAYMENT_REQUIRED, Json(payment_required)).into_response());
        }
    };

    info!(
        url = %create.url,
        from = %payment_payload.payload.authorization.from,
        to = %payment_payload.payload.authorization.to,
        value = %payment_payload.payload.authorization.value,
        network = %payment_payload.network,
        "x402: processing payment"
    );

    // Build payment requirements for facilitator
    let payment_requirements = FacilitatorPaymentRequirement {
        scheme: "exact".to_string(),
        network: x402_config.network.clone(),
        max_amount_required: x402_config.price_per_link.clone(),
        pay_to: x402_config.merchant_wallet.clone(),
        asset: x402_config.asset_address.clone(),
        mime_type: "application/json".to_string(),
        resource: Some(format!("{}/x402/shorten", create.url)),
        description: Some("Link shortening service".to_string()),
        max_timeout_seconds: Some(60),
    };

    // Verify and settle the payment
    match facilitator
        .verify_and_settle(&payment_payload, &payment_requirements)
        .await
    {
        Ok(settlement) => {
            info!(
                tx_hash = %settlement.transaction,
                payer = %settlement.payer,
                "x402: payment settled successfully"
            );

            // Payment successful, create the link
            let response = app.create_link("x402".to_string(), create).await?;

            Ok(Json(response).into_response())
        }
        Err(e) => {
            error!(
                error = %e,
                from = %payment_payload.payload.authorization.from,
                "x402: payment settlement failed"
            );

            // Return 402 again so client can retry
            let payment_required = PaymentRequiredResponse {
                x402_version: 1,
                accepts: vec![PaymentRequirement {
                    scheme: "exact".to_string(),
                    network: x402_config.network.clone(),
                    max_amount_required: x402_config.price_per_link.clone(),
                    asset: x402_config.asset_address.clone(),
                    pay_to: x402_config.merchant_wallet.clone(),
                    resource: Some("/x402/shorten".to_string()),
                    description: Some("Link shortening service".to_string()),
                    max_timeout_seconds: Some(60),
                }],
            };

            Ok((StatusCode::PAYMENT_REQUIRED, Json(payment_required)).into_response())
        }
    }
}

/// Configuration for x402 payment endpoint
#[derive(Debug, Clone)]
pub struct X402Config {
    pub network: String,
    pub price_per_link: String,
    pub asset_address: String,
    pub merchant_wallet: String,
}
