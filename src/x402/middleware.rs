use super::types::PaymentPayload;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::warn;

/// Helper to parse payment from X-PAYMENT header value
pub fn parse_payment_header(header_value: &str) -> Result<PaymentPayload, PaymentParseError> {
    // Decode base64
    let decoded = BASE64
        .decode(header_value)
        .map_err(|_| PaymentParseError::InvalidBase64)?;

    // Parse JSON
    let payment: PaymentPayload = serde_json::from_slice(&decoded).map_err(|e| {
        warn!(
            error = %e,
            raw_json = %String::from_utf8_lossy(&decoded),
            "Failed to parse payment payload"
        );
        PaymentParseError::InvalidJson
    })?;

    Ok(payment)
}

/// Error type for payment parsing
#[derive(Debug, thiserror::Error)]
pub enum PaymentParseError {
    #[error("Invalid base64 encoding in payment header")]
    InvalidBase64,
    #[error("Invalid JSON in payment payload")]
    InvalidJson,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payment() {
        use super::super::types::{PaymentPayloadData, TransferAuthorization};

        let payment = PaymentPayload {
            x402_version: 1,
            scheme: "exact".to_string(),
            network: "base-sepolia".to_string(),
            payload: PaymentPayloadData {
                signature: "0xsig123".to_string(),
                authorization: TransferAuthorization {
                    from: "0x1234".to_string(),
                    to: "0x5678".to_string(),
                    value: "5000".to_string(),
                    valid_after: "0".to_string(),
                    valid_before: "999999999999".to_string(),
                    nonce: "0xnonce".to_string(),
                },
            },
        };

        // Encode manually for testing
        let json = serde_json::to_string(&payment).unwrap();
        let encoded = BASE64.encode(json.as_bytes());

        // Parse it back
        let parsed = parse_payment_header(&encoded).unwrap();
        assert_eq!(parsed.x402_version, 1);
        assert_eq!(parsed.scheme, "exact");
        assert_eq!(parsed.network, "base-sepolia");
        assert_eq!(parsed.payload.authorization.value, "5000");
    }
}
