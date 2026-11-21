// Arayüzün kullandığı üç parçayı node_modules'tan ui/vendor'a kopyalar.
// Paketleyici (bundler) yok: arayüz düz ES modülleriyle çalışır, bu dosyalar da öyle yüklenir.
// `npm install` sonunda kendiliğinden çalışır.
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const out = path.join(root, 'ui', 'vendor');

const copies = [
  ['htm/preact/standalone.module.js', 'preact-htm.js'],
  ['marked/lib/marked.esm.js', 'marked.js'],
  ['@fontsource-variable/mona-sans/files/mona-sans-latin-wght-normal.woff2', 'fonts/mona-sans-latin.woff2'],
  ['@fontsource-variable/mona-sans/files/mona-sans-latin-ext-wght-normal.woff2', 'fonts/mona-sans-latin-ext.woff2'],
  ['@fontsource-variable/mona-sans/LICENSE', 'fonts/OFL.txt'],
];

let missing = 0;
for (const [from, to] of copies) {
  const src = path.join(root, 'node_modules', from);
  const dest = path.join(out, to);
  if (!fs.existsSync(src)) {
    console.warn(`vendor: bulunamadı: ${from}`);
    missing += 1;
    continue;
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
}
console.log(`vendor: ${copies.length - missing}/${copies.length} dosya kopyalandı -> ui/vendor`);
