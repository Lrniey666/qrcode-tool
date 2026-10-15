// Cover and back-cover artwork (A4, vector). Text is added by Word on top of it, so the
// artwork is the same for every language. Writes art/cover.svg, art/back.svg and PNG
// fallbacks rendered by Slint (older Word versions cannot show SVG).
//
// Usage (SLINT_SCALE_FACTOR=1.5): node art.js
const fs = require('fs');
const path = require('path');
const { execFile } = require('child_process');
const QRCode = require('qrcode');

const out = path.join(__dirname, 'art');
fs.mkdirSync(out, { recursive: true });

const W = 2100; // 0.1 mm units: A4 = 210 × 297 mm
const H = 2970;
const BRAND = `
  <linearGradient id="brand" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#1D4FFF"/>
    <stop offset="46%" stop-color="#159FF0"/>
    <stop offset="100%" stop-color="#19DFC2"/>
  </linearGradient>
  <linearGradient id="line" x1="0" y1="0" x2="1" y2="0">
    <stop offset="0%" stop-color="#1D4FFF"/>
    <stop offset="100%" stop-color="#19DFC2"/>
  </linearGradient>`;

// The app logo (scan brackets around a finder pattern), drawn in a 1254 box.
function logo(x, y, size, colorMain = 'url(#brand)', colorBrackets = 'url(#brand)') {
  const k = size / 1254;
  return `<g transform="translate(${x} ${y}) scale(${k})">
    <path d="M450 235 H345 C279 235 225 289 225 355 V455" fill="none" stroke="${colorBrackets}" stroke-width="82" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M805 235 H905 C971 235 1025 289 1025 355 V455" fill="none" stroke="${colorBrackets}" stroke-width="82" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M225 765 V865 C225 931 279 985 345 985 H450" fill="none" stroke="${colorBrackets}" stroke-width="82" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M805 985 H905 C971 985 1025 931 1025 865 V765" fill="none" stroke="${colorBrackets}" stroke-width="82" stroke-linecap="round" stroke-linejoin="round"/>
    <rect x="355" y="375" width="350" height="350" rx="63" fill="none" stroke="${colorMain}" stroke-width="70"/>
    <rect x="430" y="450" width="180" height="180" rx="34" fill="${colorMain}"/>
  </g>`;
}

// QR code modules as one path (scanning the cover opens the project page).
function qrPath(text, x, y, size) {
  const q = QRCode.create(text, { errorCorrectionLevel: 'M' });
  const n = q.modules.size;
  const m = size / n;
  let d = '';
  for (let r = 0; r < n; r++)
    for (let c = 0; c < n; c++) if (q.modules.get(r, c)) d += `M${(x + c * m).toFixed(2)} ${(y + r * m).toFixed(2)}h${m.toFixed(2)}v${m.toFixed(2)}h-${m.toFixed(2)}z`;
  return `<path d="${d}" fill="#111827"/>`;
}

// Faint grid of rounded "modules" used as texture on the gradient.
function texture(x0, y0, cols, rows, cell, seed) {
  let s = seed;
  const rnd = () => ((s = (s * 1103515245 + 12345) % 2147483648) / 2147483648);
  let out = '';
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) {
      const v = rnd();
      if (v < 0.42) continue;
      const fade = 1 - (c / cols) * 0.5 - (r / rows) * 0.3;
      out += `<rect x="${x0 + c * cell}" y="${y0 + r * cell}" width="${cell * 0.78}" height="${cell * 0.78}" rx="${cell * 0.18}" fill="#ffffff" opacity="${(0.05 + v * 0.1 * fade).toFixed(3)}"/>`;
    }
  return out;
}

function cover() {
  const card = { x: 1250, y: 1330, s: 640 };
  const q = { x: card.x + 110, y: card.y + 110, s: card.s - 220 };
  const br = 70; // bracket length
  const bx0 = q.x - 46;
  const by0 = q.y - 46;
  const bx1 = q.x + q.s + 46;
  const by1 = q.y + q.s + 46;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="210mm" height="297mm">
  <defs>${BRAND}</defs>
  <rect width="${W}" height="${H}" fill="#ffffff"/>
  <path d="M0 0 H${W} V1700 C1650 1880 900 1960 0 1880 Z" fill="url(#brand)"/>
  <circle cx="1820" cy="260" r="620" fill="#ffffff" opacity="0.07"/>
  <circle cx="1960" cy="1180" r="380" fill="#ffffff" opacity="0.06"/>
  ${texture(1180, 140, 12, 9, 72, 7)}
  <rect x="190" y="200" width="250" height="250" rx="58" fill="#ffffff"/>
  ${logo(190, 200, 250)}
  <!-- scanned card with brackets, as in the app -->
  <rect x="${card.x + 18}" y="${card.y + 26}" width="${card.s}" height="${card.s}" rx="56" fill="#0b1f4d" opacity="0.18"/>
  <rect x="${card.x}" y="${card.y}" width="${card.s}" height="${card.s}" rx="56" fill="#ffffff"/>
  ${qrPath('https://github.com/naritaroad/qrcode-tool', q.x, q.y, q.s)}
  <g fill="none" stroke="#1D4FFF" stroke-width="16" stroke-linecap="round" stroke-linejoin="round">
    <path d="M${bx0} ${by0 + br} V${by0 + 24} Q${bx0} ${by0} ${bx0 + 24} ${by0} H${bx0 + br}"/>
    <path d="M${bx1 - br} ${by0} H${bx1 - 24} Q${bx1} ${by0} ${bx1} ${by0 + 24} V${by0 + br}"/>
    <path d="M${bx0} ${by1 - br} V${by1 - 24} Q${bx0} ${by1} ${bx0 + 24} ${by1} H${bx0 + br}"/>
    <path d="M${bx1 - br} ${by1} H${bx1 - 24} Q${bx1} ${by1} ${bx1} ${by1 - 24} V${by1 - br}"/>
  </g>
  <!-- result label -->
  <rect x="${card.x - 150}" y="${card.y + card.s - 120}" width="560" height="120" rx="60" fill="#1f1f1f" opacity="0.96"/>
  <circle cx="${card.x - 82}" cy="${card.y + card.s - 60}" r="26" fill="none" stroke="#ffffff" stroke-width="7"/>
  <path d="M${card.x - 108} ${card.y + card.s - 60} H${card.x - 56} M${card.x - 82} ${card.y + card.s - 86} C${card.x - 96} ${card.y + card.s - 70} ${card.x - 96} ${card.y + card.s - 50} ${card.x - 82} ${card.y + card.s - 34} M${card.x - 82} ${card.y + card.s - 86} C${card.x - 68} ${card.y + card.s - 70} ${card.x - 68} ${card.y + card.s - 50} ${card.x - 82} ${card.y + card.s - 34}" fill="none" stroke="#ffffff" stroke-width="6"/>
  <rect x="${card.x - 30}" y="${card.y + card.s - 82}" width="290" height="26" rx="13" fill="#ffffff"/>
  <rect x="${card.x - 30}" y="${card.y + card.s - 44}" width="190" height="18" rx="9" fill="#ffffff" opacity="0.55"/>
  <circle cx="${card.x + 350}" cy="${card.y + card.s - 60}" r="40" fill="#ffffff" opacity="0.12"/>
  <path d="M${card.x + 340} ${card.y + card.s - 78} L${card.x + 360} ${card.y + card.s - 60} L${card.x + 340} ${card.y + card.s - 42}" fill="none" stroke="#ffffff" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
  <rect x="190" y="2790" width="1720" height="10" rx="5" fill="url(#line)"/>
</svg>`;
}

function back() {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="210mm" height="297mm">
  <defs>${BRAND}</defs>
  <rect width="${W}" height="${H}" fill="#ffffff"/>
  <rect x="190" y="170" width="1720" height="10" rx="5" fill="url(#line)"/>
  <path d="M0 ${H} H${W} V2240 C1500 2120 700 2140 0 2290 Z" fill="url(#brand)"/>
  <circle cx="260" cy="2900" r="420" fill="#ffffff" opacity="0.06"/>
  ${texture(1300, 2400, 10, 7, 72, 3)}
  <rect x="${W / 2 - 150}" y="2480" width="300" height="300" rx="70" fill="#ffffff"/>
  ${logo(W / 2 - 150, 2480, 300)}
</svg>`;
}

fs.writeFileSync(path.join(out, 'cover.svg'), cover());
fs.writeFileSync(path.join(out, 'back.svg'), back());
console.log('svg written');

// ── PNG fallbacks rendered by Slint ──
const slint = require('slint-ui');
const CAPTURE = path.join(__dirname, 'capture.exe');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const capture = (file) =>
  new Promise((resolve, reject) =>
    execFile(CAPTURE, [String(process.pid), file, 'client'], (e, so, se) => (e ? reject(new Error(se || e.message)) : resolve(so.trim()))),
  );
const source = (name) => `export component Art inherits Window {
  no-frame: true;
  width: 480px;
  height: 679px;
  Image { width: 100%; height: 100%; source: @image-url("${name}.svg"); }
}`;

async function main() {
  for (const name of ['cover', 'back']) {
    const ui = slint.loadSource(source(name), path.join(out, `${name}.slint`));
    const w = new ui.Art();
    w.window.logicalPosition = { x: 0, y: 0 };
    w.show();
    await sleep(900);
    console.log(name, await capture(path.join(out, `${name}.png`)));
    w.hide();
  }
}
slint.runEventLoop({ runningCallback: () => main().then(() => process.exit(0), (e) => { console.error(e); process.exit(1); }), quitOnLastWindowClosed: false });
