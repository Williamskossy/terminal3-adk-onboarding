//! `execute-payout` — move the money. Carries recipient PII, so every
//! identity field is a `{{profile.<field>}}` marker resolved host-side.
//!
//! The contract builds a request that *describes* the recipient without ever
//! holding their details. Reading these bytes back inside WASM yields the
//! unresolved template, which is the entire point.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Deserialize;

const REMIT_BASE: &str = "https://api.remit-provider.example/v1";

#[derive(Debug, Deserialize)]
pub struct PayoutReq {
    pub quote_id: String,
    pub amount_minor: u64,
    pub currency: String,
    /// Caller-supplied idempotency key. Also the KV receipt key.
    pub reference: String,
}

// ---------------------------------------------------------------------------
// Pure logic
// ---------------------------------------------------------------------------

pub fn parse_payout_req(input: &[u8]) -> Result<PayoutReq, String> {
    let req: PayoutReq =
        serde_json::from_slice(input).map_err(|e| format!("execute-payout: bad JSON: {e}"))?;
    validate_payout_req(&req)?;
    Ok(req)
}

pub fn validate_payout_req(req: &PayoutReq) -> Result<(), String> {
    if req.quote_id.is_empty() {
        return Err("execute-payout: quote_id is required".to_string());
    }
    if req.amount_minor == 0 {
        return Err("execute-payout: amount_minor must be > 0".to_string());
    }
    if req.currency.len() != 3 || !req.currency.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(format!(
            "execute-payout: currency must be a 3-letter uppercase ISO-4217 code, got {:?}",
            req.currency
        ));
    }
    // The reference becomes a KV key and an idempotency header — keep it tame.
    if req.reference.is_empty() || req.reference.len() > 64 {
        return Err("execute-payout: reference must be 1..=64 chars".to_string());
    }
    if !req
        .reference
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(
            "execute-payout: reference may contain only [A-Za-z0-9_-]".to_string(),
        );
    }
    Ok(())
}

/// The payout body, with recipient identity left as host-resolved placeholders.
///
/// Every `{{profile.*}}` marker here is substituted on the host stack between
/// manifest validation and the outbound call. Only the `profile` namespace is
/// permitted — the host rejects anything else with `placeholder-denied`.
pub fn payout_body(req: &PayoutReq) -> serde_json::Value {
    serde_json::json!({
        "quote_id": req.quote_id,
        "reference": req.reference,
        "amount": { "minor": req.amount_minor, "currency": req.currency },
        "recipient": {
            // Resolved host-side. Plaintext PII never enters WASM memory.
            "given_name":     "{{profile.first_name}}",
            "family_name":    "{{profile.last_name}}",
            "born_on":        "{{profile.date_of_birth}}",
            "email":          "{{profile.verified_contacts.email.value}}",
            "bank_account":   "{{profile.payout.account_number}}",
            "bank_code":      "{{profile.payout.bank_code}}",
        },
    })
}

// ---------------------------------------------------------------------------
// Host-facing entry point
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub fn execute_payout(input: &[u8]) -> Result<Vec<u8>, String> {
    use crate::host::interfaces::{http_with_placeholders as hwp, kv_store, logging};

    let req = parse_payout_req(input)?;
    let api_key = crate::secrets::provider_api_key()?;

    // Idempotency: if we already have a receipt under this reference, return it
    // rather than paying twice.
    let map = crate::tenant_map_name("receipts");
    if let Ok(Some(existing)) = kv_store::get(&map, req.reference.as_bytes()) {
        let _ = logging::info(&format!("payout {} already settled; replaying receipt", req.reference));
        return Ok(existing);
    }

    let mut headers = crate::provider_headers(&api_key);
    headers.push(("Idempotency-Key".to_string(), req.reference.clone()));

    let resp = hwp::call(&hwp::Request {
        method: hwp::Verb::Post,
        url: format!("{REMIT_BASE}/transfers"),
        headers: Some(headers),
        payload: Some(serde_json::to_vec(&payout_body(&req)).map_err(|e| e.to_string())?),
    })
    .map_err(|e| format!("payout failed: {}", format_http_error(e)))?;

    if resp.code != 200 && resp.code != 201 {
        // Deliberately does not echo the response body: on a placeholder-resolved
        // request the upstream's error may quote the substituted PII back at us.
        return Err(format!("payout rejected by provider: HTTP {}", resp.code));
    }

    // Record the receipt so the call is idempotent and auditable.
    kv_store::put(&map, req.reference.as_bytes(), &resp.payload)
        .map_err(|e| format!("kv write receipt: {e}"))?;

    let _ = logging::info(&format!("payout {} settled", req.reference));
    Ok(resp.payload)
}

/// Map the typed placeholder errors to messages that never leak resolved PII.
#[cfg(target_arch = "wasm32")]
fn format_http_error(e: crate::host::interfaces::http_with_placeholders::HttpError) -> String {
    use crate::host::interfaces::http_with_placeholders::HttpError;
    match e {
        HttpError::EgressDenied(host) => format!("egress denied for host {host}"),
        HttpError::PlaceholderDenied(marker) => format!("placeholder not permitted: {marker}"),
        HttpError::PlaceholderUnknown(field) => {
            format!("your profile is missing a required field: {field}")
        }
        HttpError::PlaceholderNoUserContext => {
            "no user context bound — execute-payout must be called through the Session API"
                .to_string()
        }
        HttpError::UpstreamError(reason) => format!("upstream: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_req() -> &'static [u8] {
        br#"{"quote_id":"q_123","amount_minor":50000,"currency":"NGN","reference":"ref-001"}"#
    }

    #[test]
    fn accepts_valid_request() {
        let req = parse_payout_req(ok_req()).unwrap();
        assert_eq!(req.reference, "ref-001");
    }

    #[test]
    fn rejects_reference_with_path_characters() {
        let err = parse_payout_req(
            br#"{"quote_id":"q","amount_minor":1,"currency":"NGN","reference":"a/b"}"#,
        )
        .unwrap_err();
        assert!(err.contains("[A-Za-z0-9_-]"), "{err}");
    }

    #[test]
    fn rejects_missing_quote_id() {
        let err = parse_payout_req(
            br#"{"quote_id":"","amount_minor":1,"currency":"NGN","reference":"r"}"#,
        )
        .unwrap_err();
        assert!(err.contains("quote_id"), "{err}");
    }

    /// The security property this contract exists for: no recipient identity
    /// appears in the body the contract builds — only host-resolved markers.
    #[test]
    fn body_contains_only_placeholders_for_pii() {
        let req = parse_payout_req(ok_req()).unwrap();
        let body = payout_body(&req);
        let recipient = &body["recipient"];
        for field in [
            "given_name",
            "family_name",
            "born_on",
            "email",
            "bank_account",
            "bank_code",
        ] {
            let v = recipient[field].as_str().unwrap();
            assert!(
                v.starts_with("{{profile.") && v.ends_with("}}"),
                "{field} must be a profile placeholder, got {v:?}"
            );
        }
    }

    #[test]
    fn body_carries_no_secret_namespace_markers() {
        let req = parse_payout_req(ok_req()).unwrap();
        let s = serde_json::to_string(&payout_body(&req)).unwrap();
        assert!(
            !s.contains("{{secrets."),
            "the host rejects non-profile namespaces; none should be emitted"
        );
    }
}
