// Copies app/ui (plus extra .slint dirs) to out/<lang>/ with every @tr("…") literal replaced by its
// translation from app/translations/<lang>/LC_MESSAGES/qrcode-tool.po. The Slint interpreter in
// Node then shows the real windows in that language without gettext.
//
// Usage: node bake.js <repo> <outRoot> <lang> [extraDir...]
const fs = require('fs');
const path = require('path');

const [repo, outRoot, lang, ...extra] = process.argv.slice(2);

function unescapePo(s) {
  return s.replace(/\\(.)/g, (m, c) => ({ n: '\n', t: '\t', '"': '"', '\\': '\\' }[c] ?? c));
}

function parsePo(file) {
  const text = fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
  const byCtx = new Map();
  const byId = new Map();
  const re = /(?:^msgctxt "((?:[^"\\]|\\.)*)"\n)?^msgid ((?:".*"\n)+)^msgstr ((?:".*"\n?)+)/gm;
  const join = (block) => [...block.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => unescapePo(m[1])).join('');
  let m;
  while ((m = re.exec(text))) {
    const id = join(m[2]);
    const str = join(m[3]);
    if (!id || !str) continue;
    byCtx.set(`${m[1] ? unescapePo(m[1]) : ''}\u0004${id}`, str);
    if (!byId.has(id)) byId.set(id, str);
  }
  return { byCtx, byId };
}

function unescapeSlint(s) {
  return s.replace(/\\(.)/g, (m, c) => ({ n: '\n', '"': '"', '\\': '\\' }[c] ?? m));
}
function escapeSlint(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n');
}

function bakeSource(src, po, stats) {
  const decls = [...src.matchAll(/^(?:export\s+)?(?:component|global)\s+([A-Za-z0-9_-]+)/gm)].map((m) => [m.index, m[1]]);
  return src.replace(/@tr\(\s*"((?:[^"\\]|\\.)*)"/g, (whole, lit, offset) => {
    let ctx = '';
    for (const [pos, name] of decls) if (pos < offset) ctx = name;
    const id = unescapeSlint(lit);
    const str = po.byCtx.get(`${ctx}\u0004${id}`) ?? po.byId.get(id);
    if (str === undefined) {
      stats.missing.add(id);
      return whole;
    }
    stats.done++;
    return `@tr("${escapeSlint(str)}"`;
  });
}

function copyDir(src, dst, po, stats) {
  fs.mkdirSync(dst, { recursive: true });
  for (const e of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, e.name);
    const d = path.join(dst, e.name);
    if (e.isDirectory()) copyDir(s, d, po, stats);
    else if (e.name.endsWith('.slint') && po) fs.writeFileSync(d, bakeSource(fs.readFileSync(s, 'utf8'), po, stats));
    else fs.copyFileSync(s, d);
  }
}

const po = lang === 'en' ? null : parsePo(path.join(repo, 'app', 'translations', lang, 'LC_MESSAGES', 'qrcode-tool.po'));
const extraPo = lang === 'en' ? null : extra.length ? (() => {
  // Optional doc-only translations: <extraDir>/strings.<lang>.json ({ "English": "translation" }).
  const merged = { byCtx: new Map(po.byCtx), byId: new Map(po.byId) };
  for (const dir of extra) {
    const f = path.join(dir, `strings.${lang}.json`);
    if (fs.existsSync(f)) for (const [k, v] of Object.entries(JSON.parse(fs.readFileSync(f, 'utf8')))) merged.byId.set(k, v);
  }
  return merged;
})() : null;

const out = path.join(outRoot, lang);
fs.rmSync(out, { recursive: true, force: true });
const stats = { done: 0, missing: new Set() };
copyDir(path.join(repo, 'app', 'ui'), path.join(out, 'ui'), po, stats);
for (const dir of extra) copyDir(dir, path.join(out, path.basename(dir)), extraPo, stats);

// Embeddable copies of the toolbar and bubble windows, so a scene can draw them on a
// mock desktop (a Window cannot be a child element).
{
  const f = path.join(out, 'ui', 'floating.slint');
  let src = fs.readFileSync(f, 'utf8');
  src = src
    .replace('export component ToolbarWindow inherits Window {', 'export component ToolbarPanel inherits Rectangle {')
    .replace('export component BubbleWindow inherits Window {', 'export component BubblePanel inherits Rectangle {')
    .replace(/^\s*(no-frame|always-on-top): true;\r?\n/gm, '')
    .replace(/^\s*title: "QRcode tool";\r?\n/gm, '')
    .replace(/^\s*icon: Icons\.logo;\r?\n/gm, '');
  if (!src.includes('ToolbarPanel') || !src.includes('BubblePanel')) throw new Error('floating.slint changed; update the embed patch');
  fs.writeFileSync(path.join(out, 'ui', 'floating_embed.slint'), src);
}
console.log(`${lang}: ${stats.done} strings baked, ${stats.missing.size} missing`);
for (const id of stats.missing) console.log('  missing:', JSON.stringify(id));
