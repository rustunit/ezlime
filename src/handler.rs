use crate::{app::App, auth::AuthenticatedKey};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_turnstile::VerifiedTurnstile;
use ezlime_rs::CreateLinkRequest;
use std::sync::Arc;
use tracing::info;
use x402_types::{
    proto::v1::{PaymentPayload, SettleResponse},
    util::b64::Base64Bytes,
    chain::ChainId,
    networks::chain_id_by_network_name, 
};
use x402_chain_eip155::{v1_eip155_exact::ExactEvmPayload};

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
    let payment: PaymentPayload = headers
        .get("x-payment")
        .ok_or_else(|| anyhow::anyhow!("Missing X-Payment header"))
        .and_then(|header| {
            let decoded = Base64Bytes::from(header.as_bytes())
                .decode()
                .map_err(|_| anyhow::anyhow!("Invalid X-Payment header"))?;

            serde_json::from_slice(decoded.as_ref())
                .map_err(|_| anyhow::anyhow!("Invalid X-Payment header"))
        })?;



    let chain_id: ChainId = payment.network.parse()
                .map_err(|_| anyhow::anyhow!("Invalid network"))?;

    let (amount, from, to) = match chain_id.namespace.as_str() {
        "eip155" => {
            let evm_payload: ExactEvmPayload = 
            serde_json::from_str(payment.payload.get())
                .map_err(|_| anyhow::anyhow!("Invalid EVM payload"))?;
            (
                evm_payload.authorization.value.to_string(),
                evm_payload.authorization.from.to_string(),
                evm_payload.authorization.to.to_string(),
            )
        },
        "solana" => {
            return Err(anyhow::anyhow!("Solana payments are not supported").into());
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported network").into());
        }
    };

    // Extract transaction hash from the settlement extension (error if missing)
    let tx_hash = settlement
        .ok_or(anyhow::anyhow!("No transaction hash in settlement"))
        .and_then(|s| 
            if let SettleResponse::Success { transaction: t, payer: _, network: _ } = s {
                Ok(t)
            } else {
                Err(anyhow::anyhow!("No transaction hash in settlement"))
            }
        )?;

    info!(
        network = ?payment.network,
        chain_id = ?chain_id,
        amount = %amount,
        from = %from,
        to = %to,
        tx_hash = %tx_hash,
        "x402 payment details"
    );

    let is_testnet = &chain_id == chain_id_by_network_name("base-sepolia").unwrap();

    let response = app
        .create_link("x402".to_string(), create, is_testnet)
        .await?;

    app.store_transaction(response.id.clone(), tx_hash, payment.network.to_string())
        .await?;

    Ok(Json(response).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{counter::ClickCounter, db::MockLinksDB};
    use axum::http::HeaderValue;
    use ezlime_rs::CreatedLinkResponse;
    use x402_chain_eip155::v1_eip155_exact::ExactEvmPayloadAuthorization;
    use x402_types::timestamp::UnixTimestamp;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_handle_x402_create_sepolia_returns_demo() {
        // Create a mock app
        let mut db = MockLinksDB::new();
        db.expect_create_transaction().returning(|_| Ok(()));

        let app = App::new(
            "http://localhost:8080".to_string(),
            6,
            Arc::new(db),
            Arc::new(ClickCounter::new()),
            10,
        );

        let chain_id = ChainId::from_network_name("base-sepolia").unwrap();
        // Create a PaymentPayload for BaseSepolia
        let payment = PaymentPayload {
            x402_version: x402_types::proto::v1::X402Version1,
            scheme: "exact".to_string(),
            network: chain_id.to_string(),
            payload: ExactEvmPayload {
                signature: vec![0u8; 65].into(),
                authorization: ExactEvmPayloadAuthorization {
                    from: "0x0000000000000000000000000000000000000000".parse().unwrap(),
                    to: "0x0000000000000000000000000000000000000000".parse().unwrap(),
                    value: "1000000".parse().unwrap(),
                    valid_after: UnixTimestamp::from_secs(0),
                    valid_before: UnixTimestamp::from_secs(u64::MAX),
                    nonce: [0u8; 32].into(),
                },
            },
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
        let mock_settlement = SettleResponse::Success {
            payer: "0x0000000000000000000000000000000000000000".to_string(),
            transaction: "0xdeadbeefcafe".to_string(),
            network: chain_id.to_string(),
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
