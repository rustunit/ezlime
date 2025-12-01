use crate::{app::App, auth::AuthenticatedKey};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_turnstile::VerifiedTurnstile;
use ezlime_rs::CreateLinkRequest;
use std::{borrow::Cow, sync::Arc};
use tracing::info;
use x402_rs::{network::Network, types::PaymentPayload};

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
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

    Ok(Json(app.create_link(api_key, create, false).await?).into_response())
}

pub async fn handle_public_create(
    _verified: VerifiedTurnstile,
    State(app): State<Arc<App>>,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!("handle_public_create: '{}'", create.url);

    Ok(Json(app.create_link("public".to_string(), create, false).await?).into_response())
}

pub async fn handle_x402_create(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(url = %create.url, "handle_x402_create");

    // Check if payment was made on Sepolia by inspecting the X-Payment header
    let is_testnet = headers
        .get("x-payment")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            let base64_bytes = x402_rs::types::Base64Bytes(Cow::Borrowed(s.as_bytes()));
            PaymentPayload::try_from(base64_bytes).ok()
        })
        .map(|payment| payment.network == Network::BaseSepolia)
        .unwrap_or(false);

    let response = app
        .create_link("x402".to_string(), create, is_testnet)
        .await?;

    Ok(Json(response).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{counter::ClickCounter, db::MockLinksDB};
    use axum::http::HeaderValue;
    use ezlime_rs::CreatedLinkResponse;
    use x402_rs::types::{ExactEvmPayload, ExactEvmPayloadAuthorization, ExactPaymentPayload};

    #[tokio::test]
    async fn test_handle_x402_create_sepolia_returns_demo() {
        // Create a mock app
        let db = MockLinksDB::new();
        let app = App::new(
            "http://localhost:8080".to_string(),
            6,
            Arc::new(db),
            Arc::new(ClickCounter::new()),
            10,
        );

        // Create a PaymentPayload for BaseSepolia
        let payment = PaymentPayload {
            x402_version: x402_rs::types::X402Version::V1,
            scheme: x402_rs::types::Scheme::Exact,
            network: Network::BaseSepolia,
            payload: ExactPaymentPayload::Evm(ExactEvmPayload {
                signature: x402_rs::types::EvmSignature(vec![0u8; 65]),
                authorization: ExactEvmPayloadAuthorization {
                    from: "0x0000000000000000000000000000000000000000"
                        .parse()
                        .unwrap(),
                    to: "0x0000000000000000000000000000000000000000"
                        .parse()
                        .unwrap(),
                    value: x402_rs::types::TokenAmount(
                        x402_rs::__reexports::alloy::primitives::U256::from(1000000),
                    ),
                    valid_after: x402_rs::timestamp::UnixTimestamp(0),
                    valid_before: x402_rs::timestamp::UnixTimestamp(u64::MAX),
                    nonce: x402_rs::types::HexEncodedNonce([0u8; 32]),
                },
            }),
        };

        // Encode the payment as base64 JSON
        let payment_json = serde_json::to_string(&payment).unwrap();
        let payment_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            payment_json.as_bytes(),
        );

        // Create headers with the X-Payment header
        let mut headers = HeaderMap::new();
        headers.insert("x-payment", HeaderValue::from_str(&payment_base64).unwrap());

        // Create the request
        let test_url = "https://example.com/test".to_string();
        let request = CreateLinkRequest {
            url: test_url.clone(),
        };

        // Call the handler
        let result = handle_x402_create(State(app), headers, Json(request)).await;

        assert!(result.is_ok());

        // Extract the response
        let response = result.unwrap().into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_data: CreatedLinkResponse = serde_json::from_slice(&body_bytes).unwrap();

        // Verify it's the demo response
        assert_eq!(response_data.id, "rustunit");
        assert_eq!(
            response_data.shortened_url,
            "http://localhost:8080/rustunit"
        );
        assert_eq!(response_data.original_url, test_url);
    }

    #[tokio::test]
    async fn test_handle_x402_create_without_header() {
        // Create a mock app that expects create_link to be called
        let mut db = MockLinksDB::new();
        db.expect_create().times(1).returning(|_| {
            Ok(crate::models::CreateLink {
                id: "test123".to_string(),
                url: "https://example.com/test".to_string(),
                key: "x402".to_string(),
            })
        });
        db.expect_get().times(0).returning(|_| Ok(None));

        let app = App::new(
            "http://localhost:8080".to_string(),
            6,
            Arc::new(db),
            Arc::new(ClickCounter::new()),
            10,
        );

        // Create headers without X-Payment header
        let headers = HeaderMap::new();

        // Create the request
        let request = CreateLinkRequest {
            url: "https://example.com/test".to_string(),
        };

        // Call the handler
        let result = handle_x402_create(State(app), headers, Json(request)).await;

        assert!(result.is_ok());

        // Extract the response
        let response = result.unwrap().into_response();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_data: CreatedLinkResponse = serde_json::from_slice(&body_bytes).unwrap();

        // Verify it's a real link (not demo)
        // The actual ID will be a hash of the URL, not "test123"
        assert_ne!(response_data.id, "rustunit", "Should not be demo response");
        assert_eq!(
            response_data.id, "cuwckd",
            "Should be the computed hash of the URL"
        );
    }
}
