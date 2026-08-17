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
so walkthrough steps 3–5 cannot be completed by any client-side means. Full evidence below.

| Stage | Result |
|---|---|
| Claim API key + DID (SSO) | ✅ |
| Quickstart — connect & authenticate | ✅ `did:t3n:039b9146…` issued |
| Set Up Dev Env — Rust + WASM toolchain | ✅ |
| Set Up Dev Env — `TenantClient` | ⚠️ built, but its documented verification call is broken (findings #6, #7) |
| Walkthrough 1 — write contract | ✅ |
| Walkthrough 2 — build contract | ✅ 194 KB component, header `0061736d0d000100` |
| Walkthrough 3 — register contract | ❌ **blocked** — node rejects the SDK's own request |
| Walkthrough 4 — invoke contract | ⬜ unreachable |
| Walkthrough 5 — test | ⬜ unreachable |

**7 findings** are documented in [`FINDINGS.md`](./FINDINGS.md) — 2 blocking, 1 security-relevant.

Headline items:

1. **The documented Quickstart code cannot run.** `T3nClient` requires a `trustAnchor`
   argument that appears on no ADK documentation page. Fix: `fetchTrustedManifest("testnet")`.
   Security-relevant, because the error message's suggested alternative
   (`{ unsafe_trust_server: true }`) silently disables DKG attestation verification.
2. **Every control RPC is rejected by the node** with `Invalid action request: missing field
   'script_name'`. Reproduced 4 times across 2 runs, each with a server-side request ID.
3. `tenant.me()` from the docs doesn't exist — it's `tenant.tenant.me()`.

---

## Repository contents

| Path | What it is |
|---|---|
| [`FINDINGS.md`](./FINDINGS.md) | The 7 findings, with versions, exact errors, request IDs, and fixes |
| [`SUBMISSION.md`](./SUBMISSION.md) | The write-up (same content as the submitted Google Doc) |
| `my-t3n-app/quickstart.ts` | The Node/TS client: connect → authenticate → TenantClient → register attempt |
| `artifacts/z_tenant_flight.wasm` | The compiled contract component (194 KB) |
| `evidence/logs/*.log` | Raw, unedited command output — the primary evidence |
| `evidence/screenshots/*.png` | The same logs rendered for readability, plus browser captures |
| `tools/render-terminal.js` | Renders a captured log to a terminal-styled page for screenshotting |
| `*.sh` | The setup/build/capture scripts, so the whole thing is reproducible |

Not included, by choice: `z-tenant-flight/` is an **unmodified** upstream clone
([Terminal-3/z-tenant-flight](https://github.com/Terminal-3/z-tenant-flight), v0.4.1) and
`docs/` is a local snapshot of Terminal 3's own documentation. Neither is our work.

> **On the screenshots:** the terminal PNGs are rendered from the raw logs in
> `evidence/logs/` rather than captured off a screen, so the text stays legible and
> selectable. The logs are the authoritative artifact — every number, DID, and request ID
> in the images comes from them verbatim. Nothing was hand-edited.

---

## Reproducing this

Prerequisites: WSL2 (or any Linux/macOS), and a T3N API key from the
[claim page](https://go.terminal3.io/adk-community).

```bash
git clone <this repo> && cd terminal3-adk-bounty
cp .env.example .env          # then paste your key into it

bash install-toolchain.sh     # rustup + wasm32-wasip2 + node + wasm-tools
bash build-contract.sh        # clones z-tenant-flight and builds the component
bash run-quickstart.sh        # connect, authenticate, attempt registration
bash capture-evidence.sh      # regenerate evidence/logs/
```

On a slow connection, use `build-contract-retry.sh` instead — it raises cargo's network
timeouts and splits `fetch` from `build` so a stall doesn't discard compile progress
(finding #3).

`capture-evidence.sh` refuses to finish if the API key ever appears in a captured log.

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

Request IDs for tracing (2 runs):
`5fafa1df-f268-46a6-8407-308b0a1eaa5e`, `eb753b64-c2e6-4bfb-8d90-debbfd442097`,
`eee34a47-d631-47bf-9455-cc8f466dca30`, `c18753da-5d80-4ab1-9676-1258623e4811`
