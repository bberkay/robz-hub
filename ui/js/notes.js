// Sürüm notlarından gösterilecek bölümü seçer. Yalnız metin işler (DOM yok); test/notes.test.js sınar.
//
// GitHub sürüm notu, uygulamanın içinde gösterilmesi gerekenden fazlasını içerir ("hangi dosyayı indirmeliyim",
// SmartScreen uyarısı, SHA-256…). Robz Hub bunları zaten kendisi hallettiği için yalnız "neler değişti" kısmı gösterilir.
//
// İki kural, sırayla:
//   1. Dil işaretleri:  <!-- robzplay:tr --> … <!-- robzplay:en --> … <!-- robzplay:end -->
//      (ön ek serbesttir; <!-- tr --> de olur). Yalnız seçili dilin bloğu gösterilir; yoksa diğer dil.
//   2. İşaret yoksa ve katalog bir başlık verdiyse ("## Bu sürümde"): o başlıktan sonrası gösterilir.
//   Hiçbiri tutmazsa notun tamamı gösterilir.

const MARKER = /<!--\s*(?:[\w-]+:)?(tr|en|end)\s*-->/gi;

function stripLeadingHeading(markdown) {
  return markdown.replace(/^\s*#{1,2}\s+[^\n]*\n+/, '');
}

// İngilizce blok çoğu zaman <details><summary>English</summary> … </details> içindedir; sarmalı kaldır.
function unwrapDetails(markdown) {
  const m = /^\s*<details>\s*<summary>[^<]*<\/summary>([\s\S]*?)<\/details>\s*$/i.exec(markdown);
  return m ? m[1] : markdown;
}

function splitByMarkers(body) {
  const blocks = {};
  let current = null;
  let last = 0;
  let found = false;
  MARKER.lastIndex = 0;
  for (let m = MARKER.exec(body); m; m = MARKER.exec(body)) {
    found = true;
    if (current) blocks[current] = (blocks[current] || '') + body.slice(last, m.index);
    const name = m[1].toLowerCase();
    current = name === 'end' ? null : name;
    last = MARKER.lastIndex;
  }
  if (current) blocks[current] = (blocks[current] || '') + body.slice(last);
  return found ? blocks : null;
}

// Notun ilk maddesinden (yoksa ilk paragrafından) düz metin bir özet; ana sayfadaki sürüm akışında kullanılır.
export function excerpt(markdown, max = 160) {
  const lines = String(markdown || '').split('\n');
  const isBullet = (line) => /^\s*[-*+]\s+/.test(line);
  let start = lines.findIndex(isBullet);
  if (start === -1) start = lines.findIndex((line) => line.trim() && !/^\s*(#|\||<|>|```|---)/.test(line));
  if (start === -1) return '';

  // Madde sonraki satırlara taşmış olabilir: boş satıra ya da yeni maddeye kadar birleştir.
  let text = lines[start];
  for (let i = start + 1; i < lines.length; i += 1) {
    if (!lines[i].trim() || isBullet(lines[i]) || /^\s*#/.test(lines[i])) break;
    text += ` ${lines[i].trim()}`;
  }
  text = text
    .replace(/^\s*[-*+]\s+/, '')
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/<[^>]+>/g, '')
    .replace(/[*_`~]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (text.length > max) text = `${text.slice(0, max - 1).replace(/\s+\S*$/, '')}…`;
  return text;
}

// Dönüş: { markdown, language } — language, gösterilen metnin dili ('tr' | 'en') ya da bilinmiyorsa null.
export function selectNotes(body, { lang, from = null, languages = [] } = {}) {
  const text = String(body || '').replace(/\r\n/g, '\n');

  const blocks = splitByMarkers(text);
  if (blocks) {
    const language = blocks[lang]?.trim() ? lang : blocks.tr?.trim() ? 'tr' : blocks.en?.trim() ? 'en' : null;
    if (language) {
      return { markdown: stripLeadingHeading(unwrapDetails(blocks[language])).trim(), language };
    }
  }

  const language = languages.length === 1 ? languages[0] : null;
  if (from) {
    const lines = text.split('\n');
    const index = lines.findIndex((line) => line.trim() === from.trim());
    if (index !== -1) return { markdown: lines.slice(index + 1).join('\n').trim(), language };
  }
  return { markdown: text.trim(), language };
}
