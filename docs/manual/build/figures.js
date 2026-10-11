// Turns the raw screenshots in docs/manual/screenshots/<lang>/ into the figures used by the manual
// (figures/<lang>/<name>.jpg): crops the region that matters, rounds the corners and
// adds a soft shadow on a white background.
//
// Usage: node figures.js <lang>
const fs = require('fs');
const path = require('path');
const { PNG } = require('pngjs');
const jpeg = require('jpeg-js');

const lang = process.argv[2];
const src = path.join(__dirname, '..', 'screenshots', lang);
const dst = path.join(__dirname, 'figures', lang);
fs.mkdirSync(dst, { recursive: true });

// Screenshots are taken at 1.5× scale; `crop` is in physical pixels [x, y, w, h].
const FIGURES = {
  welcome: {},
  'tray-menu': { crop: [1050, 300, 870, 780] },
  // menus: desktop at logical (560, 150), image at (120, 250); 300 wide
  'shell-desktop': { crop: [690, 180, 780, 700] },
  'shell-image': { crop: [0, 300, 780, 760] },
  'scan-screen': { crop: [0, 0, 1920, 1080] },
  'scan-region': { crop: [0, 0, 1920, 1080] },
  'detail-wifi': { crop: [520, 290, 880, 500] },
  'detail-link': { crop: [520, 290, 880, 500] },
  'detail-warning': { crop: [520, 270, 880, 540] },
  'detail-danger': { crop: [520, 290, 880, 500] },
  camera: {},
  'image-preview': {},
  'toolbar-row': { crop: [480, 0, 960, 300] },
  'toolbar-right': { crop: [1260, 90, 660, 945] },
  'toolbar-left': { crop: [0, 90, 660, 945] },
  bubble: { crop: [1380, 520, 540, 300] },
  generator: {},
  'settings-general': {},
  'settings-scanning': {},
  'settings-camera': {},
  'settings-shortcuts': {},
  'settings-privacy': {},
  'settings-about': {},
  'message-update': {},
};

function load(file) {
  const p = PNG.sync.read(fs.readFileSync(file));
  return { width: p.width, height: p.height, data: p.data };
}

function crop(img, [x, y, w, h]) {
  w = Math.min(w, img.width - x);
  h = Math.min(h, img.height - y);
  const out = { width: w, height: h, data: Buffer.alloc(w * h * 4) };
  for (let r = 0; r < h; r++) img.data.copy(out.data, r * w * 4, ((y + r) * img.width + x) * 4, ((y + r) * img.width + x + w) * 4);
  return out;
}

// Coverage (0..1) of a pixel centre inside a rounded rectangle.
function inside(px, py, w, h, r) {
  const cx = Math.min(Math.max(px, r), w - r);
  const cy = Math.min(Math.max(py, r), h - r);
  const d = Math.hypot(px - cx, py - cy);
  return Math.min(1, Math.max(0, r + 0.5 - d));
}

// Distance from a point to a rounded rectangle (0 inside).
function distance(px, py, x0, y0, w, h, r) {
  const qx = Math.abs(px - (x0 + w / 2)) - (w / 2 - r);
  const qy = Math.abs(py - (y0 + h / 2)) - (h / 2 - r);
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}

function frame(img, { radius = 14, margin = 36, blur = 20, offset = 6, opacity = 0.13, border = [206, 211, 220] } = {}) {
  const W = img.width + margin * 2;
  const H = img.height + margin * 2;
  const out = Buffer.alloc(W * H * 3, 255);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const o = (y * W + x) * 3;
      const ix = x - margin + 0.5;
      const iy = y - margin + 0.5;
      // shadow
      const d = distance(x + 0.5, y + 0.5 - offset, margin, margin, img.width, img.height, radius);
      let shade = d <= 0 ? opacity : opacity * Math.exp(-((d / blur) ** 2) * 2.2);
      let r = 255 * (1 - shade);
      let g = r;
      let b = r;
      const cov = ix >= 0 && iy >= 0 && ix <= img.width && iy <= img.height ? inside(ix, iy, img.width, img.height, radius) : 0;
      if (cov > 0) {
        const sx = Math.min(img.width - 1, Math.max(0, Math.floor(ix)));
        const sy = Math.min(img.height - 1, Math.max(0, Math.floor(iy)));
        const s = (sy * img.width + sx) * 4;
        // 1 px border on the edge of the rounded shape
        const edge = distance(ix, iy, 0, 0, img.width, img.height, radius) > -1.2;
        const pr = edge ? border[0] : img.data[s];
        const pg = edge ? border[1] : img.data[s + 1];
        const pb = edge ? border[2] : img.data[s + 2];
        r = r * (1 - cov) + pr * cov;
        g = g * (1 - cov) + pg * cov;
        b = b * (1 - cov) + pb * cov;
      }
      out[o] = r;
      out[o + 1] = g;
      out[o + 2] = b;
    }
  }
  return { width: W, height: H, rgb: out };
}

function saveJpeg({ width, height, rgb }, file) {
  const data = Buffer.alloc(width * height * 4);
  for (let i = 0, j = 0; i < rgb.length; i += 3, j += 4) {
    data[j] = rgb[i];
    data[j + 1] = rgb[i + 1];
    data[j + 2] = rgb[i + 2];
    data[j + 3] = 255;
  }
  fs.writeFileSync(file, jpeg.encode({ data, width, height }, 90).data);
}

for (const [name, spec] of Object.entries(FIGURES)) {
  let img = load(path.join(src, `${name}.png`));
  if (spec.crop) img = crop(img, spec.crop);
  const out = frame(img);
  saveJpeg(out, path.join(dst, `${name}.jpg`));
  console.log(`${lang} ${name} ${out.width}x${out.height}`);
}
