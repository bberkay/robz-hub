// Web sitesini üretir:  node site/build.mjs   ->  site/dist/ (Türkçe), site/dist/en/ (İngilizce)
//
// Uygulama kartları ../catalog.json'dan, ekran görüntüleri ../docs/screenshots'tan, simgeler ../ui/assets'ten gelir;
// kataloğa eklenen her uygulama sitede de kendiliğinden görünür. Sürüm numaraları ve yıldız sayısı derleme sırasında
// GitHub'dan okunur (ağ yoksa boş kalır) ve sayfada site.js ile güncellenir. GitHub Pages iş akışı bunu çalıştırır.
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import content from './content.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const out = join(here, 'dist');

const REPO = 'kutsalkiremitci/robz-hub';
const SITE = 'https://kutsalkiremitci.github.io/robz-hub/';
const REPO_URL = `https://github.com/${REPO}`;
const DOWNLOAD = `${REPO_URL}/releases/latest/download/RobzHub-Setup.exe`;

const catalog = JSON.parse(readFileSync(join(root, 'catalog.json'), 'utf8'));
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));

const esc = (value) =>
  String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
const pick = (text, lang) => (typeof text === 'string' ? text : text?.[lang] ?? text?.tr ?? '');

// ---- simgeler (16'lık ızgara) ---------------------------------------------------------------------------------------

const ICONS = {
  download: '<path d="M8 2.25v8M4.75 7 8 10.25 11.25 7M2.75 13.25h10.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>',
  star: '<path d="M8 1.6l1.95 3.95 4.35.63-3.15 3.07.74 4.33L8 11.53l-3.89 2.05.74-4.33L1.7 6.18l4.35-.63z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>',
  github:
    '<path fill="currentColor" d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"/>',
  discord: '<path d="M2.5 3.25h11v7.5H8.25L5 13.25v-2.5H2.5z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>',
  kick: '<path d="M8 6.75a1.25 1.25 0 1 0 0 2.5 1.25 1.25 0 0 0 0-2.5zM4.75 4.75a4.6 4.6 0 0 0 0 6.5M11.25 4.75a4.6 4.6 0 0 1 0 6.5M2.6 2.9a7.3 7.3 0 0 0 0 10.2M13.4 2.9a7.3 7.3 0 0 1 0 10.2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>',
  external: '<path d="M9.5 2.5h4v4M13.5 2.5 7.75 8.25M11.5 9.5v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>',
  tag: '<path d="M2.5 2.5h5.1l5.9 5.9-5.1 5.1-5.9-5.9zM5.4 5.4h.01" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>',
  info: '<path d="M8 2.25a5.75 5.75 0 1 0 0 11.5 5.75 5.75 0 0 0 0-11.5zM8 7.4v3.6M8 5.1h.01" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>',
  plus: '<path d="M8 3v10M3 8h10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>',
};
// Bayraklar SVG: Windows bayrak emojilerini "TR", "GB" harfleri olarak gösterir.
const STAR = Array.from({ length: 10 }, (_, k) => {
  const angle = Math.PI + (k * Math.PI) / 5;
  const radius = k % 2 ? 0.96 : 2.5;
  return `${(16.1 + radius * Math.cos(angle)).toFixed(2)},${(10 + radius * Math.sin(angle)).toFixed(2)}`;
}).join(' ');
const FLAGS = {
  tr: `<svg class="flag" width="21" height="14" viewBox="0 0 30 20" aria-hidden="true"><rect width="30" height="20" fill="#E30A17"/><circle cx="10.5" cy="10" r="5" fill="#fff"/><circle cx="11.75" cy="10" r="4" fill="#E30A17"/><polygon points="${STAR}" fill="#fff"/></svg>`,
  en: '<svg class="flag" width="21" height="14" viewBox="0 0 45 30" aria-hidden="true"><rect width="45" height="30" fill="#012169"/><path d="M0,0 L45,30 M45,0 L0,30" stroke="#fff" stroke-width="6"/><path d="M0,0 L45,30 M45,0 L0,30" stroke="#C8102E" stroke-width="2.2"/><path d="M22.5,0 V30 M0,15 H45" stroke="#fff" stroke-width="9"/><path d="M22.5,0 V30 M0,15 H45" stroke="#C8102E" stroke-width="5.4"/></svg>',
};
const LANGS = { tr: 'Türkçe', en: 'English' };

const icon = (name, size = 16) =>
  `<svg class="i" width="${size}" height="${size}" viewBox="0 0 16 16" aria-hidden="true">${ICONS[name]}</svg>`;

// ---- GitHub'dan derleme anındaki bilgiler (ağ yoksa boş) -------------------------------------------------------------

async function github(path) {
  try {
    const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'robz-hub-site' };
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    const res = await fetch(`https://api.github.com/${path}`, { headers, signal: AbortSignal.timeout(8000) });
    return res.ok ? await res.json() : null;
  } catch {
    return null;
  }
}

const live = { stars: null, hubVersion: pkg.version, hubSize: null, apps: {} };
{
  const [repo, release, ...apps] = await Promise.all([
    github(`repos/${REPO}`),
    github(`repos/${REPO}/releases/latest`),
    ...catalog.apps.map((app) => github(`repos/${app.repo}/releases/latest`)),
  ]);
  if (repo) live.stars = repo.stargazers_count;
  if (release) {
    live.hubVersion = release.tag_name.replace(/^v/, '');
    live.hubSize = release.assets.find((a) => a.name === 'RobzHub-Setup.exe')?.size ?? null;
  }
  catalog.apps.forEach((app, i) => {
    if (apps[i]) live.apps[app.id] = apps[i].tag_name.replace(/^v/, '');
  });
}

const sizeText = (bytes, lang) => {
  const mb = (bytes ?? 2 * 1024 * 1024) / (1024 * 1024);
  return `${new Intl.NumberFormat(lang === 'tr' ? 'tr-TR' : 'en-US', { maximumFractionDigits: 1 }).format(mb)} MB`;
};

// ---- dosyalar -------------------------------------------------------------------------------------------------------

rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, 'assets', 'img'), { recursive: true });
mkdirSync(join(out, 'en'), { recursive: true });
cpSync(join(here, 'assets'), join(out, 'assets'), { recursive: true });
cpSync(join(root, 'docs', 'screenshots'), join(out, 'assets', 'img'), { recursive: true });
cpSync(join(root, 'assets', 'hub', 'logo.svg'), join(out, 'assets', 'logo.svg'));
cpSync(join(root, 'src-tauri', 'icons', '32x32.png'), join(out, 'assets', 'favicon-32.png'));
cpSync(join(root, 'src-tauri', 'icons', '128x128@2x.png'), join(out, 'assets', 'apple-touch-icon.png'));
for (const name of ['banner-tr.png', 'banner-en.png', 'og-tr.png', 'og-en.png']) {
  if (existsSync(join(root, 'docs', name))) cpSync(join(root, 'docs', name), join(out, 'assets', 'img', name));
}
const fontDir = join(root, 'node_modules', '@fontsource-variable', 'mona-sans', 'files');
mkdirSync(join(out, 'assets', 'fonts'), { recursive: true });
for (const subset of ['latin', 'latin-ext']) {
  cpSync(join(fontDir, `mona-sans-${subset}-standard-normal.woff2`), join(out, 'assets', 'fonts', `mona-sans-${subset}.woff2`));
}
const appIcons = {};
for (const app of catalog.apps) {
  if (!app.icon?.startsWith('assets/')) continue;
  const file = `${app.id}${extname(app.icon)}`;
  cpSync(join(root, 'ui', app.icon), join(out, 'assets', 'apps', file));
  appIcons[app.id] = `assets/apps/${file}`;
}

// ---- sayfa ----------------------------------------------------------------------------------------------------------

function windowFrame(src, alt, base, { eager = false, cls = '' } = {}) {
  return `<figure class="window ${cls}">
      <div class="window-bar" aria-hidden="true">
        <img src="${base}assets/logo.svg" width="14" height="14" alt="">
        <span>Robz Hub</span>
        <span class="window-controls"><i></i><i></i><i></i></span>
      </div>
      <img src="${base}assets/img/${src}" width="1240" height="800" alt="${esc(alt)}" ${eager ? 'fetchpriority="high"' : 'loading="lazy"'}>
    </figure>`;
}

function diff(t) {
  const n = t.diff.del.length;
  const rows = [
    ...t.diff.del.map((text, i) => ({ kind: 'del', old: i + 1, now: '', text })),
    ...t.diff.add.map((text, i) => ({ kind: 'add', old: '', now: i + 1, text })),
  ];
  return `<figure class="diff" aria-label="${esc(t.diffLabel)}">
      <figcaption class="diff-head">
        <span class="diff-file">${esc(t.diffFile)}</span>
        <span class="diff-stat"><span class="diff-stat-add">+${t.diff.add.length}</span> <span class="diff-stat-del">−${n}</span>
          <span class="diff-blocks" aria-hidden="true">${'<i class="a"></i>'.repeat(t.diff.add.length)}${'<i class="d"></i>'.repeat(n)}</span></span>
      </figcaption>
      <div class="diff-hunk" aria-hidden="true">@@ -1,${n} +1,${t.diff.add.length} @@</div>
      <ol class="diff-lines">
        ${rows
          .map(
            (row, i) => `<li class="diff-${row.kind}" style="--i:${i}">
          <span class="diff-num" aria-hidden="true">${row.old}</span><span class="diff-num" aria-hidden="true">${row.now}</span>
          <span class="diff-sign" aria-hidden="true">${row.kind === 'del' ? '-' : '+'}</span>
          <span class="diff-text">${row.kind === 'del' ? '<span class="sr-only">− </span>' : '<span class="sr-only">+ </span>'}${esc(row.text)}</span>
        </li>`,
          )
          .join('')}
      </ol>
    </figure>`;
}

function appCard(app, t, base) {
  const lang = t.lang;
  const version = live.apps[app.id];
  const highlights = (app.highlights || []).slice(0, 3);
  return `<article class="app" id="app-${app.id}">
      <header class="app-head">
        ${appIcons[app.id] ? `<img class="app-icon" src="${base}${appIcons[app.id]}" width="64" height="64" alt="">` : ''}
        <div>
          <h3>${esc(app.name)}</h3>
          <p class="app-meta">
            ${app.role ? `<span class="label">${esc(pick(app.role, lang))}</span>` : ''}
            <span class="vtag" data-app-repo="${esc(app.repo)}" ${version ? '' : 'hidden'}>${icon('tag', 12)}<span class="sr-only">${esc(t.latest)} </span><span data-version>${esc(version || '')}</span></span>
          </p>
        </div>
      </header>
      <p class="app-tagline">${esc(pick(app.tagline, lang))}</p>
      <p class="app-desc">${esc(pick(app.description, lang))}</p>
      ${
        highlights.length
          ? `<dl class="app-highlights">${highlights
              .map((h) => `<div><dt>${esc(pick(h.title, lang))}</dt><dd>${esc(pick(h.text, lang))}</dd></div>`)
              .join('')}</dl>`
          : ''
      }
      <p class="app-links">
        ${app.links?.website ? `<a href="${esc(app.links.website)}">${icon('external', 14)}${esc(t.appWebsite)}</a>` : ''}
        <a href="https://github.com/${esc(app.repo)}">${icon('github', 14)}${esc(t.appRepo)}</a>
      </p>
    </article>`;
}

function bundleBox(bundle, t, base) {
  const lang = t.lang;
  const members = bundle.apps.map((id) => catalog.apps.find((a) => a.id === id)).filter(Boolean);
  return `<aside class="bundle">
      <div class="bundle-icons" aria-hidden="true">
        ${members.map((m) => `<img src="${base}${appIcons[m.id]}" width="56" height="56" alt="">`).join(`<span class="bundle-plus">${icon('plus', 18)}</span>`)}
      </div>
      <div>
        <h3>${esc(pick(bundle.title, lang))}</h3>
        <p>${esc(pick(bundle.description, lang))}</p>
        <p class="bundle-note">${esc(t.bundleNote)}</p>
      </div>
    </aside>`;
}

function downloadButtons(t, { big = false } = {}) {
  return `<div class="cta">
      <a class="btn btn-primary ${big ? 'btn-xl' : 'btn-lg'}" href="${DOWNLOAD}" data-download>
        ${icon('download', 18)}<span>${esc(t.download)}</span><span class="btn-meta" data-hub-size>${sizeText(live.hubSize, t.lang)}</span>
      </a>
      <a class="btn btn-star ${big ? 'btn-xl' : 'btn-lg'}" href="${REPO_URL}" title="${esc(t.starTitle)}">
        ${icon('star', 18)}<span>${esc(t.star)}</span><span class="btn-count" data-stars ${live.stars == null ? 'hidden' : ''}>${live.stars ?? ''}</span>
      </a>
    </div>`;
}

function page(t) {
  const lang = t.lang;
  const base = lang === 'tr' ? '' : '../';
  const url = lang === 'tr' ? SITE : `${SITE}en/`;
  const other = lang === 'tr' ? `${base}en/` : base || './';

  return `<!doctype html>
<html lang="${lang}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${esc(t.title)}</title>
  <meta name="description" content="${esc(t.description)}">
  <meta name="theme-color" content="#0d1117">
  <meta name="color-scheme" content="dark">
  <link rel="canonical" href="${url}">
  <link rel="alternate" hreflang="tr" href="${SITE}">
  <link rel="alternate" hreflang="en" href="${SITE}en/">
  <link rel="alternate" hreflang="x-default" href="${SITE}">
  <link rel="icon" href="${base}assets/logo.svg" type="image/svg+xml">
  <link rel="icon" href="${base}assets/favicon-32.png" sizes="32x32" type="image/png">
  <link rel="apple-touch-icon" href="${base}assets/apple-touch-icon.png">
  <link rel="preload" href="${base}assets/fonts/mona-sans-latin.woff2" as="font" type="font/woff2" crossorigin>
  <link rel="stylesheet" href="${base}assets/site.css">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Robz Hub">
  <meta property="og:title" content="${esc(t.title)}">
  <meta property="og:description" content="${esc(t.description)}">
  <meta property="og:url" content="${url}">
  <meta property="og:image" content="${SITE}assets/img/og-${lang}.png">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="630">
  <meta property="og:locale" content="${t.locale}">
  <meta name="twitter:card" content="summary_large_image">
  <script type="application/ld+json">${JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'Robz Hub',
    operatingSystem: 'Windows 10, Windows 11',
    applicationCategory: 'UtilitiesApplication',
    description: t.description,
    downloadUrl: DOWNLOAD,
    softwareVersion: live.hubVersion,
    license: 'https://opensource.org/licenses/MIT',
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
  })}</script>
</head>
<body>
  <a class="skip" href="#main">${esc(t.skip)}</a>
  <header class="top">
    <div class="wrap top-inner">
      <a class="brand" href="${base || './'}"><img src="${base}assets/logo.svg" width="28" height="28" alt=""><span>Robz Hub</span></a>
      <nav class="top-nav" aria-label="${esc(t.title)}">
        <a href="#apps">${esc(t.nav.apps)}</a>
        <a href="#usage">${esc(t.nav.usage)}</a>
        <a href="#faq">${esc(t.nav.faq)}</a>
        <a href="${REPO_URL}">${icon('github')}<span>${esc(t.nav.github)}</span></a>
      </nav>
      <details class="lang" data-lang-menu>
        <summary aria-label="${lang === 'tr' ? 'Dil' : 'Language'}: ${LANGS[lang]}">${FLAGS[lang]}<span>${LANGS[lang]}</span><svg class="i lang-chevron" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 6 8 10.5 12.5 6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg></summary>
        <ul>
          ${['tr', 'en']
            .map((code) => {
              const href = code === lang ? (lang === 'tr' ? './' : './') : code === 'en' ? 'en/' : '../';
              const current = code === lang ? ' aria-current="page"' : '';
              return `<li><a href="${href}" hreflang="${code}" lang="${code}"${current}>${FLAGS[code]}<span>${LANGS[code]}</span></a></li>`;
            })
            .join('')}
        </ul>
      </details>
    </div>
  </header>

  <main id="main">
    <section class="hero">
      <div class="wrap hero-grid">
        <div class="hero-text">
          <a class="pill" href="${REPO_URL}/releases/latest" data-release-link>
            <span class="pill-tag">${icon('tag', 12)}<span data-hub-version>v${esc(live.hubVersion)}</span></span>
            <span>${esc(t.releasePill)}</span>
          </a>
          <h1>${esc(t.heroTitle)}</h1>
          <p class="lead">${esc(t.heroLead)}</p>
          ${downloadButtons(t, { big: true })}
          <ul class="facts">${t.facts.map((f) => `<li>${esc(f)}</li>`).join('')}</ul>
        </div>
        ${windowFrame(lang === 'tr' ? '01-ana-sayfa.png' : '06-home-en.png', t.shotAlt, base, { eager: true, cls: 'hero-shot' })}
      </div>
    </section>

    <section class="why" aria-labelledby="why-title">
      <div class="wrap why-grid">
        <div>
          <h2 id="why-title">${esc(t.whyTitle)}</h2>
          <p class="section-lead">${esc(t.whyLead)}</p>
        </div>
        ${diff(t)}
      </div>
    </section>

    <section class="section" id="apps" aria-labelledby="apps-title">
      <div class="wrap">
        <h2 id="apps-title">${esc(t.appsTitle)}</h2>
        <p class="section-lead">${esc(t.appsLead)}</p>
        <div class="apps">${catalog.apps.map((app) => appCard(app, t, base)).join('')}</div>
        ${(catalog.bundles || []).map((b) => bundleBox(b, t, base)).join('')}
      </div>
    </section>

    <section class="section" id="usage" aria-labelledby="usage-title">
      <div class="wrap">
        <h2 id="usage-title">${esc(t.usageTitle)}</h2>
        <p class="section-lead">${esc(t.usageLead)}</p>
        <ol class="steps">
          ${t.steps
            .map(
              (step, i) => `<li class="step ${step.shot ? 'step-shot' : ''}">
            <div class="step-text">
              <span class="step-num" aria-hidden="true">${i + 1}</span>
              <h3>${esc(step.title)}</h3>
              <p>${esc(step.text)}</p>
              ${step.note ? `<p class="note">${icon('info')}<span>${esc(step.note)}</span></p>` : ''}
              ${i === 0 ? downloadButtons(t) : ''}
            </div>
            ${step.shot ? windowFrame(step.shot, step.shotAlt, base) : ''}
          </li>`,
            )
            .join('')}
        </ol>
      </div>
    </section>

    <section class="section" aria-labelledby="details-title">
      <div class="wrap">
        <h2 id="details-title">${esc(t.detailsTitle)}</h2>
        <dl class="details">
          ${t.details.map(([title, text]) => `<div><dt>${esc(title)}</dt><dd>${esc(text)}</dd></div>`).join('')}
        </dl>
      </div>
    </section>

    <section class="section" aria-labelledby="screens-title">
      <div class="wrap">
        <h2 id="screens-title">${esc(t.screensTitle)}</h2>
        <div class="screens">
          ${t.screens
            .map(
              ([file, caption]) => `<figure>
            <a href="${base}assets/img/${file}" title="${esc(t.openImage)}"><img src="${base}assets/img/${file}" width="1240" height="800" alt="${esc(caption)}" loading="lazy"></a>
            <figcaption>${esc(caption)}</figcaption>
          </figure>`,
            )
            .join('')}
        </div>
      </div>
    </section>

    <section class="section" id="faq" aria-labelledby="faq-title">
      <div class="wrap faq-wrap">
        <h2 id="faq-title">${esc(t.faqTitle)}</h2>
        <div class="faq">
          ${t.faq.map(([q, a]) => `<details><summary>${esc(q)}</summary><p>${esc(a)}</p></details>`).join('')}
        </div>
      </div>
    </section>

    <section class="final" aria-labelledby="final-title">
      <div class="wrap">
        <h2 id="final-title">${esc(t.finalTitle)}</h2>
        ${downloadButtons(t, { big: true })}
        <p class="final-text">${esc(t.finalText)}</p>
        <p class="final-links">
          <a href="${catalog.links.discord}">${icon('discord')}${esc(t.discord)}</a>
          <a href="${catalog.links.kick}">${icon('kick')}${esc(t.kick)}</a>
          <a href="${REPO_URL}/issues">${icon('github')}${esc(t.issues)}</a>
        </p>
      </div>
    </section>
  </main>

  <footer class="foot">
    <div class="wrap foot-inner">
      <a class="brand" href="${base || './'}"><img src="${base}assets/logo.svg" width="24" height="24" alt=""><span>Robz Hub</span></a>
      <p>${esc(t.footerLicense)} <a href="${REPO_URL}">${REPO}</a></p>
      <p class="foot-small">${esc(t.footerDisclaimer)}</p>
      <p><a href="${other}" hreflang="${lang === 'tr' ? 'en' : 'tr'}">${esc(t.otherLang)}</a></p>
    </div>
  </footer>
  <script src="${base}assets/site.js" defer></script>
</body>
</html>
`;
}

writeFileSync(join(out, 'index.html'), page(content.tr));
writeFileSync(join(out, 'en', 'index.html'), page(content.en));
writeFileSync(join(out, '.nojekyll'), '');
console.log(`site/dist yazıldı (Robz Hub ${live.hubVersion}, ${catalog.apps.length} uygulama, yıldız: ${live.stars ?? '-'})`);
