//! `assess` and `get-band` — the host-facing surface.
//!
//! `assess` is the only place raw statement data exists, and it exists only in
//! enclave memory for the duration of the call. Nothing but the band is written
//! to KV or returned.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Deserialize;

const STATEMENT_BASE: &str = "https://api.openbanking-provider.example/v1";

#[derive(Debug, Deserialize)]
pub struct AssessReq {
    /// Open-banking provider slug, e.g. "mono", "okra", "plaid".
    pub provider: String,
    /// How many months of history to request (3..=24).
    pub months: u32,
    /// Caller-chosen id this assessment is stored and later retrieved under.
    pub assessment_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GetBandReq {
    pub assessment_id: String,
}

// ---------------------------------------------------------------------------
// Pure validation — natively testable
// ---------------------------------------------------------------------------

pub fn parse_assess_req(input: &[u8]) -> Result<AssessReq, String> {
    let req: AssessReq =
        serde_json::from_slice(input).map_err(|e| format!("assess: bad JSON: {e}"))?;
    if !(3..=24).contains(&req.months) {
        return Err(format!("assess: months must be 3..=24, got {}", req.months));
    }
    validate_id("assess: provider", &req.provider, 32)?;
    validate_id("assess: assessment_id", &req.assessment_id, 64)?;
    Ok(req)
}

pub fn parse_get_band_req(input: &[u8]) -> Result<GetBandReq, String> {
    let req: GetBandReq =
        serde_json::from_slice(input).map_err(|e| format!("get-band: bad JSON: {e}"))?;
    validate_id("get-band: assessment_id", &req.assessment_id, 64)?;
    Ok(req)
}

/// KV keys and URL path segments come from caller input, so constrain them.
pub fn validate_id(label: &str, value: &str, max: usize) -> Result<(), String> {
    if value.is_empty() || value.len() > max {
        return Err(format!("{label} must be 1..={max} chars"));
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!("{label} may contain only [A-Za-z0-9_-]"));
    }
    Ok(())
}

/// The statement request. Borrower identifiers stay as host-resolved markers.
pub fn statement_body(req: &AssessReq) -> serde_json::Value {
    serde_json::json!({
        "months": req.months,
        "subject": {
            // Resolved host-side; plaintext never enters WASM memory.
            "bvn":            "{{profile.ng.bvn}}",
            "account_number": "{{profile.payout.account_number}}",
            "bank_code":      "{{profile.payout.bank_code}}",
        },
    })
}

// ---------------------------------------------------------------------------
// Host-facing
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub fn assess(input: &[u8]) -> Result<Vec<u8>, String> {
    use crate::host::interfaces::{http_with_placeholders as hwp, kv_store, logging};

    let req = parse_assess_req(input)?;
    let api_key = api_key()?;

    let mut headers = crate::provider_headers(&api_key);
    headers.push(("X-Provider".to_string(), req.provider.clone()));

    let resp = hwp::call(&hwp::Request {
        method: hwp::Verb::Post,
        url: format!("{STATEMENT_BASE}/statements"),
        headers: Some(headers),
        payload: Some(serde_json::to_vec(&statement_body(&req)).map_err(|e| e.to_string())?),
    })
    .map_err(|e| format!("statement fetch failed: {}", format_http_error(e)))?;

    if resp.code != 200 {
        // No body echo: the upstream error may quote resolved identifiers back.
        return Err(format!("statement provider returned HTTP {}", resp.code));
    }

    // ---- the sensitive window: raw statement exists only in these few lines ----
    let statement = crate::band::parse_statement(&resp.payload)?;
    let aggregates = crate::band::aggregate(&statement);
    let band = crate::band::score(&aggregates);
    drop(statement); // transactions dropped before anything is persisted
    // ---------------------------------------------------------------------------

    let out = serde_json::to_vec(&band).map_err(|e| e.to_string())?;

    // Only the band is persisted. Never the statement, never the aggregates.
    let map = crate::tenant_map_name("bands");
    kv_store::put(&map, req.assessment_id.as_bytes(), &out)
        .map_err(|e| format!("kv write band: {e}"))?;

    // Log the band, never the inputs.
    let _ = logging::info(&format!(
        "assessment {} -> band {} (score {}, {} months)",
        req.assessment_id, band.band, band.score, band.months_observed
    ));

    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub fn get_band(input: &[u8]) -> Result<Vec<u8>, String> {
    use crate::host::interfaces::kv_store;

    let req = parse_get_band_req(input)?;
    let map = crate::tenant_map_name("bands");
    kv_store::get(&map, req.assessment_id.as_bytes())
        .map_err(|e| format!("kv read {map}: {e}"))?
        .ok_or_else(|| format!("no band recorded for assessment {}", req.assessment_id))
}

#[cfg(target_arch = "wasm32")]
fn api_key() -> Result<String, String> {
    use crate::host::interfaces::kv_store;

    let map = crate::tenant_map_name("secrets");
    let bytes = kv_store::get(&map, b"statement_provider_api_key")
        .map_err(|e| format!("kv read {map}: {e}"))?
        .ok_or_else(|| {
            format!("statement_provider_api_key not found in {map} — seed it via the tenant SDK")
        })?;
    String::from_utf8(bytes).map_err(|e| format!("api key is not valid UTF-8: {e}"))
}

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
            "no user context bound — assess must be called through the Session API".to_string()
        }
        HttpError::UpstreamError(reason) => format!("upstream: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_window() {
        let err =
            parse_assess_req(br#"{"provider":"mono","months":1,"assessment_id":"a1"}"#).unwrap_err();
        assert!(err.contains("3..=24"), "{err}");
    }

    #[test]
    fn rejects_id_with_path_characters() {
        let err = parse_assess_req(
            br#"{"provider":"mono","months":6,"assessment_id":"../secrets"}"#,
        )
        .unwrap_err();
        assert!(err.contains("[A-Za-z0-9_-]"), "{err}");
    }

    #[test]
    fn accepts_valid_request() {
        let req =
            parse_assess_req(br#"{"provider":"mono","months":6,"assessment_id":"a-1"}"#).unwrap();
        assert_eq!(req.months, 6);
        assert_eq!(req.assessment_id, "a-1");
    }

    /// Borrower identifiers must be markers, never literals.
    #[test]
    fn statement_body_uses_only_placeholders_for_identity() {
        let req =
            parse_assess_req(br#"{"provider":"mono","months":6,"assessment_id":"a-1"}"#).unwrap();
        let body = statement_body(&req);
        for field in ["bvn", "account_number", "bank_code"] {
            let v = body["subject"][field].as_str().unwrap();
            assert!(
                v.starts_with("{{profile.") && v.ends_with("}}"),
                "{field} must be a profile placeholder, got {v:?}"
            );
        }
    }
}
