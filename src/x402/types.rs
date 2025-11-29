use serde::{Deserialize, Serialize};

/// Response returned when payment is required (HTTP 402)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequiredResponse {
    pub x402_version: u32,
    pub accepts: Vec<PaymentRequirement>,
}

/// Specification of how payment should be made (sent to client in 402 response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirement {
    /// Payment scheme (typically "exact")
    pub scheme: String,
    /// Blockchain network (e.g., "base-sepolia", "base")
    pub network: String,
    /// Maximum amount required in token base units
    pub max_amount_required: String,
    /// Token contract address (USDC on Base)
    pub asset: String,
    /// Merchant wallet address
    pub pay_to: String,
    /// Resource being purchased
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Description of the payment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Maximum timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_timeout_seconds: Option<u64>,
}

/// Payment proof provided by client in X-PAYMENT header (full x402 spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayload {
    pub x402_version: u32,
    pub scheme: String,
    pub network: String,
    pub payload: PaymentPayloadData,
}

/// ERC-3009 transfer authorization data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayloadData {
    /// Signature of the authorization
    pub signature: String,
    /// Transfer authorization details
    pub authorization: TransferAuthorization,
}

/// ERC-3009 transfer authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferAuthorization {
    /// Payer address
    pub from: String,
    /// Payee address (merchant)
    pub to: String,
    /// Amount in token base units
    pub value: String,
    /// Valid after timestamp
    pub valid_after: String,
    /// Valid before timestamp
    pub valid_before: String,
    /// Unique nonce
    pub nonce: String,
}

/// Request to facilitator for verify/settle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitatorRequest {
    pub payment_payload: PaymentPayload,
    pub payment_requirements: FacilitatorPaymentRequirement,
}

/// Payment requirements sent to facilitator (different format than client response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitatorPaymentRequirement {
    pub scheme: String,
    pub network: String,
    pub max_amount_required: String,
    pub pay_to: String,
    pub asset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_timeout_seconds: Option<u64>,
}

/// Response from facilitator /verify endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub is_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
}

/// Response from facilitator /settle endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleResponse {
    pub success: bool,
    pub payer: String,
    pub transaction: String,
    pub network: String,
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
                max_amount_required: "5000".to_string(),
                asset: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
                pay_to: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
                resource: Some("/x402/shorten".to_string()),
                description: Some("Link shortening service".to_string()),
                max_timeout_seconds: Some(60),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("x402Version"));
        assert!(json.contains("accepts"));
        assert!(json.contains("maxAmountRequired"));
    }

    #[test]
    fn test_payment_payload_deserialization() {
        let json = r#"{
            "x402Version": 1,
            "scheme": "exact",
            "network": "base-sepolia",
            "payload": {
                "signature": "0xabc123",
                "authorization": {
                    "from": "0x1234",
                    "to": "0x5678",
                    "value": "5000",
                    "validAfter": "1740672089",
                    "validBefore": "1740672154",
                    "nonce": "0xnonce123"
                }
            }
        }"#;

        let payload: PaymentPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.x402_version, 1);
        assert_eq!(payload.scheme, "exact");
        assert_eq!(payload.payload.authorization.value, "5000");
    }
}
