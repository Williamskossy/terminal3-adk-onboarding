# Terminal 3 ADK — Onboarding Report

**Bounty:** [Create Agent ID, claim free tokens, & deploy first RUST contract on the network](https://superteam.fun/earn/listing/ai-id) · sponsor **LOL ventures**
**Submitted by:** RayVen
**Repo:** https://github.com/Williamskossy/terminal3-adk-onboarding
**Date:** 17 August 2026

| | |
|---|---|
| **Tenant DID** | `did:t3n:039b91465f60d1a39e2297dd04db188a2cef4736` |
| **Environment** | `testnet` — `https://cn-api.sg.testnet.t3n.terminal3.io` |
| **Trust manifest** | version `1786457685`, signed `2026-08-11T14:14:45Z` |
| **SDK** | `@terminal3/t3n-sdk` **4.39.1** (latest published) |
| **Host** | Windows 11 + WSL2 Ubuntu 24.04 · rustc 1.97.1 · Node 22.23.2 |

---

## 1. Summary

I completed the Quickstart and the first two walkthrough steps, then hit a **server-side
blocker that makes contract registration impossible from any client**. Rather than stop
there, I documented eight findings and built two additional TEE contracts with real use
cases, so this submission covers the bonus criterion even though step 3 is unreachable.

| Stage | Result |
|---|---|
| Claim API key + DID via SSO | ✅ |
| **Quickstart** — connect & authenticate | ✅ DID issued, trust anchor signature-verified |
| Set Up Dev Env — Rust + WASM toolchain | ✅ |
| Set Up Dev Env — `TenantClient` | ⚠️ constructed; its documented verification call is broken (findings 6, 7) |
| **Walkthrough 1** — write contract | ✅ |
| **Walkthrough 2** — build contract | ✅ 194 KB WASM component, header `0061736d0d000100` |
| **Walkthrough 3** — register contract | ❌ **blocked** — node rejects the SDK's own request |
| **Walkthrough 4** — invoke | ⬜ unreachable |
| **Walkthrough 5** — test | ⬜ unreachable |
| **Bonus** — beyond the first contract + use case | ✅ **two** further contracts built, 30 tests passing |

---

## 2. The blocker

Both control-plane calls fail server-side with the same complaint, at different payload
offsets:

```
RpcError: Invalid action request: missing field `script_name` at line 1 column 105
  rpcMethod: 'action.execute', httpStatus: -32602
  requestId: eee34a47-d631-47bf-9455-cc8f466dca30      <- tenant.tenant.me()

RpcError: Invalid action request: missing field `script_name` at line 1 column 189
  rpcMethod: 'action.execute', httpStatus: -32602
  requestId: c18753da-5d80-4ab1-9676-1258623e4811      <- tenant.contracts.register(...)
```

Reproduced **4 times across 2 runs**. All four request IDs are in §6 for log tracing.

**It is not a caller error:**

- The documented `register({ tail, version, wasm })` API exposes no parameter that could
  carry `script_name`.
- In the shipped `index.d.ts`, `script_name` occurs **only** inside `ChargeReason` — a
  `token.get-usage` **response** type. No request type models it.
- The strings `script_name` / `scriptName` do not appear in `dist/index.esm.js` at all.
- `script_name` **is** documented for step 4 (`invoke`), where the caller passes it
  explicitly. So the field exists in the wire protocol; the control path never populates it.
- 4.39.1 is the newest published version (4.32.0 → 4.39.1), so there is no SDK upgrade
  available that would send it.

**Conclusion:** the deployed testnet node requires a field the latest published SDK does not
send. No client-side workaround exists. **Any developer attempting this bounty right now
will be stopped at the same point.**

**To unblock:** a node build treating `script_name` as optional for control actions, a
published SDK that sends it, or documentation of the config field that supplies it
(`TenantClientConfig.tenantContractId` is undocumented and was the only candidate I found).

---

## 3. Findings

Full detail with versions, exact errors, and fixes is in **`FINDINGS.md`** in the repo.

| # | Severity | Finding |
|---|---|---|
| **5** | **BLOCKING** | **The documented Quickstart code cannot run.** `T3nClient` requires a `trustAnchor` argument that appears on **no** ADK doc page. Also **security-relevant** — see below. |
| **7** | **BLOCKING** | **Every control RPC rejected**: `missing field 'script_name'`. Blocks registration entirely (§2). |
| **8** | High | **The docs tell you not to hex-encode the tenant DID — but you must.** The documented form doesn't compile, and their own reference repo hex-encodes. The warning is inverted. |
| 6 | Medium | `tenant.me()` doesn't exist; it's `tenant.tenant.me()`. Also miscited in register-contract's troubleshooting table. |
| 1 | Medium | No Windows instructions anywhere in Get Started — every command is bash-only. Plus a WSL `PATH` trap. |
| 2 | Low | Documented repo structure omits `.cargo/config.toml` (which is what makes `cargo build` target WASM) and a third vendored WIT package. |
| 3 | Low | `cargo build` aborts on a slow connection with no guidance; presents as "the ADK build is broken". |
| 4 | Trivial | `wasm-tools` is "optional" on one page and load-bearing on the next. |

### The two worth acting on first

**Finding 5 is a security issue, not just a docs gap.** The quickstart's `new T3nClient({
wasmComponent, handlers })` throws `T3nConfigError: trustAnchor is required...`. The error
message helpfully offers two options, and the *easy* one is `{ unsafe_trust_server: true }` —
which **silently disables DKG attestation verification against a real node**. A developer
who is blocked and wants to move on will reach for it. The correct answer exists in the SDK's
own README but nothing in the docs points there:

```typescript
trustAnchor: await fetchTrustedManifest("testnet")   // "sandbox" | "testnet" | "production"
```

Verified working: anchor fetched from `<node>/api/trust-manifest`, signature checked against
the SDK-pinned operator key, 3 `expected_peer_ids`, 1 `rtmr3_allowlist` entry. **Putting this
line in the quickstart removes the incentive to disable attestation.**

**Finding 8 will cost every contract author time.** The docs say:

> `// tenant_did() already returns the tid as a string — do not hex::encode it again.`

But `host-tenant-1.0.0/package.wit` declares `tenant-did: func() -> list<u8>` (20 raw bytes),
so the documented `format!("z:{}:secrets", tid)` is a compile error — `Vec<u8>` has no
`Display`. And `z-tenant-flight/src/search.rs:182`, in the repo the docs tell you to clone,
does exactly what the comment forbids:

```rust
let map_name = alloc::format!("z:{}:secrets", hex::encode(&tid));
```

---

## 4. Beyond the first contract — two use cases, both built

Both compile to valid WASM components against the real host WIT and pass native unit tests.
Neither can be registered (§2), which is the only reason they are not deployed.

### A. `z-remit-guard` — confidential cross-border remittance

**168 KB component · 14 tests passing**

| Export | PII | Host interface |
|---|---|---|
| `quote-transfer` | no | `http` |
| `execute-payout` | yes | `http-with-placeholders` |
| `get-receipt` | no | `kv-store` |

**The problem.** A remittance operator must touch a recipient's legal name, date of birth,
and bank details to move money, and in a normal architecture its own servers see all of it.
That plaintext *is* the compliance surface: encryption at rest, access logs, retention
policy, breach exposure.

**What changes here.** The payout body is templated with `{{profile.*}}` markers the host
resolves *after* the contract has built the request. The operator runs the payout logic
without ever holding the PII, and a compromised build has nothing to exfiltrate — reading
its own request body back yields the unresolved template.

Also implemented: KV-backed idempotency (replays a stored receipt instead of paying twice),
and error mapping that deliberately never echoes an upstream body, since a
placeholder-resolved request's error may quote the substituted PII back.

**Why this is a market, not a demo.** Remittance into Africa is a high-volume, thin-margin,
heavily-regulated corridor where PII handling is the dominant compliance cost. "We never hold
your recipient's details, and here is the attestation" is a claim a competitor on ordinary
infrastructure cannot make.

### B. `z-credit-band` — confidential credit assessment

**172 KB component · 16 tests passing**

| Export | PII | Host interface |
|---|---|---|
| `assess` | yes | `http-with-placeholders`, `kv-store` |
| `get-band` | no | `kv-store` |

**The problem.** To borrow, you hand a lender months of raw bank statements. The lender keeps
them, as does every downstream system. You cannot un-share them — and the lender only ever
needed to know one thing.

**The inversion.** `assess` fetches the statement *inside the enclave*, scores it, and returns
**only** a band: `A`–`D`, a 0–100 score, machine-readable reasons for adverse-action notices,
and a deliberately coarse inflow bucket (`50k_200k`, never the exact figure). Transactions are
dropped before anything is written; only the band reaches KV.

| Party | Sees |
|---|---|
| Borrower | everything (it is their data) |
| The contract | transactions transiently, in enclave memory, never persisted |
| Tenant operator | the band only |
| Lender | the band only |

**Why the TEE is load-bearing.** Any ordinary server could return only a band — but you would
have to *trust* it to discard the statement. Here the code is attested and the operator cannot
read enclave memory, so "only the band leaves" is a property of the deployment rather than a
line in a privacy policy. That is the difference between a privacy feature and a privacy
guarantee, and it is what makes this worth building on T3N specifically.

The scoring logic is deliberately pure and separated from all host calls, so it unit-tests
natively — 16 tests including determinism, monotonicity (a strictly better borrower never
gets a worse band), and a test asserting the serialised band leaks no transaction detail.

### Tests that assert the security property

Most notable, in both contracts:

```rust
// z-remit-guard: every PII field must be a host-resolved marker, never a literal
for field in ["given_name","family_name","born_on","email","bank_account","bank_code"] {
    let v = recipient[field].as_str().unwrap();
    assert!(v.starts_with("{{profile.") && v.ends_with("}}"));
}

// z-credit-band: the band that leaves the enclave carries no statement data
for forbidden in ["amount_minor","balance_after","transactions","age_days"] {
    assert!(!json.contains(forbidden));
}
```

---

## 5. Evidence

Raw, unedited command output is in `evidence/logs/`; the PNGs in `evidence/screenshots/` are
those same logs rendered for legibility. **The logs are the authoritative artifact** — every
figure, DID, and request ID in the images comes from them verbatim, and nothing was
hand-edited. The capture script refuses to complete if the API key ever appears in a log.

| Screenshot | Shows |
|---|---|
| `01-toolchain.png` | rustc 1.97.1, cargo, Node 22.23.2, `wasm32-wasip2` installed |
| `02-build-contract.png` | build artifact, component header, sha256 |
| `03-quickstart-register.png` | trust anchor verified → DID issued → both `script_name` failures with request IDs |
| `04-node-sdk-versions.png` | node trust manifest, 4.39.1 is latest, `script_name` absent from the SDK bundle |

---

## 6. Request IDs for tracing

| Call | Request ID |
|---|---|
| `tenant.tenant.me()` — run 1 | `5fafa1df-f268-46a6-8407-308b0a1eaa5e` |
| `contracts.register` — run 1 | `eb753b64-c2e6-4bfb-8d90-debbfd442097` |
| `tenant.tenant.me()` — run 2 | `eee34a47-d631-47bf-9455-cc8f466dca30` |
| `contracts.register` — run 2 | `c18753da-5d80-4ab1-9676-1258623e4811` |

---

## 7. Reproducing this

```bash
cp .env.example .env          # paste your key from the claim page
bash install-toolchain.sh     # rustup + wasm32-wasip2 + node
bash build-contract.sh        # clone + build the reference contract
bash run-quickstart.sh        # connect, authenticate, attempt registration
bash build-extra-contracts.sh # build + test both additional contracts
bash capture-evidence.sh      # regenerate evidence/logs/
```

On a slow link use `build-contract-retry.sh` (finding 3).

---

## 8. What I would do next, given a working registration

1. Register `z-remit-guard`, create its `secrets` and `receipts` KV maps scoped to the
   returned `contract_id`, seed the provider key via the tenant SDK, and run the full
   quote → payout → receipt flow end to end.
2. Verify the placeholder guarantee empirically — confirm that a deliberately malformed
   `{{profile.*}}` marker returns `placeholder-denied`, and that a `{{secrets.*}}` marker is
   rejected as the WIT comments claim.
3. Register `z-credit-band` and confirm that only the band lands in KV after `assess`.
4. Test the version-shadowing warning in the register-contract docs: register `0.1.1` at the
   same tail and check whether calls pinning `0.1.0` still route to the pinned version. The
   docs flag this as unverified ("several teams have found…"), and it is worth confirming.
