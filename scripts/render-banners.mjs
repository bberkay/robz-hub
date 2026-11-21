// README başlık görselini ve sosyal medya önizlemesini (Open Graph) üretir:  node scripts/render-banners.mjs
//   docs/banner-tr.png, docs/banner-en.png   1280×640 (2x çizilir: 2560×1280), README'nin en üstünde
//   docs/og-tr.png, docs/og-en.png           1200×630, bağlantı paylaşılınca görünen önizleme
// Microsoft Edge'in başsız kipiyle çizer (Windows'ta zaten kurulu); ekran görüntüleri docs/screenshots'tan alınır.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const EDGE = ['C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Microsoft/Edge/Application/msedge.exe'].find(existsSync);
if (!EDGE) throw new Error('Microsoft Edge bulunamadı');

const file = (p) => pathToFileURL(join(root, p)).href;
const font = (subset) => file(`node_modules/@fontsource-variable/mona-sans/files/mona-sans-${subset}-standard-normal.woff2`);

const TEXT = {
  tr: {
    title: 'Bütün Robz uygulamaları tek yerde.',
    sub: 'Kur, güncelle, başlat. Rust ile yazıldı, açık kaynak.',
    del: 'iki site, iki kurulum, elle güncelleme',
    add: 'tek uygulama, tek tık',
    shot: 'docs/screenshots/01-ana-sayfa.png',
  },
  en: {
    title: 'Every Robz app in one place.',
    sub: 'Install, update, launch. Built with Rust, open source.',
    del: 'two sites, two installers, manual updates',
    add: 'one app, one click',
    shot: 'docs/screenshots/06-home-en.png',
  },
};

function html(t, { width, height }) {
  const scale = width / 1280;
  return `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face { font-family: 'Mona Sans'; font-weight: 200 900; font-stretch: 75% 125%; src: url('${font('latin')}') format('woff2'); unicode-range: U+0000-00FF, U+0131, U+2000-206F; }
@font-face { font-family: 'Mona Sans'; font-weight: 200 900; font-stretch: 75% 125%; src: url('${font('latin-ext')}') format('woff2'); unicode-range: U+0100-02BA; }
* { box-sizing: border-box; margin: 0; }
html, body { width: ${width}px; height: ${height}px; overflow: hidden; background: #0d1117; }
.stage { position: relative; width: 1280px; height: ${Math.round(height / scale)}px; transform: scale(${scale}); transform-origin: 0 0; overflow: hidden;
  font-family: 'Mona Sans', sans-serif; color: #f0f6fc; -webkit-font-smoothing: antialiased; }
.text { position: absolute; left: 72px; top: 72px; width: 580px; }
.brand { display: flex; align-items: center; gap: 14px; font-size: 26px; font-weight: 700; font-stretch: 112%; }
.brand img { width: 48px; height: 48px; border-radius: 11px; }
h1 { margin-top: 48px; font-size: 62px; line-height: 1.02; font-weight: 820; font-stretch: 125%; letter-spacing: -0.025em; text-wrap: balance; }
p { margin-top: 22px; color: #9198a1; font-size: 21px; line-height: 1.45; }
.diff { margin-top: 34px; overflow: hidden; border: 1px solid #3d444d; border-radius: 10px;
  font-family: 'Cascadia Code', 'Cascadia Mono', Consolas, monospace; font-size: 18px; }
.diff div { padding: 9px 16px; }
.d { background: rgba(248, 81, 73, 0.12); color: #ffdcd7; }
.d span { text-decoration: line-through; text-decoration-color: rgba(248, 81, 73, 0.7); }
.a { background: rgba(46, 160, 67, 0.18); color: #aff5b4; }
.d b { color: #f85149; } .a b { color: #3fb950; }
.repo { position: absolute; left: 72px; bottom: 44px; color: #6e7681; font-family: 'Cascadia Code', Consolas, monospace; font-size: 16px; }
.window { position: absolute; left: 690px; top: 92px; width: 820px; overflow: hidden; border: 1px solid #3d444d; border-radius: 12px;
  background: #010409; box-shadow: 0 30px 80px rgba(1, 4, 9, 0.9); }
.bar { display: flex; align-items: center; gap: 9px; height: 34px; padding-left: 14px; border-bottom: 1px solid rgba(61,68,77,.7);
  background: #161b22; color: #9198a1; font-size: 13px; }
.bar img { width: 15px; height: 15px; border-radius: 3px; }
.window > img { display: block; width: 100%; }
.line { position: absolute; left: 0; right: 0; bottom: 0; height: 6px; background: #fb8f44; }
</style></head><body><div class="stage">
  <div class="text">
    <div class="brand"><img src="${file('ui/assets/hub/icon.png')}" alt="">Robz Hub</div>
    <h1>${t.title}</h1>
    <p>${t.sub}</p>
    <div class="diff"><div class="d"><b>- </b><span>${t.del}</span></div><div class="a"><b>+ </b>${t.add}</div></div>
  </div>
  <div class="repo">github.com/kutsalkiremitci/robz-hub</div>
  <div class="window"><div class="bar"><img src="${file('ui/assets/hub/icon.png')}" alt="">Robz Hub</div><img src="${file(t.shot)}" alt=""></div>
  <div class="line"></div>
</div></body></html>`;
}

function render(name, t, size, deviceScale) {
  const work = mkdtempSync(join(tmpdir(), 'robzhub-banner-'));
  const page = join(work, 'banner.html');
  writeFileSync(page, html(t, size));
  const output = join(root, 'docs', name);
  execFileSync(EDGE, [
    '--headless=new',
    '--disable-gpu',
    '--hide-scrollbars',
    '--no-first-run',
    `--user-data-dir=${join(work, 'profile')}`,
    `--force-device-scale-factor=${deviceScale}`,
    `--window-size=${size.width},${size.height}`,
    '--virtual-time-budget=3000',
    `--screenshot=${output}`,
    pathToFileURL(page).href,
  ], { stdio: 'ignore' });
  rmSync(work, { recursive: true, force: true });
  console.log(`docs/${name}`);
}

for (const lang of ['tr', 'en']) {
  render(`banner-${lang}.png`, TEXT[lang], { width: 1280, height: 640 }, 2);
  render(`og-${lang}.png`, TEXT[lang], { width: 1200, height: 630 }, 1);
}
