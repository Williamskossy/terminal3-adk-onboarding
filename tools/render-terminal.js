// Render a captured terminal log to a standalone HTML page styled as a terminal
// window, then print the pixel height needed to capture it in one shot.
//
//   node render-terminal.js <log> <out.html> "<window title>"
//
// Prints the required viewport height to stdout (for chrome --window-size).

const fs = require("fs");
const path = require("path");

const [logPath, outPath, titleArg] = process.argv.slice(2);
if (!logPath || !outPath) {
  console.error("usage: node render-terminal.js <log> <out.html> [title]");
  process.exit(1);
}

const raw = fs.readFileSync(logPath, "utf8").replace(/\s+$/, "");
const lines = raw.split("\n");
const title = titleArg || path.basename(logPath);

const esc = (s) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

function classify(line) {
  const t = line.trim();
  if (/^#/.test(t)) return "comment";
  if (/^\$ /.test(t) || /^\s*\$ /.test(line)) return "prompt";
  if (/RpcError|TypeError|T3nConfigError|FATAL|FAILED|^!!|\bError:/.test(line)) return "err";
  if (/^\s*!!/.test(line) || /^\s*Falling back/.test(line)) return "warn";
  if (/\bOK\b|-> WASM COMPONENT|signature-verified|Connected as:|registered |TenantClient ready|does not appear/.test(line))
    return "ok";
  if (/^\s*at /.test(line)) return "stack";
  if (/^===/.test(t)) return "section";
  return "";
}

function highlight(line) {
  let h = esc(line);
  // DIDs, eth addresses, request ids, versions
  h = h.replace(/(did:t3n:[0-9a-f]+)/g, '<span class="tok-did">$1</span>');
  h = h.replace(/(0x[0-9a-fA-F]{40})/g, '<span class="tok-addr">$1</span>');
  h = h.replace(
    /([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/g,
    '<span class="tok-id">$1</span>',
  );
  h = h.replace(/(`[^`]+`)/g, '<span class="tok-code">$1</span>');
  return h;
}

const body = lines
  .map((l) => {
    const cls = classify(l);
    return `<div class="ln${cls ? " " + cls : ""}">${highlight(l) || "&nbsp;"}</div>`;
  })
  .join("\n");

const LINE_H = 21;
const CHROME_H = 40; // title bar
const PAD_V = 40;

// Long lines wrap, so rendered rows > lines.length. Estimate wrapped rows from
// the content width at 13px monospace (~7.2px/char over 1056px of content box),
// deliberately conservative — over-estimating adds background-coloured space,
// under-estimating clips the last line.
const CHARS_PER_ROW = 128;
const rows = lines.reduce(
  (n, l) => n + Math.max(1, Math.ceil(l.length / CHARS_PER_ROW)),
  0,
);
const height = CHROME_H + PAD_V + rows * LINE_H + 24;

const html = `<!doctype html>
<html><head><meta charset="utf-8"><title>${esc(title)}</title>
<style>
  * { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; background: #0e1116; }
  .win {
    width: 1100px; margin: 0 auto;
    background: #0d1117;
    border: 1px solid #262c36;
    border-radius: 8px;
    overflow: hidden;
    font-family: "Cascadia Mono", "Consolas", "DejaVu Sans Mono", monospace;
  }
  .bar {
    height: ${CHROME_H}px; display: flex; align-items: center; gap: 8px;
    padding: 0 14px; background: #161b22; border-bottom: 1px solid #262c36;
  }
  .dot { width: 11px; height: 11px; border-radius: 50%; }
  .d1 { background: #ff5f57; } .d2 { background: #febc2e; } .d3 { background: #28c840; }
  .bar .t {
    margin-left: 10px; color: #8b949e; font-size: 12.5px; letter-spacing: .02em;
  }
  .body { padding: 20px 22px; }
  .ln {
    color: #c9d1d9; font-size: 13px; line-height: ${LINE_H}px;
    white-space: pre-wrap; word-break: break-word;
  }
  .comment { color: #6e7681; }
  .prompt  { color: #79c0ff; font-weight: 600; }
  .section { color: #d2a8ff; font-weight: 600; }
  .ok      { color: #56d364; }
  .warn    { color: #e3b341; }
  .err     { color: #ff7b72; }
  .stack   { color: #6e7681; font-size: 12px; }
  .tok-did  { color: #56d364; }
  .tok-addr { color: #79c0ff; }
  .tok-id   { color: #ffa657; }
  .tok-code { color: #a5d6ff; }
</style></head>
<body>
  <div class="win">
    <div class="bar">
      <span class="dot d1"></span><span class="dot d2"></span><span class="dot d3"></span>
      <span class="t">${esc(title)}</span>
    </div>
    <div class="body">
${body}
    </div>
  </div>
</body></html>
`;

fs.writeFileSync(outPath, html);
process.stdout.write(String(height));
