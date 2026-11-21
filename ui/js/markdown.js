// Sürüm notlarını (GitHub Markdown) güvenli HTML'e çevirir.
//
// Notlar uzaktan gelir; bu yüzden marked'ın çıktısı doğrudan basılmaz. Önce ayrı bir belgeye ayrıştırılır, yalnız izin
// verilen etiketler ve öznitelikler bırakılır (betik, stil, olay öznitelikleri, javascript: adresleri atılır), sonra
// sayfaya eklenir. Sayfanın içerik güvenlik ilkesi (index.html) ikinci savunma hattıdır.
import { marked } from '../vendor/marked.js';

const ALLOWED = new Set([
  'p', 'br', 'hr', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'li', 'strong', 'b', 'em', 'i', 'del', 's', 'code',
  'pre', 'blockquote', 'a', 'img', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'details', 'summary', 'kbd', 'sub',
  'sup', 'span', 'div',
]);
const DROP = new Set([
  'script', 'style', 'iframe', 'object', 'embed', 'form', 'svg', 'math', 'link', 'meta', 'base', 'template',
  'noscript', 'input', 'button', 'textarea', 'select', 'video', 'audio', 'picture', 'source',
]);
const ATTRS = {
  a: ['href', 'title'],
  img: ['src', 'alt', 'title', 'width', 'height'],
  th: ['align'],
  td: ['align'],
  p: ['align'],
  div: ['align'],
  ol: ['start'],
};
const ALERTS = { NOTE: 'note', TIP: 'tip', IMPORTANT: 'important', WARNING: 'warning', CAUTION: 'caution' };

// Göreli bağlantıları depoya göre tamamlar; https olmayanları atar.
function resolveUrl(value, repo, raw) {
  const url = String(value || '').trim();
  if (/^https:\/\//i.test(url)) return url;
  if (!url || url.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith('//')) return null;
  if (!repo) return null;
  const path = url.replace(/^\.?\//, '');
  return raw
    ? `https://raw.githubusercontent.com/${repo}/HEAD/${path}`
    : `https://github.com/${repo}/blob/HEAD/${path}`;
}

function clean(node, repo) {
  for (const child of [...node.childNodes]) {
    if (child.nodeType === Node.COMMENT_NODE) {
      child.remove();
      continue;
    }
    if (child.nodeType !== Node.ELEMENT_NODE) continue;
    const tag = child.tagName.toLowerCase();
    if (DROP.has(tag)) {
      child.remove();
      continue;
    }
    clean(child, repo);
    if (!ALLOWED.has(tag)) {
      child.replaceWith(...child.childNodes); // bilinmeyen etiket: içeriği kalsın, kendisi gitsin
      continue;
    }
    const keep = ATTRS[tag] || [];
    for (const attr of [...child.attributes]) {
      if (!keep.includes(attr.name)) child.removeAttribute(attr.name);
    }
    if (tag === 'a') {
      const href = resolveUrl(child.getAttribute('href'), repo, false);
      if (href) child.setAttribute('href', href);
      else child.removeAttribute('href');
    }
    if (tag === 'img') {
      const src = resolveUrl(child.getAttribute('src'), repo, true);
      if (!src) {
        child.remove();
        continue;
      }
      child.setAttribute('src', src);
      child.setAttribute('loading', 'lazy');
    }
    if (tag === 'blockquote') markAlert(child);
  }
}

// GitHub'ın "> [!NOTE]" uyarı kutuları.
function markAlert(blockquote) {
  const first = blockquote.querySelector('p');
  const m = first && /^\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*/.exec(first.textContent);
  if (!m) return;
  blockquote.className = `alert alert-${ALERTS[m[1]]}`;
  const text = first.firstChild;
  if (text?.nodeType === Node.TEXT_NODE) text.textContent = text.textContent.replace(/^\s*\[![A-Z]+\]\s*/, '');
  if (first.firstChild?.nodeName === 'BR') first.firstChild.remove();
}

const cache = new Map();

export function renderMarkdown(markdown, repo) {
  const key = `${repo}\n${markdown}`;
  if (cache.has(key)) return cache.get(key);
  const html = marked.parse(markdown, { gfm: true, breaks: false, async: false });
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  clean(doc.body, repo);
  const safe = doc.body.innerHTML;
  if (cache.size > 200) cache.clear();
  cache.set(key, safe);
  return safe;
}
