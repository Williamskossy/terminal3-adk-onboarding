# Terminal 3 ADK — findings log

Recorded live while working through the ADK docs on **Windows 11 + WSL2 (Ubuntu 24.04)**.
Each entry: what I did, what I expected, what happened, and the fix that worked.

Environment:
- Windows 11 Pro 22621, WSL2 Ubuntu 24.04.3 LTS
- rustc 1.97.1, cargo 1.97.1, `wasm32-wasip2` target
- Node v22.23.2 (nvm), npm 10.9.8
- Slow/jittery internet link (relevant to #3)

---

## 1. No Windows instructions anywhere in the Get Started path — `severity: medium`

**Where:** `get-started/quickstart`, `get-started/prerequisites/set-up-dev-env`

Every command in the onboarding path is bash-only:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
export T3N_API_KEY="<key>"
source "$HOME/.cargo/env"
```

On Windows none of these work. `curl | sh` has no shell to pipe to, `export` is
`$env:T3N_API_KEY = "..."` in PowerShell, and `source` doesn't exist. Windows
users must independently know to install `rustup-init.exe` — or, as I did,
discover that WSL is the smoother path.

There is a second, subtler Windows trap: **WSL inherits the Windows `PATH`**, so
inside a fresh WSL shell `npm` resolves to the *Windows* `npm.cmd` via `/mnt/c`
while `node` appears missing entirely. Following the quickstart in that state
produces confusing failures that look like SDK problems.

**Suggested fix:** add a short "On Windows" note to the prerequisites page — either
`rustup-init.exe` + PowerShell equivalents, or an explicit "use WSL2, and install
Node *inside* WSL" recommendation. The latter is one sentence and avoids the
whole class of problem.

---

## 2. Documented repo structure doesn't match the actual repo — `severity: low`

**Where:** `get-started/walkthrough/write-contract`, "Repository Structure"

The docs show `z-tenant-flight/` as:

```
├── src/{lib.rs, search.rs, booking.rs}
├── wit/{world.wit, deps/}
└── Cargo.toml
```

and state the vendored deps are "(host-interfaces, host-tenant)" — "here,
`host-interfaces-2.1.0/` and `host-tenant-1.0.0/`".

The actual clone (`--depth 1`, today) also contains:

- **`.cargo/config.toml`** — sets `build.target = "wasm32-wasip2"`. Not mentioned
  anywhere, but it's why `cargo build` works without `--target` in that repo.
  Worth documenting, since a reader copying the layout into their own project
  and omitting this file gets a native build instead of a WASM one.
- **`wit/deps/host-outbox-1.0.0/`** — a third vendored host package the prose
  doesn't list.
- `README.md`

**Suggested fix:** regenerate the tree from the real repo and add one line on what
`.cargo/config.toml` does.

---

## 3. `cargo build` aborts on a slow connection with no guidance — `severity: low` (docs), high impact regionally

**Where:** `get-started/walkthrough/build-contract`

First build attempt died after 8m21s:

```
warning: spurious network error (3 tries remaining): transfer too slow:
         failed to transfer more than 10 bytes in 30s (transferred 0 bytes)
error: failed to download from `https://static.crates.io/crates/wit-component/0.243.0/download`
```

12 of ~13 crates downloaded, then one stalled. This is cargo's default
`http.low-speed-limit` (10 bytes/s over 30s) firing on a slow link — not a T3N
issue, but it *presents* as "the ADK build is broken", and anyone onboarding from
a region with a slow link to crates.io will hit it.

**Fix that worked:**

```bash
export CARGO_NET_RETRY=10
export CARGO_HTTP_TIMEOUT=300
export CARGO_HTTP_LOW_SPEED_LIMIT=1
export CARGO_HTTP_MULTIPLEXING=false
cargo fetch --target wasm32-wasip2      # fetch separately, retry-able
cargo build --target wasm32-wasip2 --release --offline
```

Splitting `fetch` from `build` matters: a stall during a combined run discards
compile progress too.

**Suggested fix:** a short troubleshooting note on the build page, or an entry in
`tips/common-errors`. Cheap to add, saves a first-time builder a confusing 8-minute
failure.

---

## 4. `wasm-tools` is "optional" on one page and load-bearing on the next — `severity: trivial`

**Where:** `prerequisites/set-up-dev-env` vs `walkthrough/build-contract`

Set-up marks it optional:

```bash
cargo install wasm-tools   # optional — inspect/verify the component
```

But build-contract's "Verify the component interface" section is written as a
normal step and only mentions installing it *after* showing the command that
needs it. It also warns the install "takes about 2 minutes with no progress
output" — which is exactly the kind of thing worth knowing *before* you're
mid-walkthrough.

**Suggested fix:** either promote it to a real prerequisite, or label the verify
section "optional" to match.

**Aside:** you can confirm you got a component rather than a bare core module
without installing anything, by checking the 8-byte header:

```bash
xxd -l 8 -p target/wasm32-wasip2/release/*.wasm
# 0061736d0d000100  -> component (layer=1)
# 0061736d01000000  -> bare core module (layer=0)
```

---

---

## 5. The documented Quickstart code does not run — `trustAnchor` is required and never mentioned — `severity: BLOCKING`

**Where:** `get-started/quickstart`, step 3 ("Connect and authenticate")
**Versions:** `@terminal3/t3n-sdk` 4.39.1 (latest published), node cluster `testnet` manifest `1786457685`

Copy-pasting the documented `new T3nClient({ wasmComponent, handlers })` sample fails
immediately:

```
T3nConfigError: T3nClient: `trustAnchor` is required and must be either a TrustAnchor
({ expected_peer_ids, rtmr3_allowlist }) that pins the node's DKG attestation, or the
explicit opt-out { unsafe_trust_server: true }. The unsafe option disables attestation
verification (local dev / mock-signer nodes only) — never use it against a real node.
  code: 'CONFIG_ERROR', field: 'trustAnchor'
```

`trustAnchor` appears **nowhere** in any of the 13 ADK doc pages — not in the quickstart,
not in the reference, not in `tips/common-errors`. The SDK's own `README.md` documents the
correct usage, but nothing in the docs site points a reader there.

**Fix that worked** — the SDK exports a helper that returns an operator-signed anchor,
verified against a key pinned inside the package:

```typescript
import { fetchTrustedManifest } from "@terminal3/t3n-sdk";

const t3n = new T3nClient({
  trustAnchor: await fetchTrustedManifest("testnet"),   // "sandbox" | "testnet" | "production"
  wasmComponent,
  handlers: { EthSign: metamask_sign(address, undefined, T3N_API_KEY) },
});
```

Confirmed working — anchor fetched from `<node>/api/trust-manifest`, signature verified,
3 `expected_peer_ids`, 1 `rtmr3_allowlist` entry, manifest `1786457685` signed
`2026-08-11T14:14:45Z`. Handshake and `authenticate()` then succeed.

**Why this matters:** this is the first code sample a new developer runs, and it cannot
work as written. It's also a security-relevant omission — a developer who hits the error
and reaches for the `{ unsafe_trust_server: true }` arm suggested in the message (the
obvious quick unblock) silently disables DKG attestation verification against a real node.
The docs should show `fetchTrustedManifest` so nobody reasons their way to the unsafe path.

**Suggested fix:** add `trustAnchor: await fetchTrustedManifest("testnet")` to the quickstart
sample, with one line on what it does and an explicit "do not use `unsafe_trust_server`
against testnet or production."

---

## 6. `tenant.me()` doesn't exist — it's `tenant.tenant.me()` — `severity: medium`

**Where:** `get-started/prerequisites/set-up-dev-env`, step 3; also referenced in
`walkthrough/register-contract`'s troubleshooting table ("Confirm with `tenant.me()`")

The documented verification line:

```typescript
await tenant.me(); // throws if something's wrong; confirms the client actually works
```

fails with:

```
TypeError: tenant.me is not a function
```

In SDK 4.39.1, `me()` is declared on `TenantNamespace`, which `TenantClient` exposes as the
readonly property `.tenant`. So the call is `tenant.tenant.me()`. (`TenantClient`'s own
surface is `config`, `tenant`, `maps`, `contracts`, `token`, plus `admitForOrg`,
`getEnvironment`, `canonicalName`, `controlPayload`, `executeControl`,
`executeBusinessContract`.)

The doubled name reads like a typo, which is probably how the docs came to drop one — worth
either documenting explicitly or adding a `me()` delegate on `TenantClient`.

**Note:** correcting the call gets past the `TypeError` but then hits finding #7 below, so
this verification step cannot currently succeed by any spelling.

---

## 7. Every control RPC is rejected by the node: `missing field 'script_name'` — `severity: BLOCKING`

**Where:** `prerequisites/set-up-dev-env` (`tenant.tenant.me()`) and
`walkthrough/register-contract` (`tenant.contracts.register`)
**Versions:** `@terminal3/t3n-sdk` 4.39.1 (latest published — 4.39.1 is the newest of 4.32.0→4.39.1),
node cluster `testnet`, trust manifest `1786457685` signed `2026-08-11T14:14:45Z`,
node `https://cn-api.sg.testnet.t3n.terminal3.io`

Both control-plane calls fail server-side with the same complaint, at different payload
offsets:

```
RpcError: Invalid action request: missing field `script_name` at line 1 column 105
  rpcMethod: 'action.execute', httpStatus: -32602
  requestId: 5fafa1df-f268-46a6-8407-308b0a1eaa5e      <- tenant.tenant.me()

RpcError: Invalid action request: missing field `script_name` at line 1 column 189
  rpcMethod: 'action.execute', httpStatus: -32602
  requestId: eb753b64-c2e6-4bfb-8d90-debbfd442097      <- tenant.contracts.register({tail, version, wasm})
```

Both request IDs should be traceable in node logs.

**This blocks the bounty task itself** — the contract compiles to a valid WASM component
(194 KB, header `0061736d0d000100`, layer=1) but cannot be registered, so walkthrough steps
3, 4 and 5 are all unreachable.

**Evidence it is not a caller mistake:**

- The documented register API is `{ tail, version, wasm }` — there is no documented
  parameter that would supply `script_name`.
- In the shipped `index.d.ts`, `script_name` occurs only inside `ChargeReason`, a *response*
  type for `token.get-usage`. No request type models it.
- The strings `script_name` and `scriptName` do not appear in `dist/index.esm.js` at all
  (`script` appears 3 times). *Caveat:* the bundle is string-array obfuscated, so a literal
  could in principle be encoded — but combined with the type definitions and the fact that
  both control operations fail identically, the SDK's control path evidently doesn't send it.
- `TenantClientConfig` has an undocumented `tenantContractId?: string`, which I left unset.
  If that is what populates `script_name`, it is documented nowhere and the walkthrough
  never sets it — which is itself the bug.
- `script_name` *is* documented for step 4 (`invoke`), where the caller passes it explicitly.
  So the field exists in the wire protocol; the control path just doesn't populate it.

**Reading:** a wire-format skew between the latest published SDK and the currently deployed
testnet node — the node requires a field the SDK doesn't send. Nothing a developer following
the walkthrough can work around from the client side.

**What would unblock it:** either a node build that treats `script_name` as optional for
control actions, a published SDK that sends it, or documentation of the config field that
supplies it.

<!-- Append further findings below as they occur. Keep the format:
     what I did / expected / got / fix. -->
