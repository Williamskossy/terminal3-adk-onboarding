//! Reading the provider API key from the tenant's `secrets` KV map.
//!
//! The key is seeded by the tenant SDK before the contract runs — there is no
//! `set-credentials` host function, and the key is never a contract argument.

#[cfg(target_arch = "wasm32")]
use alloc::format;
#[cfg(target_arch = "wasm32")]
use alloc::string::String;

/// Fetch the provider API key from `z:<tid>:secrets`.
///
/// The map name must be the FULL `z:<tid>:<map>` path; the host enforces the
/// prefix. See `crate::tenant_map_name` for the hex-encoding caveat.
#[cfg(target_arch = "wasm32")]
pub fn provider_api_key() -> Result<String, String> {
    use crate::host::interfaces::kv_store;

    let map = crate::tenant_map_name("secrets");
    let bytes = kv_store::get(&map, b"remit_provider_api_key")
        .map_err(|e| format!("kv read {map}: {e}"))?
        .ok_or_else(|| {
            format!("remit_provider_api_key not found in {map} — seed it via the tenant SDK first")
        })?;
    String::from_utf8(bytes).map_err(|e| format!("api key is not valid UTF-8: {e}"))
}
