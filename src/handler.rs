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
use x402_rs::{
    network::Network,
    types::{PaymentPayload, SettleResponse},
};

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
    Extension(settlement): Extension<Option<SettleResponse>>,
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(create): Json<CreateLinkRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(url = %create.url, "handle_x402_create");

    // Extract payment details from the X-Payment header
    let payment = headers
        .get("x-payment")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            let base64_bytes = x402_rs::types::Base64Bytes(Cow::Borrowed(s.as_bytes()));
            PaymentPayload::try_from(base64_bytes).ok()
        })
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid X-Payment header"))?;

    // Extract payment amount and addresses from EVM payload
    let (amount, from, to) = match &payment.payload {
        x402_rs::types::ExactPaymentPayload::Evm(evm_payload) => {
            let amount = evm_payload.authorization.value.0.to_string();
            let from = evm_payload.authorization.from.to_string();
            let to = evm_payload.authorization.to.to_string();
            (amount, from, to)
        }
        x402_rs::types::ExactPaymentPayload::Solana(_) => {
            return Err(anyhow::anyhow!("Solana payments are not supported").into());
        }
    };

    // Extract transaction hash from the settlement extension (error if missing)
    let tx_hash = settlement
        .and_then(|s| s.transaction)
        .map(|tx| tx.to_string())
        .ok_or_else(|| anyhow::anyhow!("No transaction hash in settlement"))?;

    info!(
        network = ?payment.network,
        amount = %amount,
        from = %from,
        to = %to,
        tx_hash = %tx_hash,
        "x402 payment details"
    );

    let is_testnet = payment.network == Network::BaseSepolia;

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

        // Create a mock settlement response with a transaction hash
        let mock_settlement = SettleResponse {
            success: true,
            error_reason: None,
            payer: x402_rs::types::MixedAddress::from(
                "0x0000000000000000000000000000000000000000"
                    .parse::<x402_rs::types::EvmAddress>()
                    .unwrap(),
            ),
            transaction: Some(x402_rs::types::TransactionHash::Evm([0x12; 32])),
            network: Network::BaseSepolia,
        };

        // Call the handler
        let result = handle_x402_create(
            Extension(Some(mock_settlement)),
            State(app),
            headers,
            Json(request),
        )
        .await;

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
    async fn test_handle_x402_create_without_payment_header() {
        // Create a mock app
        let db = MockLinksDB::new();

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

        // Call the handler - should fail without X-Payment header
        let result = handle_x402_create(Extension(None), State(app), headers, Json(request)).await;

        assert!(result.is_err());
    }
}
