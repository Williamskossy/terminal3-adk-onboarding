//! z-remit-guard v0.1.0 — confidential cross-border remittance.
//!
//! ## The problem this exists to solve
//!
//! A remittance operator has to touch a recipient's legal name, date of birth,
//! and bank details to move money — and in most architectures the operator's own
//! application servers see all of it. That plaintext is the compliance surface:
//! it has to be encrypted at rest, access-logged, retention-bounded, and it is
//! what leaks in a breach.
//!
//! On T3N the recipient's details never enter this contract's memory. The payout
//! request is templated with `{{profile.<field>}}` markers and the host resolves
//! them from the calling user's profile *after* this WASM has finished building
//! the request. So:
//!
//!   - the operator can run the payout logic without holding the PII,
//!   - a compromised build of this contract has nothing to exfiltrate — reading
//!     its own request body back yields only the unresolved template,
//!   - the delegation grant, not this code, decides which profile fields are
//!     reachable.
//!
//! ## Shape
//!
//! | Export            | PII | Host interface              |
//! |-------------------|-----|-----------------------------|
//! | `quote-transfer`  | no  | `http`                      |
//! | `execute-payout`  | yes | `http-with-placeholders`    |
//! | `get-receipt`     | no  | `kv-store`                  |
//!
//! The provider API key is read at runtime from the tenant's `secrets` KV map,
//! seeded by the tenant SDK before first use. It is never a contract argument.
//!
//! ## Host capabilities
//!
//! `tenant_context`, `logging`, `kv_store`, `http`, `http_with_placeholders` —
//! all derived from the imports in `wit/world.wit`, not from a separate manifest.

#![warn(clippy::style, missing_debug_implementations)]
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

extern crate alloc;

pub const CONTRACT_VERSION: &str = "0.1.0";

wit_bindgen::generate!({
    world: "remit-guard",
    path: "wit",
    additional_derives: [
        serde::Deserialize,
        serde::Serialize,
    ],
    generate_all,
});

mod payout;
mod quote;
mod receipt;
mod secrets;

struct Component;

#[cfg(target_arch = "wasm32")]
impl exports::z::remit_guard::contracts::Guest for Component {
    fn quote_transfer(
        req: exports::z::remit_guard::contracts::GenericInput,
    ) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
        let input = req.input.ok_or("quote-transfer: missing input")?;
        quote::quote_transfer(&input)
    }

    fn execute_payout(
        req: exports::z::remit_guard::contracts::GenericInput,
    ) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
        let input = req.input.ok_or("execute-payout: missing input")?;
        payout::execute_payout(&input)
    }

    fn get_receipt(
        req: exports::z::remit_guard::contracts::GenericInput,
    ) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
        let input = req.input.ok_or("get-receipt: missing input")?;
        receipt::get_receipt(&input)
    }
}

#[cfg(target_arch = "wasm32")]
export!(Component);

/// Build the fully-qualified tenant KV map name for `map`.
///
/// NOTE: `tenant_did()` returns the raw 20-byte CompactDid (`list<u8>` in the
/// WIT), so it MUST be hex-encoded to form the `z:<tid>:<map>` path. The ADK
/// docs currently say the opposite ("already returns the tid as a string — do
/// not hex::encode it again"); the documented form does not compile, and the
/// sponsor's own `z-tenant-flight` reference does hex-encode. See FINDINGS #8.
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
        assert_eq!(parts.len(), 3, "CONTRACT_VERSION must be MAJOR.MINOR.PATCH");
        for part in parts {
            assert!(part.parse::<u32>().is_ok(), "each part must be numeric");
        }
    }
}
