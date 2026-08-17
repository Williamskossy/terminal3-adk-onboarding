//! z-credit-band v0.1.0 — confidential credit assessment.
//!
//! ## The problem
//!
//! To borrow, you hand a lender months of raw bank statements. The lender keeps
//! them; so does every downstream system. You cannot un-share them, and the
//! lender only ever needed one thing: are you creditworthy?
//!
//! ## The inversion
//!
//! This contract fetches the statement *inside the enclave* and returns only a
//! band (`A`–`D`), a score, a coarse inflow bucket, and machine-readable reasons.
//! The transactions are never returned and never persisted — only the band is
//! written to `z:<tid>:bands`.
//!
//! The borrower's identifiers are never contract arguments either: the statement
//! request is templated with `{{profile.<field>}}` markers that the host resolves
//! after this WASM has built the request.
//!
//! What each party ends up holding:
//!
//! | Party | Sees |
//! |---|---|
//! | Borrower | everything (it is their data) |
//! | This contract | transactions transiently, in enclave memory, never persisted |
//! | Tenant operator | the band only |
//! | Lender | the band only |
//!
//! ## Why a TEE is load-bearing here
//!
//! Any ordinary server could compute a band and return only that — but you would
//! have to *trust* it to discard the statement. Here the code is attested and the
//! operator cannot read enclave memory, so "only the band leaves" is a property
//! of the deployment rather than a promise in a privacy policy.
//!
//! ## Host capabilities
//!
//! `tenant_context`, `logging`, `kv_store`, `http_with_placeholders` — derived
//! from the imports in `wit/world.wit`.

#![warn(clippy::style, missing_debug_implementations)]
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

extern crate alloc;

pub const CONTRACT_VERSION: &str = "0.1.0";

wit_bindgen::generate!({
    world: "credit-band",
    path: "wit",
    additional_derives: [
        serde::Deserialize,
        serde::Serialize,
    ],
    generate_all,
});

pub mod assess;
pub mod band;

struct Component;

#[cfg(target_arch = "wasm32")]
impl exports::z::credit_band::contracts::Guest for Component {
    fn assess(
        req: exports::z::credit_band::contracts::GenericInput,
    ) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
        let input = req.input.ok_or("assess: missing input")?;
        assess::assess(&input)
    }

    fn get_band(
        req: exports::z::credit_band::contracts::GenericInput,
    ) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
        let input = req.input.ok_or("get-band: missing input")?;
        assess::get_band(&input)
    }
}

#[cfg(target_arch = "wasm32")]
export!(Component);

/// Build the fully-qualified tenant KV map name for `map`.
///
/// `tenant_did()` returns the raw 20-byte CompactDid (`list<u8>`), so it MUST be
/// hex-encoded to form `z:<tid>:<map>`. The ADK docs say the opposite; the
/// documented form does not compile. See FINDINGS #8.
#[cfg(target_arch = "wasm32")]
fn tenant_map_name(map: &str) -> alloc::string::String {
    let tid = host::tenant::tenant_context::tenant_did();
    alloc::format!("z:{}:{}", hex::encode(&tid), map)
}

/// Standard provider headers. `Vec<(String, String)>` matches the WIT
/// `option<list<tuple<string, string>>>` header shape.
#[cfg(target_arch = "wasm32")]
fn provider_headers(
    api_key: &str,
) -> alloc::vec::Vec<(alloc::string::String, alloc::string::String)> {
    use alloc::string::ToString;
    alloc::vec![
        ("Authorization".to_string(), alloc::format!("Bearer {api_key}")),
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Accept".to_string(), "application/json".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::CONTRACT_VERSION;

    #[test]
    fn contract_version_is_semver() {
        let parts: alloc::vec::Vec<&str> = CONTRACT_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
        for p in parts {
            assert!(p.parse::<u32>().is_ok());
        }
    }
}
