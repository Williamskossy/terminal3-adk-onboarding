//! `get-receipt` — read back a recorded payout receipt from tenant KV.
//! No PII, no egress; just a keyed read out of `z:<tid>:receipts`.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReceiptReq {
    pub reference: String,
}

pub fn parse_receipt_req(input: &[u8]) -> Result<ReceiptReq, String> {
    let req: ReceiptReq =
        serde_json::from_slice(input).map_err(|e| format!("get-receipt: bad JSON: {e}"))?;
    if req.reference.is_empty() || req.reference.len() > 64 {
        return Err("get-receipt: reference must be 1..=64 chars".to_string());
    }
    if !req
        .reference
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("get-receipt: reference may contain only [A-Za-z0-9_-]".to_string());
    }
    Ok(req)
}

#[cfg(target_arch = "wasm32")]
pub fn get_receipt(input: &[u8]) -> Result<Vec<u8>, String> {
    use crate::host::interfaces::kv_store;

    let req = parse_receipt_req(input)?;
    let map = crate::tenant_map_name("receipts");

    kv_store::get(&map, req.reference.as_bytes())
        .map_err(|e| format!("kv read {map}: {e}"))?
        .ok_or_else(|| format!("no receipt recorded for reference {}", req.reference))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_reference() {
        assert!(parse_receipt_req(br#"{"reference":""}"#).is_err());
    }

    #[test]
    fn rejects_traversal_attempt() {
        let err = parse_receipt_req(br#"{"reference":"../secrets"}"#).unwrap_err();
        assert!(err.contains("[A-Za-z0-9_-]"), "{err}");
    }

    #[test]
    fn accepts_plain_reference() {
        assert_eq!(
            parse_receipt_req(br#"{"reference":"ref-001"}"#).unwrap().reference,
            "ref-001"
        );
    }
}
