use serde::{Deserialize, Serialize};

/// Response returned when payment is required (HTTP 402)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequiredResponse {
    pub x402_version: u32,
    pub accepts: Vec<PaymentRequirement>,
}

/// Specification of how payment should be made
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirement {
    /// Payment scheme (typically "exact")
    pub scheme: String,
    /// Blockchain network (e.g., "base-sepolia", "base")
    pub network: String,
    /// Amount in token units (e.g., "0.005")
    pub amount: String,
    /// Token contract address (USDC on Base)
    pub asset: String,
    /// Merchant wallet address
    pub destination: String,
}

/// Payment proof provided by client in X-PAYMENT header
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayload {
    /// Transaction hash on blockchain
    pub transaction_hash: String,
    /// Network where transaction was submitted
    pub network: String,
    /// Optional: payment requirement that was fulfilled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_requirement: Option<PaymentRequirement>,
}

/// Response from facilitator /verify endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response from facilitator /settle endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleResponse {
    pub settled: bool,
    pub transaction_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_required_serialization() {
        let response = PaymentRequiredResponse {
            x402_version: 1,
            accepts: vec![PaymentRequirement {
                scheme: "exact".to_string(),
                network: "base-sepolia".to_string(),
                amount: "0.005".to_string(),
                asset: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
                destination: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("x402Version"));
        assert!(json.contains("accepts"));
    }

    #[test]
    fn test_payment_payload_deserialization() {
        let json = r#"{
            "transactionHash": "0xabc123",
            "network": "base-sepolia"
        }"#;

        let payload: PaymentPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.transaction_hash, "0xabc123");
        assert_eq!(payload.network, "base-sepolia");
    }
}
