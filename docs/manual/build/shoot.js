// Renders the app's real Slint windows (baked in one language) with demo data and saves
// screenshots for the user manual (docs/manual/screenshots/<lang>/<name>.png, not in git).
//
// Usage (SLINT_SCALE_FACTOR=1.5): node shoot.js <repo> <lang> <outDir> [--all | --only=name,name]
//   default    take only the screenshots that are missing
//   --only=…   retake these (plus anything missing); see NAMES for the list
//   --all      retake everything
// Expects baked/<lang>/ from bake.js and capture.exe next to this script.
const fs = require('fs');
const path = require('path');
const { execFile } = require('child_process');
const slint = require('slint-ui');
const { PNG } = require('pngjs');
const QRCode = require('qrcode');

const [repo, lang, outDir, ...flags] = process.argv.slice(2);
const SCALE = 1.5;
fs.mkdirSync(outDir, { recursive: true });

// Every screenshot, in the order they are taken.
const NAMES = [
  'desktop', 'scan-screen', 'scan-region', 'detail-wifi', 'detail-link', 'detail-danger', 'detail-warning',
  'toolbar-row', 'toolbar-right', 'toolbar-left', 'bubble', 'tray-menu', 'shell-desktop', 'shell-image',
  'camera-source', 'camera', 'image-preview', 'generator', 'welcome',
  'settings-general', 'settings-scanning', 'settings-camera', 'settings-shortcuts', 'settings-privacy', 'settings-about',
  'message-update',
];
// Screenshots drawn on top of another one are retaken whenever that one is.
const DEPENDS = {
  desktop: ['scan-screen', 'scan-region', 'detail-wifi', 'detail-link', 'detail-danger', 'detail-warning', 'image-preview'],
  'camera-source': ['camera'],
};
const ALL = flags.includes('--all');
const ONLY = new Set(
  flags
    .filter((f) => f.startsWith('--only='))
    .flatMap((f) => f.slice(7).split(','))
    .map((s) => s.trim())
    .filter(Boolean),
);
for (const n of ONLY) if (!NAMES.includes(n)) throw new Error(`unknown screenshot "${n}". Known: ${NAMES.join(', ')}`);
const retaken = new Set();
function wanted(name) {
  if (ALL || ONLY.has(name) || !fs.existsSync(path.join(outDir, `${name}.png`))) return true;
  return Object.entries(DEPENDS).some(([parent, kids]) => kids.includes(name) && retaken.has(parent));
}
const CAPTURE = path.join(__dirname, 'capture.exe');
const baked = path.join(__dirname, 'baked', lang);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── translations for data that Rust would normally fill in ──
const po = new Map();
if (lang !== 'en') {
  const text = fs.readFileSync(path.join(repo, 'app', 'translations', lang, 'LC_MESSAGES', 'qrcode-tool.po'), 'utf8').replace(/\r\n/g, '\n');
  const un = (s) => s.replace(/\\(.)/g, (m, c) => ({ n: '\n', '"': '"', '\\': '\\' }[c] ?? c));
  const join = (b) => [...b.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => un(m[1])).join('');
  for (const m of text.matchAll(/^msgid ((?:".*"\n)+)^msgstr ((?:".*"\n?)+)/gm)) {
    const id = join(m[1]);
    const s = join(m[2]);
    if (id && s && !po.has(id)) po.set(id, s);
  }
}
const T = (id, ...args) => {
  let s = po.get(id) ?? id;
  for (const a of args) s = s.replace('{}', a);
  return s;
};
const L = (en, zh) => (lang === 'zh-TW' ? zh : en);

// ── images ──
function qrImage(text, px = 6, margin = 4) {
  const q = QRCode.create(text, { errorCorrectionLevel: 'M' });
  const n = q.modules.size;
  const size = (n + margin * 2) * px;
  const data = new Uint8Array(size * size * 4).fill(255);
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++)
      if (q.modules.get(y, x))
        for (let dy = 0; dy < px; dy++)
          for (let dx = 0; dx < px; dx++) {
            const o = (((y + margin) * px + dy) * size + (x + margin) * px + dx) * 4;
            data[o] = data[o + 1] = data[o + 2] = 0;
          }
  return { width: size, height: size, data, modules: n };
}
function loadPng(file) {
  const p = PNG.sync.read(fs.readFileSync(file));
  return { width: p.width, height: p.height, data: new Uint8Array(p.data) };
}
function crop(img, x, y, w, h) {
  const out = { width: w, height: h, data: new Uint8Array(w * h * 4) };
  for (let r = 0; r < h; r++) {
    const src = ((y + r) * img.width + x) * 4;
    out.data.set(img.data.subarray(src, src + w * 4), r * w * 4);
  }
  return out;
}
function savePng(img, file) {
  const p = new PNG({ width: img.width, height: img.height });
  p.data = Buffer.from(img.data);
  fs.writeFileSync(file, PNG.sync.write(p));
}
// Forces alpha to 255 (PrintWindow leaves it undefined for some windows).
function opaque(img) {
  for (let i = 3; i < img.data.length; i += 4) img.data[i] = 255;
  return img;
}

// ── capture ──
const capture = (out, mode = 'client') =>
  new Promise((resolve, reject) =>
    execFile(CAPTURE, [String(process.pid), out, mode], (err, stdout, stderr) =>
      err ? reject(new Error(stderr || err.message)) : resolve(stdout.trim()),
    ),
  );

// `hold` is repeated until the capture is done: the real mouse cursor also sends events to
// full-screen scenes and would otherwise undo a simulated hover.
async function shot(comp, name, { mode = 'client', x = 0, y = 0, wait = 900, before, hold } = {}) {
  if (!NAMES.includes(name)) throw new Error(`add "${name}" to NAMES`);
  if (!wanted(name)) {
    console.log(`${lang} ${name} kept`);
    return loadPng(path.join(outDir, `${name}.png`));
  }
  retaken.add(name);
  comp.window.logicalPosition = { x, y };
  comp.show();
  await sleep(wait);
  if (before) {
    await before(comp);
    await sleep(400);
  }
  let timer;
  if (hold) {
    hold(comp);
    timer = setInterval(() => hold(comp), 25);
    await sleep(300);
  }
  const file = path.join(outDir, `${name}.png`);
  const size = await capture(file, mode).finally(() => clearInterval(timer));
  comp.hide();
  await sleep(150);
  const img = opaque(loadPng(file));
  savePng(img, file);
  console.log(`${lang} ${name} ${size}`);
  return img;
}

const ui = slint.loadFile(path.join(baked, 'ui', 'main.slint'), { style: 'fluent' });
const doc = slint.loadFile(path.join(baked, 'docscenes', 'scenes.slint'), { style: 'fluent' });
const model = (arr) => new slint.ArrayModel(arr);
const px = (v) => Math.round(v * SCALE);

// Demo codes on the mock event page.
const CODES = {
  link: 'https://events.example.org/register',
  wifi: 'WIFI:T:WPA;S:Seminar-Guest;P:autumn-2026;;',
  contact: L(
    'BEGIN:VCARD\nVERSION:3.0\nFN:Mei-Ling Chen\nORG:Seminar Office\nTEL:+886-2-2345-6789\nEMAIL:office@example.org\nEND:VCARD',
    'BEGIN:VCARD\nVERSION:3.0\nFN:陳美玲\nORG:講座籌備處\nTEL:+886-2-2345-6789\nEMAIL:office@example.org\nEND:VCARD',
  ),
  short: 'http://bit.ly/4kQ2xZp',
};
const qrs = {
  qr0: qrImage(CODES.link),
  qr1: qrImage(CODES.wifi),
  qr2: qrImage(CODES.contact),
  qr3: qrImage(CODES.short),
};
const ORGANIZER = L('Mei-Ling Chen', '陳美玲');
// QR positions on the mock desktop, logical pixels (see scenes.slint).
const qrBox = (i) => ({ x: 165 + 260 * i, y: 284, w: 170, h: 170 });
const pill = (i, kind, label, sublabel, level, selected = false, live = true) => {
  const b = qrBox(i);
  return { x: px(b.x), y: px(b.y), w: px(b.w), h: px(b.h), kind, label, sublabel, level, selected, live };
};
const PILLS = () => [
  pill(0, 'url', 'events.example.org', '', 0, true),
  pill(1, 'wifi', 'Seminar-Guest', T('Wi-Fi network'), 0),
  pill(2, 'contact', ORGANIZER, T('Contact'), 0),
  pill(3, 'url', 'bit.ly', T('Not encrypted (http)'), 2),
];
const HOTKEYS = { hotkey_screen: 'Ctrl+Alt+Q', hotkey_region: 'Ctrl+Alt+A', hotkey_camera: 'Ctrl+Alt+W' };
const LANGS = [T('Use system language'), 'English', '繁體中文', '简体中文', 'Español', 'Português (Brasil)', '日本語', '한국어', 'Français', 'Deutsch', 'Русский'];

const detail = (d) => ({
  kind: d.kind,
  title: d.title,
  content: d.content ?? '',
  level: d.level ?? 0,
  fields: model(d.fields.map(([key, value, copyable = true, secret = false]) => ({ key, value, copyable, secret }))),
  findings: model((d.findings ?? []).map(([code, det, level]) => ({ code, detail: det, level }))),
  actions: model(d.actions.map(([id, primary = false, danger = false]) => ({ id, primary, danger }))),
  confirm_danger: d.confirm ?? false,
  can_trust: d.canTrust ?? false,
  trust_domain: d.trust ?? '',
});

function overlay(extra = {}) {
  const o = new ui.OverlayWindow();
  o.window.logicalSize = { width: 1280, height: 720 };
  Object.assign(o, { screenshot: DESKTOP, img_w: 1920, img_h: 1080, animate: false, primary: true, ...HOTKEYS }, extra);
  return o;
}

let DESKTOP;

async function main() {
  // 1. The mock desktop, used as the "frozen screen" and as the camera / image source.
  const desk = new doc.SceneDesktop();
  Object.assign(desk, qrs);
  DESKTOP = await shot(desk, 'desktop');

  // 2. Whole-screen scan: four codes found, the first one selected.
  await shot(overlay({ pills: model(PILLS()), status: 2, found_count: 4 }), 'scan-screen');

  // 3. Select area: dragging a box around the Wi-Fi code.
  {
    const o = overlay({ region_mode: true, status: 0, pills: model([]) });
    const b = qrBox(1);
    const x0 = b.x - 26;
    const y0 = b.y - 26;
    const end = { x: x0 + b.w + 52, y: y0 + b.h + 52 };
    await shot(o, 'scan-region', {
      before: async (c) => {
        c.window.dispatchEvent({ type: 'pointer-moved', position: { x: x0, y: y0 } });
        c.window.dispatchEvent({ type: 'pointer-pressed', position: { x: x0, y: y0 }, button: 'left' });
        for (let k = 1; k <= 10; k++)
          c.window.dispatchEvent({ type: 'pointer-moved', position: { x: x0 + (k * (end.x - x0)) / 10, y: y0 + (k * (end.y - y0)) / 10 } });
      },
      // Keep the box where it is if the real mouse moves meanwhile.
      hold: (c) => c.window.dispatchEvent({ type: 'pointer-moved', position: end }),
    });
  }

  // 4. Detail cards on the overlay.
  await shot(
    overlay({
      pills: model(PILLS()),
      status: 2,
      found_count: 4,
      detail_visible: true,
      detail: detail({
        kind: 'wifi',
        title: 'Seminar-Guest',
        fields: [['ssid', 'Seminar-Guest'], ['security', 'WPA/WPA2', false], ['password', 'autumn-2026', true, true]],
        actions: [['connect-wifi', true], ['copy-password'], ['close']],
      }),
    }),
    'detail-wifi',
  );
  await shot(
    overlay({
      pills: model(PILLS()),
      status: 2,
      found_count: 4,
      detail_visible: true,
      detail: detail({
        kind: 'url',
        title: 'events.example.org',
        fields: [['url', CODES.link]],
        actions: [['open', true], ['copy'], ['close']],
        canTrust: true,
        trust: 'events.example.org',
      }),
    }),
    'detail-link',
  );
  await shot(
    overlay({
      pills: model([pill(3, 'url', 'login-check.example.net', T('Disguised address — it really goes to {}', 'login-check.example.net'), 3, true)]),
      status: 2,
      found_count: 1,
      detail_visible: true,
      detail: detail({
        kind: 'url',
        title: 'login-check.example.net',
        level: 3,
        fields: [['url', 'https://bank.example.com@login-check.example.net/']],
        findings: [['user-info', 'login-check.example.net', 3]],
        actions: [['open'], ['copy'], ['close']],
      }),
    }),
    'detail-danger',
  );
  await shot(
    overlay({
      pills: model(PILLS()),
      status: 2,
      found_count: 4,
      detail_visible: true,
      detail: detail({
        kind: 'url',
        title: 'bit.ly',
        level: 2,
        fields: [['url', CODES.short]],
        findings: [['not-encrypted', '', 2], ['shortener', 'bit.ly', 2]],
        actions: [['open', true], ['copy'], ['close']],
      }),
    }),
    'detail-warning',
  );

  // 5. Floating toolbar, column and bubble on the desktop (hover shows the hint).
  const floating = async (mode, name, hover) => {
    const s = new doc.SceneFloating();
    Object.assign(s, qrs, { mode });
    await shot(s, name, {
      hold: hover ? (c) => c.window.dispatchEvent({ type: 'pointer-moved', position: hover }) : undefined,
    });
  };
  await floating('row', 'toolbar-row', { x: 762, y: 49 });
  await floating('right', 'toolbar-right', { x: 1224, y: 452 });
  await floating('left', 'toolbar-left', { x: 56, y: 161 });
  await floating('bubble', 'bubble', null);

  // 6. Tray menu.
  {
    const s = new doc.SceneTray();
    Object.assign(s, qrs);
    await shot(s, 'tray-menu');
  }

  // 6b. Windows right-click menus: desktop background and one picture.
  for (const mode of ['desktop', 'image']) {
    const s = new doc.SceneShellMenu();
    s.mode = mode;
    await shot(s, `shell-${mode}`);
  }

  // 7. Camera: a phone with the Wi-Fi code held up to the webcam.
  const photo = new doc.ScenePhoto();
  photo.qr = qrImage(CODES.wifi, 8);
  const PHOTO = await shot(photo, 'camera-source');
  {
    const p = new ui.PreviewWindow();
    p.window.logicalSize = { width: 760, height: 600 };
    Object.assign(p, {
      camera_mode: true,
      frame: PHOTO,
      frame_w: PHOTO.width,
      frame_h: PHOTO.height,
      cameras: model(['Integrated Webcam']),
      status: 2,
      pills: model([{ x: px(520), y: px(194), w: px(240), h: px(240), kind: 'wifi', label: 'Seminar-Guest', sublabel: T('Wi-Fi network'), level: 0, selected: true, live: true }]),
    });
    await shot(p, 'camera', { mode: 'frame', x: 60, y: 30 });
  }

  // 8. Image file: a photo of the contact card.
  {
    const b = qrBox(2);
    const img = crop(DESKTOP, px(b.x - 50), px(b.y - 60), px(b.w + 100), px(b.h + 160));
    const p = new ui.PreviewWindow();
    p.window.logicalSize = { width: 760, height: 560 };
    Object.assign(p, {
      camera_mode: false,
      frame: img,
      frame_w: img.width,
      frame_h: img.height,
      status: 2,
      pills: model([{ x: px(50), y: px(60), w: px(b.w), h: px(b.h), kind: 'contact', label: ORGANIZER, sublabel: T('Contact'), level: 0, selected: false, live: true }]),
    });
    await shot(p, 'image-preview', { mode: 'frame', x: 60, y: 30 });
  }

  // 9. Create QR code.
  {
    const g = new ui.GeneratorWindow();
    const q = qrImage(CODES.link, 8, 2);
    Object.assign(g, { text: CODES.link, ec_level: 1, preview: q, state: 1, modules: q.modules });
    await shot(g, 'generator', { mode: 'frame', x: 60, y: 30 });
  }

  // 10. Welcome.
  {
    const w = new ui.WelcomeWindow();
    Object.assign(w, { languages: model(LANGS), language_index: 0, autostart: true, hotkeys_supported: true, camera_available: true, ...HOTKEYS });
    await shot(w, 'welcome', { mode: 'frame', x: 60, y: 10 });
  }

  // 11. Settings pages.
  const settings = async (page, name, extra = {}) => {
    const s = new ui.SettingsWindow();
    s.window.logicalSize = { width: 920, height: 640 };
    Object.assign(
      s,
      {
        languages: model(LANGS),
        language_index: 0,
        theme_index: 0,
        autostart: true,
        shell_menu: true,
        shell_menu_supported: true,
        bubble_at_startup: false,
        hide_in_fullscreen: true,
        toolbar_layout_index: 0,
        check_updates: true,
        fullscreen_detection_supported: true,
        auto_open_trusted: true,
        trusted_domains: model(['events.example.org', 'github.com']),
        auto_copy: false,
        sound: false,
        all_formats: true,
        camera_available: true,
        cameras: model(['Integrated Webcam']),
        camera_index: 0,
        mirror: true,
        close_after_scan: true,
        hold_seconds: 8,
        hotkeys_supported: true,
        hotkeys: model([
          { id: 'screen', value: 'Ctrl+Alt+Q', state: 0 },
          { id: 'region', value: 'Ctrl+Alt+A', state: 0 },
          { id: 'camera', value: 'Ctrl+Alt+W', state: 0 },
          { id: 'toolbar', value: 'Ctrl+Alt+E', state: 0 },
        ]),
        history_enabled: true,
        history_max: 50,
        history: model([
          { time: '2026-11-14 10:28', kind: 'url', text: CODES.link, openable: true },
          { time: '2026-11-14 10:27', kind: 'contact', text: ORGANIZER, openable: false },
          { time: '2026-11-13 16:05', kind: 'einvoice', text: 'AB-12345678', openable: false },
          { time: '2026-11-12 09:41', kind: 'text', text: L('Room 302 — bring your student ID', '302 教室，請攜帶學生證'), openable: false },
        ]),
        version: '1.0.0',
        update_status: T('QRcode tool {} is the latest version.', '1.0.0'),
        trademark_notice: 'QR Code is a registered trademark of DENSO WAVE INCORPORATED.',
        page,
      },
      extra,
    );
    await shot(s, name, { mode: 'frame', x: 40, y: 10 });
  };
  await settings(0, 'settings-general', { toolbar_layout_index: 1 });
  await settings(1, 'settings-scanning');
  await settings(2, 'settings-camera');
  await settings(3, 'settings-shortcuts');
  await settings(4, 'settings-privacy');
  await settings(5, 'settings-about');

  // 12. Update prompt.
  {
    const m = new ui.MessageWindow();
    Object.assign(m, { kind: 'update-available', arg: '1.0.1', level: 0 });
    await shot(m, 'message-update', { mode: 'frame', x: 200, y: 200 });
  }
  console.log(`${lang}: ${retaken.size} screenshots taken, ${NAMES.length - retaken.size} kept`);
}

setTimeout(() => {
  console.error('watchdog: took too long');
  process.exit(3);
}, 10 * 60 * 1000);
slint.runEventLoop({
  runningCallback: () =>
    main()
      .then(() => process.exit(0))
      .catch((e) => {
        console.error(e);
        process.exit(1);
      }),
  quitOnLastWindowClosed: false,
});
