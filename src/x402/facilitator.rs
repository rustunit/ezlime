use super::types::{PaymentPayload, SettleResponse, VerifyResponse};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Client for interacting with x402 facilitator service
#[derive(Clone, Debug)]
pub struct FacilitatorClient {
    base_url: String,
    client: Client,
}

impl FacilitatorClient {
    /// Create a new facilitator client
    pub fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { base_url, client })
    }

    /// Verify a payment without settling it
    /// Returns Ok(true) if payment is valid
    pub async fn verify(&self, payment: &PaymentPayload) -> Result<bool> {
        let url = format!("{}/verify", self.base_url);

        debug!(
            tx_hash = %payment.transaction_hash,
            network = %payment.network,
            "Verifying payment"
        );

        let response = self
            .client
            .post(&url)
            .json(&json!({
                "transactionHash": payment.transaction_hash,
                "network": payment.network,
            }))
            .send()
            .await
            .context("Failed to send verify request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                status = %status,
                body = %body,
                "Facilitator verify request failed"
            );
            return Ok(false);
        }

        let verify_response: VerifyResponse = response
            .json()
            .await
            .context("Failed to parse verify response")?;

        debug!(
            valid = verify_response.valid,
            message = ?verify_response.message,
            "Verify response received"
        );

        Ok(verify_response.valid)
    }

    /// Settle a payment on the blockchain
    /// This finalizes the payment and transfers funds
    pub async fn settle(&self, payment: &PaymentPayload) -> Result<SettleResponse> {
        let url = format!("{}/settle", self.base_url);

        info!(
            tx_hash = %payment.transaction_hash,
            network = %payment.network,
            "Settling payment"
        );

        let response = self
            .client
            .post(&url)
            .json(&json!({
                "transactionHash": payment.transaction_hash,
                "network": payment.network,
            }))
            .send()
            .await
            .context("Failed to send settle request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Facilitator settle request failed with status {}: {}",
                status,
                body
            );
        }

        let settle_response: SettleResponse = response
            .json()
            .await
            .context("Failed to parse settle response")?;

        info!(
            settled = settle_response.settled,
            tx_hash = %settle_response.transaction_hash,
            "Payment settled"
        );

        if !settle_response.settled {
            anyhow::bail!(
                "Payment settlement failed: {}",
                settle_response.message.unwrap_or_default()
            );
        }

        Ok(settle_response)
    }

    /// Verify and settle a payment in one call
    /// This is the recommended approach for maximum security
    pub async fn verify_and_settle(&self, payment: &PaymentPayload) -> Result<SettleResponse> {
        // First verify
        let valid = self.verify(payment).await?;
        if !valid {
            anyhow::bail!("Payment verification failed");
        }

        // Then settle
        self.settle(payment).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facilitator_client_creation() {
        let client = FacilitatorClient::new("https://example.com".to_string());
        assert!(client.is_ok());
    }
}
