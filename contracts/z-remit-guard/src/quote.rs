//! `quote-transfer` — price a corridor. Touches no PII, so it uses the plain
//! synchronous `http` interface.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Deserialize;

/// Illustrative provider base. In a real deployment this is a payout provider
/// (Paystack, Flutterwave, Wise, …). The host resolves egress per-call from the
/// calling user's grant, so changing this constant alone does not widen access.
const REMIT_BASE: &str = "https://api.remit-provider.example/v1";

#[derive(Debug, Deserialize)]
pub struct QuoteReq {
    pub from: String,
    pub to: String,
    pub amount_minor: u64,
}

// ---------------------------------------------------------------------------
// Pure logic — no host calls, so this compiles and unit-tests natively.
// ---------------------------------------------------------------------------

pub fn parse_quote_req(input: &[u8]) -> Result<QuoteReq, String> {
    let req: QuoteReq =
        serde_json::from_slice(input).map_err(|e| format!("quote-transfer: bad JSON: {e}"))?;
    validate_quote_req(&req)?;
    Ok(req)
}

pub fn validate_quote_req(req: &QuoteReq) -> Result<(), String> {
    if req.amount_minor == 0 {
        return Err("quote-transfer: amount_minor must be > 0".to_string());
    }
    for (label, code) in [("from", &req.from), ("to", &req.to)] {
        if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(format!(
                "quote-transfer: {label} must be a 3-letter uppercase ISO-4217 code, got {code:?}"
            ));
        }
    }
    if req.from == req.to {
        return Err("quote-transfer: from and to currencies are identical".to_string());
    }
    Ok(())
}

/// Body sent to the provider's quote endpoint. Separate from the host call so
/// the wire shape is testable without a node.
pub fn quote_body(req: &QuoteReq) -> serde_json::Value {
    serde_json::json!({
        "source_currency": req.from,
        "target_currency": req.to,
        "source_amount_minor": req.amount_minor,
    })
}

// ---------------------------------------------------------------------------
// Host-facing entry point.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub fn quote_transfer(input: &[u8]) -> Result<Vec<u8>, String> {
    use crate::host::interfaces::{http, logging};

    let req = parse_quote_req(input)?;
    let api_key = crate::secrets::provider_api_key()?;

    let resp = http::call(&http::Request {
        method: http::Verb::Post,
        url: format!("{REMIT_BASE}/quotes"),
        headers: Some(crate::provider_headers(&api_key)),
        payload: Some(serde_json::to_vec(&quote_body(&req)).map_err(|e| e.to_string())?),
    })
    .map_err(|e| format!("provider quote request failed: {e}"))?;

    if resp.code != 200 && resp.code != 201 {
        let body = String::from_utf8_lossy(&resp.payload);
        return Err(format!(
            "provider quote failed: HTTP {} — {body}",
            resp.code
        ));
    }

    let _ = logging::info(&format!(
        "quote {}->{} for {} minor units",
        req.from, req.to, req.amount_minor
    ));

    // Pass the provider's quote straight back; it contains no PII.
    Ok(resp.payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_amount() {
        let err = parse_quote_req(br#"{"from":"GBP","to":"NGN","amount_minor":0}"#).unwrap_err();
        assert!(err.contains("must be > 0"), "{err}");
    }

    #[test]
    fn rejects_bad_currency_code() {
        let err = parse_quote_req(br#"{"from":"gbp","to":"NGN","amount_minor":100}"#).unwrap_err();
        assert!(err.contains("ISO-4217"), "{err}");
    }

    #[test]
    fn rejects_identical_currencies() {
        let err = parse_quote_req(br#"{"from":"NGN","to":"NGN","amount_minor":100}"#).unwrap_err();
        assert!(err.contains("identical"), "{err}");
    }

    #[test]
    fn accepts_valid_request_and_builds_body() {
        let req = parse_quote_req(br#"{"from":"GBP","to":"NGN","amount_minor":50000}"#).unwrap();
        assert_eq!(req.amount_minor, 50_000);
        let body = quote_body(&req);
        assert_eq!(body["source_currency"], "GBP");
        assert_eq!(body["target_currency"], "NGN");
        assert_eq!(body["source_amount_minor"], 50_000);
    }
}
