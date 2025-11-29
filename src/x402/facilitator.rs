use super::types::{
    FacilitatorPaymentRequirement, FacilitatorRequest, PaymentPayload, SettleResponse,
    VerifyResponse,
};
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::{error, info, warn};

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
    pub async fn verify(
        &self,
        payment: &PaymentPayload,
        requirements: &FacilitatorPaymentRequirement,
    ) -> Result<VerifyResponse> {
        let url = format!("{}/verify", self.base_url);

        let request_body = FacilitatorRequest {
            payment_payload: payment.clone(),
            payment_requirements: requirements.clone(),
        };

        info!(
            from = %payment.payload.authorization.from,
            to = %payment.payload.authorization.to,
            value = %payment.payload.authorization.value,
            "Verifying payment with facilitator"
        );

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    url = %url,
                    "HTTP request to facilitator failed"
                );
                anyhow::anyhow!("Failed to send verify request: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                status = %status,
                body = %body,
                "Facilitator verify request failed"
            );
            anyhow::bail!("Facilitator returned {}: {}", status, body);
        }

        let verify_response: VerifyResponse = response
            .json()
            .await
            .context("Failed to parse verify response")?;

        if !verify_response.is_valid {
            warn!(
                payer = ?verify_response.payer,
                reason = ?verify_response.invalid_reason,
                "Payment verification failed"
            );
            anyhow::bail!(
                "Payment verification failed: {}",
                verify_response
                    .invalid_reason
                    .unwrap_or_else(|| "Unknown reason".to_string())
            );
        }

        info!(
            payer = ?verify_response.payer,
            "Payment verified successfully"
        );

        Ok(verify_response)
    }

    /// Settle a payment on the blockchain
    /// This finalizes the payment and transfers funds
    pub async fn settle(
        &self,
        payment: &PaymentPayload,
        requirements: &FacilitatorPaymentRequirement,
    ) -> Result<SettleResponse> {
        let url = format!("{}/settle", self.base_url);

        let request_body = FacilitatorRequest {
            payment_payload: payment.clone(),
            payment_requirements: requirements.clone(),
        };

        info!(
            from = %payment.payload.authorization.from,
            to = %payment.payload.authorization.to,
            value = %payment.payload.authorization.value,
            "Settling payment with facilitator"
        );

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    url = %url,
                    "HTTP request to facilitator failed"
                );
                anyhow::anyhow!("Failed to send settle request: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(
                status = %status,
                body = %body,
                "Facilitator settle request failed"
            );
            anyhow::bail!("Facilitator returned {}: {}", status, body);
        }

        let settle_response: SettleResponse = response
            .json()
            .await
            .context("Failed to parse settle response")?;

        info!(
            tx_hash = %settle_response.transaction,
            payer = %settle_response.payer,
            "Payment settled successfully"
        );

        if !settle_response.success {
            anyhow::bail!("Payment settlement failed");
        }

        Ok(settle_response)
    }

    /// Verify and settle a payment in one call
    /// This is the recommended approach for maximum security
    pub async fn verify_and_settle(
        &self,
        payment: &PaymentPayload,
        requirements: &FacilitatorPaymentRequirement,
    ) -> Result<SettleResponse> {
        // First verify
        self.verify(payment, requirements).await?;

        // Then settle
        self.settle(payment, requirements).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facilitator_client_creation() {
        let client = FacilitatorClient::new("https://x402.org/facilitator".to_string());
        assert!(client.is_ok());
    }
}
