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
        warn!(error = %e, "Failed to parse payment payload");
        PaymentParseError::InvalidJson
    })?;

    Ok(payment)
}

/// Error type for payment parsing
#[derive(Debug)]
pub enum PaymentParseError {
    InvalidBase64,
    InvalidJson,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payment() {
        let payment = PaymentPayload {
            transaction_hash: "0xabc123".to_string(),
            network: "base-sepolia".to_string(),
            payment_requirement: None,
        };

        // Encode manually for testing
        let json = serde_json::to_string(&payment).unwrap();
        let encoded = BASE64.encode(json.as_bytes());

        // Parse it back
        let parsed = parse_payment_header(&encoded).unwrap();
        assert_eq!(parsed.transaction_hash, "0xabc123");
        assert_eq!(parsed.network, "base-sepolia");
    }
}
