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

**The sponsor has since confirmed the blocker** — reported on Telegram 2026-08-17, answered
*"ah yes… they're fixing it atm"*, with registration waived for this submission (§3). I
re-tested after reporting it and it still reproduces, so the log trail now covers three
independent runs.

| Stage | Result |
|---|---|
| Claim API key + DID via SSO | ✅ |
| **Quickstart** — connect & authenticate | ✅ DID issued, trust anchor signature-verified |
| Set Up Dev Env — Rust + WASM toolchain | ✅ |
| Set Up Dev Env — `TenantClient` | ⚠️ constructed; its documented verification call is broken (findings 6, 7) |
| **Walkthrough 1** — write contract | ✅ |
| **Walkthrough 2** — build contract | ✅ 194 KB WASM component, header `0061736d0d000100` |
| **Walkthrough 3** — register contract | ❌ **blocked** — sponsor-confirmed server-side defect |
| **Walkthrough 4** — invoke | ⬜ unreachable |
| **Walkthrough 5** — test | ⬜ unreachable |
| **Bonus** — beyond the first contract + use case | ✅ **two** further contracts built, 30 tests passing |

---

## 2. The walkthrough, step by step

What I actually ran, in order, with the finding each step produced. Screenshot captions point
at the image that evidences that step; the raw log behind every image is in `evidence/logs/`.

### Step 0 — Claim an Agent ID and free tokens

Signed in at [`go.terminal3.io/adk-community`](https://go.terminal3.io/adk-community) via SSO,
which issued the API key and the tenant DID
`did:t3n:039b91465f60d1a39e2297dd04db188a2cef4736`.

Worth flagging for other developers: **the API key is displayed exactly once.** There is no
"reveal again" on the claim page, so save it before leaving the tab.

**Screenshot — the claim page with the Agent ID / DID issued.**

**Screenshot — the token balance after claiming.**

### Step 1 — Set up the dev environment

`rustup` + the `wasm32-wasip2` target + Node 22. Two things bit here: every command in Get
Started is bash-only with **no Windows path at all** (finding 1), so I moved to WSL2 Ubuntu
24.04; and `cargo build` kept aborting on my connection until I raised cargo's network timeouts
(finding 3) — which presents as "the ADK is broken" rather than "your link is slow".

**Screenshot — `01-toolchain.png`: rustc 1.97.1, cargo, Node 22.23.2, `wasm32-wasip2` installed.**

### Step 2 — Write the contract

Cloned the reference contract the docs point to
([`Terminal-3/z-tenant-flight`](https://github.com/Terminal-3/z-tenant-flight), v0.4.1) and read
it against the walkthrough. The documented repo structure omits `.cargo/config.toml` — the file
that actually makes `cargo build` target WASM — and one of the three vendored WIT packages
(finding 2). The docs also tell you *not* to hex-encode the tenant DID when building a KV map
name; you must, and their own reference repo does (finding 8, the one most likely to cost every
contract author time).

### Step 3 — Build the contract

```bash
cargo build --target wasm32-wasip2 --release
xxd -l 8 -p target/wasm32-wasip2/release/z_tenant_flight.wasm   # 0061736d0d000100
```

That header is the check that matters: `0d000100` in bytes 5–8 means a **component** (layer=1),
which is what registration requires, versus `01000000` for a bare core module. 197,904 bytes.

**Screenshot — `02-build-contract.png`: the artifact, its component header, and its sha256.**

### Step 4 — Connect and authenticate

This is where the documented quickstart stops working. `new T3nClient({ wasmComponent, handlers })`
throws `T3nConfigError: trustAnchor is required` — and `trustAnchor` appears on no ADK
documentation page (finding 5). The correct value is in the SDK's own README:

```typescript
trustAnchor: await fetchTrustedManifest("testnet")
```

With that, the anchor fetched and signature-verified against the SDK-pinned operator key, and
the node issued the DID. The docs' next call, `tenant.me()`, doesn't exist — it is
`tenant.tenant.me()` (finding 6), and that call then failed server-side, which was the first
sighting of the blocker.

**Screenshot — `03-quickstart-register.png`: trust anchor verified → DID issued → the first
`script_name` failures, with server-side request IDs.**

### Step 5 — Register the contract ❌

```
RpcError: Invalid action request: missing field `script_name` at line 1 column 189
```

Blocked here, and not by anything fixable from the client (finding 7, §3). Re-tested after
reporting it to the sponsor — still failing.

**Screenshot — `06-registration-retry.png`: the re-test after the sponsor report, still blocked,
two fresh request IDs.**

### Steps 6–7 — Invoke and test ⬜

Unreachable: both need a registered `contract_id`.

### Bonus — two further contracts

Rather than stop at a blocked step, I wrote two more TEE contracts with real use cases (§5),
built both to valid components and unit-tested them natively.

**Screenshot — `05-bonus-contracts.png`: all 30 tests passing by name, byte counts, component
headers, sha256s and exported interfaces for both.**

---

## 3. The blocker

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

Reproduced **6 times across 3 runs**, spread over ~17 hours, with the node on an unchanged
trust manifest (`1786457685`) throughout. All six request IDs are in §7 for log tracing.

### Confirmed by the sponsor

I reported this on Telegram on 2026-08-17. The sponsor's reply:

> "ah yes. — they're fixing it atm so I'll keep you posted once the error is fixed!
> just submit your did — it's ok, we understand the constraints"

So it is an acknowledged server-side defect under active repair, and registration was waived
for this submission. I re-ran the attempt **after** reporting it
(`evidence/logs/06-registration-retry.log`, 22:55:25 UTC) and it still failed identically —
the fix had not reached the `sg.testnet` cluster at the time of writing.

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

## 4. Findings

Full detail with versions, exact errors, and fixes is in **`FINDINGS.md`** in the repo.

| # | Severity | Finding |
|---|---|---|
| **5** | **BLOCKING** | **The documented Quickstart code cannot run.** `T3nClient` requires a `trustAnchor` argument that appears on **no** ADK doc page. Also **security-relevant** — see below. |
| **7** | **BLOCKING** | **Every control RPC rejected**: `missing field 'script_name'`. Blocks registration entirely. **Sponsor-confirmed and being fixed** (§3). |
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

## 5. Beyond the first contract — two use cases, both built

Both compile to valid WASM components against the real host WIT and pass native unit tests.
Neither can be registered (§3), which is the only reason they are not deployed.

### A. `z-remit-guard` — confidential cross-border remittance

**168 KB component (169,073 bytes) · 14 tests passing · exports `z:remit-guard/contracts@0.1.0`**

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

**172 KB component (175,219 bytes) · 16 tests passing · exports `z:credit-band/contracts@0.1.0`**

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

## 6. Evidence

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
| `05-bonus-contracts.png` | **all 30 tests passing by name**, exact byte counts, component headers, sha256s, exported interfaces for both bonus contracts |
| `06-registration-retry.png` | the re-test after the sponsor report — still blocked, two fresh request IDs |

Verifiable hashes of the three components:

| Artifact | Bytes | sha256 |
|---|---|---|
| `z_tenant_flight.wasm` | 197,904 | `edf634e46265bdfcb1c236190b7a7d4110ff6952e04fcec32054ccc4cd69955d` |
| `z_remit_guard.wasm` | 169,073 | `2d99683b9a258d35be30211f684c17617bf3fe54af898ffe3cf468b858860c2d` |
| `z_credit_band.wasm` | 175,219 | `77ad6772cfc6b08035ca80452419bc5a3f2219203a0c8be2a7daf07f535764bb` |

A note on the timestamps: this host's system clocks drifted after a crash mid-project, so
every evidence stamp is anchored to an authoritative HTTP `Date` header rather than the local
clock, and the capture scripts say so in the log header.

---

## 7. Request IDs for tracing

| Call | Request ID |
|---|---|
| `tenant.tenant.me()` — run 1 | `5fafa1df-f268-46a6-8407-308b0a1eaa5e` |
| `contracts.register` — run 1 | `eb753b64-c2e6-4bfb-8d90-debbfd442097` |
| `tenant.tenant.me()` — run 2 | `eee34a47-d631-47bf-9455-cc8f466dca30` |
| `contracts.register` — run 2 | `c18753da-5d80-4ab1-9676-1258623e4811` |
| `tenant.tenant.me()` — run 3, after the sponsor report | `0b7d978c-4681-49cb-be4e-75e32f55895b` |
| `contracts.register` — run 3, after the sponsor report | `7df094fe-6697-4abd-9286-352a565f907d` |

Runs 1–2 are in `03-quickstart-and-register.log`; run 3 is in `06-registration-retry.log`.

---

## 8. Reproducing this

```bash
cp .env.example .env             # paste your key from the claim page
bash install-toolchain.sh        # rustup + wasm32-wasip2 + node
bash build-contract.sh           # clone + build the reference contract
bash run-quickstart.sh           # connect, authenticate, attempt registration
bash build-extra-contracts.sh    # build + test both additional contracts
bash capture-evidence.sh         # regenerate evidence/logs/01..04
bash capture-bonus-evidence.sh   # regenerate evidence/logs/05..06
bash render-evidence.sh          # logs -> evidence/html/ (then screenshot at the printed height)
```

Both capture scripts refuse to finish if the API key ever appears in a captured log.

On a slow link use `build-contract-retry.sh` (finding 3).

---

## 9. What I would do next, given a working registration

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

The sponsor has offered to tell me when the `script_name` fix ships. Every step above is
already scripted (`run-quickstart.sh`, `capture-bonus-evidence.sh`), so re-testing on the
fixed node is one command, and I am happy to do it and report back.
