import {
  T3nClient,
  TenantClient,
  setEnvironment,
  getNodeUrl,
  getEnvironmentName,
  loadWasmComponent,
  fetchTrustedManifest,
  eth_get_address,
  metamask_sign,
  createEthAuthInput,
} from "@terminal3/t3n-sdk";

// The SDK defaults to production — set this explicitly while building.
setEnvironment("testnet");

const T3N_API_KEY = process.env.T3N_API_KEY;
if (!T3N_API_KEY) {
  console.error("T3N_API_KEY is not set in the environment.");
  process.exit(1);
}

console.log("environment:", getEnvironmentName(), "| node:", getNodeUrl());

console.log("loading WASM component...");
const wasmComponent = await loadWasmComponent(); // all crypto runs inside this component

const address = eth_get_address(T3N_API_KEY);
console.log("eth address:", address);

// ---------------------------------------------------------------------------
// trustAnchor — REQUIRED by @terminal3/t3n-sdk >= 4.x, but absent from the
// documented quickstart sample. `fetchTrustedManifest` returns an
// operator-signed anchor verified against a key pinned inside the SDK; it
// never returns an unverified one. Prefer it over { unsafe_trust_server: true },
// which disables DKG attestation verification entirely.
// ---------------------------------------------------------------------------
let trustAnchor;
try {
  trustAnchor = await fetchTrustedManifest("testnet");
  console.log("trust anchor: fetched and signature-verified");
  console.log("  expected_peer_ids:", trustAnchor.expected_peer_ids?.length ?? 0);
  console.log("  rtmr3_allowlist:  ", trustAnchor.rtmr3_allowlist?.length ?? 0);
  if ((trustAnchor as { source?: unknown }).source) {
    console.log("  source:", JSON.stringify((trustAnchor as { source?: unknown }).source));
  }
} catch (err) {
  console.warn("\n!! fetchTrustedManifest('testnet') FAILED:");
  console.warn("  ", err instanceof Error ? `${err.name}: ${err.message}` : String(err));
  console.warn(
    "   Falling back to { unsafe_trust_server: true } to continue the walkthrough.\n" +
      "   This disables DKG attestation verification and is NOT acceptable for real use.\n",
  );
  trustAnchor = { unsafe_trust_server: true as const };
}

const t3n = new T3nClient({
  trustAnchor,
  wasmComponent,
  handlers: {
    EthSign: metamask_sign(address, undefined, T3N_API_KEY),
  },
});

console.log("handshaking...");
await t3n.handshake();

const did = await t3n.authenticate(createEthAuthInput(address));
const tenantDid = did.value; // did:t3n:... — never hardcode or derive this
console.log("Connected as:", tenantDid);

// --- from Set Up Dev Env: a TenantClient built on that session ---
const tenant = new TenantClient({
  t3n,
  baseUrl: getNodeUrl(),
  tenantDid,
});

// Docs say `await tenant.me()`, but in SDK 4.39.1 `me()` lives on the
// TenantNamespace exposed as `.tenant` — so it's `tenant.tenant.me()`.
// That call currently fails server-side (node rejects the SDK's own request
// with `missing field \`script_name\``), so it is non-fatal here: it is only a
// liveness check, and registration below is the operation that matters.
try {
  const whoami = await tenant.tenant.me();
  console.log("tenant.tenant.me() ->", JSON.stringify(whoami));
} catch (err) {
  const e = err as { name?: string; detail?: string; requestId?: string; message?: string };
  console.warn("!! tenant.tenant.me() failed (non-fatal, continuing):");
  console.warn(`   ${e.name ?? "Error"}: ${e.detail ?? e.message}`);
  if (e.requestId) console.warn(`   node requestId: ${e.requestId}`);
}
console.log("TenantClient ready.");

// ---------------------------------------------------------------------------
// Step 3 — register the compiled contract
// ---------------------------------------------------------------------------
const { readFile } = await import("node:fs/promises");

// The docs assume ../z-tenant-flight/target/wasm32-wasip2/release/*.wasm.
// We build with CARGO_TARGET_DIR on ext4 (the 9p mount is far slower), so the
// artifact is copied to ../artifacts/ instead.
const WASM_PATH = "../artifacts/z_tenant_flight.wasm";
const CONTRACT_TAIL = "flight";       // short on purpose — see the docs' tail-length note
const CONTRACT_VERSION = "0.1.0";

const wasmBytes = await readFile(WASM_PATH);
console.log(`\nregistering ${WASM_PATH} (${wasmBytes.length} bytes) as tail '${CONTRACT_TAIL}' v${CONTRACT_VERSION}...`);

const result = await tenant.contracts.register({
  tail: CONTRACT_TAIL,
  version: CONTRACT_VERSION,
  wasm: wasmBytes,
});

const contractId = result.contract_id;
const tenantId = tenantDid.slice("did:t3n:".length);
const scriptName = `z:${tenantId}:${CONTRACT_TAIL}`;
console.log(`registered ${scriptName} as contract id ${contractId}`);
console.log("full register result:", JSON.stringify(result));

// Persist the DID (not secret) so later steps and the write-up can reuse it.
const fs = await import("node:fs");
fs.writeFileSync(
  "did.json",
  JSON.stringify(
    {
      tenantDid,
      ethAddress: address,
      environment: getEnvironmentName(),
      nodeUrl: getNodeUrl(),
      trustAnchorMode: "unsafe_trust_server" in trustAnchor ? "UNSAFE_FALLBACK" : "verified-manifest",
      contract: { tail: CONTRACT_TAIL, version: CONTRACT_VERSION, contractId, scriptName },
    },
    null,
    2,
  ) + "\n",
);
console.log("wrote did.json");
