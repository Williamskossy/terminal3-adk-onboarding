# Terminal 3 ADK — onboarding walkthrough, findings, and evidence

Submission for the Superteam Earn bounty
[**Create Agent ID, claim free tokens, & deploy first RUST contract on the network**](https://superteam.fun/earn/listing/ai-id)
(sponsor: LOL ventures).

**Tenant DID:** `did:t3n:039b91465f60d1a39e2297dd04db188a2cef4736`
**Environment:** `testnet` — `https://cn-api.sg.testnet.t3n.terminal3.io`
**SDK:** `@terminal3/t3n-sdk` 4.39.1 (latest published)
**Host:** Windows 11 + WSL2 Ubuntu 24.04, rustc 1.97.1, Node 22.23.2

---

## Outcome in one line

Quickstart completes and the contract compiles to a valid WASM component — but **contract
registration is blocked by a server-side wire-format mismatch** (`missing field 'script_name'`),
so walkthrough steps 3–5 cannot be completed by any client-side means. **The sponsor has
confirmed the defect and is fixing it.** Full evidence below.

| Stage | Result |
|---|---|
| Claim API key + DID (SSO) | ✅ |
| Quickstart — connect & authenticate | ✅ `did:t3n:039b9146…` issued |
| Set Up Dev Env — Rust + WASM toolchain | ✅ |
| Set Up Dev Env — `TenantClient` | ⚠️ built, but its documented verification call is broken (findings #6, #7) |
| Walkthrough 1 — write contract | ✅ |
| Walkthrough 2 — build contract | ✅ 194 KB component, header `0061736d0d000100` |
| Walkthrough 3 — register contract | ❌ **blocked** — sponsor-confirmed server-side defect |
| Walkthrough 4 — invoke contract | ⬜ unreachable |
| Walkthrough 5 — test | ⬜ unreachable |
| **Bonus — beyond the first contract, with use cases** | ✅ **two more TEE contracts, 30 tests passing** |

**8 findings** are documented in [`FINDINGS.md`](./FINDINGS.md) — 2 blocking, 1 security-relevant.

Headline items:

1. **The documented Quickstart code cannot run.** `T3nClient` requires a `trustAnchor`
   argument that appears on no ADK documentation page. Fix: `fetchTrustedManifest("testnet")`.
   Security-relevant, because the error message's suggested alternative
   (`{ unsafe_trust_server: true }`) silently disables DKG attestation verification.
2. **Every control RPC is rejected by the node** with `Invalid action request: missing field
   'script_name'`. Reproduced 6 times across 3 runs, each with a server-side request ID.
   Reported to the sponsor, who confirmed it is being fixed.
3. **The docs tell you not to hex-encode the tenant DID — but you must**, and the reference
   repo the docs tell you to clone does exactly that.
4. `tenant.me()` from the docs doesn't exist — it's `tenant.tenant.me()`.

---

## Beyond the first contract

Two additional TEE contracts, both compiled against the real host WIT and unit-tested. Neither
can be registered while finding #7 stands, which is the only reason they are not deployed.

| Contract | What it does | Component | Tests |
|---|---|---|---|
| [`contracts/z-remit-guard`](./contracts/z-remit-guard) | Confidential cross-border remittance — recipient PII is templated as `{{profile.*}}` markers the host resolves *after* the contract builds the request, so the operator never holds it | 169,073 B | 14 ✅ |
| [`contracts/z-credit-band`](./contracts/z-credit-band) | Confidential credit assessment — scores a bank statement inside the enclave and returns **only** a band (`A`–`D`), never the transactions | 175,219 B | 16 ✅ |

Both include tests that assert the security property itself, not just the happy path: that
every PII field in an outbound body is an unresolved placeholder, and that a serialised band
contains no transaction data. See [`SUBMISSION.md` §4](./SUBMISSION.md) for the full use cases.

---

## Repository contents

| Path | What it is |
|---|---|
| [`SUBMISSION.md`](./SUBMISSION.md) | The write-up (same content as the submitted Google Doc) |
| [`FINDINGS.md`](./FINDINGS.md) | The 8 findings, with versions, exact errors, request IDs, and fixes |
| `my-t3n-app/quickstart.ts` | The Node/TS client: connect → authenticate → TenantClient → register attempt |
| `contracts/z-remit-guard`, `contracts/z-credit-band` | The two additional TEE contracts (source + tests + WIT) |
| `artifacts/*.wasm` | All three compiled components |
| `evidence/logs/*.log` | Raw, unedited command output — the primary evidence |
| `evidence/screenshots/*.png` | Those logs rendered for readability, plus browser captures |
| `tools/render-terminal.js` | Renders a captured log to a terminal-styled page for screenshotting |
| `*.sh` | The setup/build/capture scripts, so the whole thing is reproducible |

Not included, by choice: `z-tenant-flight/` is an **unmodified** upstream clone
([Terminal-3/z-tenant-flight](https://github.com/Terminal-3/z-tenant-flight), v0.4.1) and
`docs/` is a local snapshot of Terminal 3's own documentation. Neither is our work.

> **On the screenshots:** the terminal PNGs are rendered from the raw logs in
> `evidence/logs/` rather than captured off a screen, so the text stays legible and
> selectable. The logs are the authoritative artifact — every number, DID, and request ID
> in the images comes from them verbatim. Nothing was hand-edited.

> **On the timestamps:** this host's system clocks drifted after a crash partway through the
> project, so evidence stamps are anchored to an authoritative HTTP `Date` header rather than
> the local clock. The capture scripts note this in each log header.

---

## Reproducing this

Prerequisites: WSL2 (or any Linux/macOS), and a T3N API key from the
[claim page](https://go.terminal3.io/adk-community).

```bash
git clone <this repo> && cd terminal3-adk-bounty
cp .env.example .env             # then paste your key into it

bash install-toolchain.sh        # rustup + wasm32-wasip2 + node (+ wasm-tools, optional)
bash build-contract.sh           # clones z-tenant-flight and builds the component
bash run-quickstart.sh           # connect, authenticate, attempt registration
bash build-extra-contracts.sh    # build + test the two additional contracts
bash capture-evidence.sh         # regenerate evidence/logs/01..04
bash capture-bonus-evidence.sh   # regenerate evidence/logs/05..06
bash render-evidence.sh          # logs -> evidence/html/, prints each page height
```

On a slow connection, use `build-contract-retry.sh` instead — it raises cargo's network
timeouts and splits `fetch` from `build` so a stall doesn't discard compile progress
(finding #3).

Both capture scripts refuse to finish if the API key ever appears in a captured log.

---

## The `script_name` blocker, briefly

```
RpcError: Invalid action request: missing field `script_name` at line 1 column 189
  rpcMethod: 'action.execute', httpStatus: -32602
  requestId: c18753da-5d80-4ab1-9676-1258623e4811
```

Both `tenant.contracts.register({tail, version, wasm})` and `tenant.tenant.me()` fail this
way. It does not appear to be a caller error:

- The documented `register` API has no parameter that could supply `script_name`.
- In the shipped `index.d.ts`, `script_name` occurs **only** inside `ChargeReason` — a
  `token.get-usage` *response* type. No request type models it.
- `script_name` and `scriptName` do not appear in `dist/index.esm.js` at all.
- `script_name` *is* documented for step 4 (`invoke`), where the caller passes it
  explicitly — so the field exists in the wire protocol; the control path just never sends it.

Reading: the deployed testnet node requires a field the latest published SDK does not send.
**The sponsor confirmed this on 2026-08-17 and said it is being fixed.**

Request IDs for tracing (3 runs, 6 failures):
`5fafa1df-f268-46a6-8407-308b0a1eaa5e`, `eb753b64-c2e6-4bfb-8d90-debbfd442097`,
`eee34a47-d631-47bf-9455-cc8f466dca30`, `c18753da-5d80-4ab1-9676-1258623e4811`,
`0b7d978c-4681-49cb-be4e-75e32f55895b`, `7df094fe-6697-4abd-9286-352a565f907d`
