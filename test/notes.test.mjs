// Sürüm notlarından gösterilecek bölümün seçimi (ui/js/notes.js). Örnekler iki deponun gerçek sürüm notu biçimidir.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { excerpt, selectNotes } from '../ui/js/notes.js';

const PLAY = [
  '<!-- robzplay:tr -->',
  "## Robz Play 0.4.0'da neler var",
  '',
  '- **Yeni görünüm.** Yeni logo.',
  '',
  '<!-- robzplay:en -->',
  '<details><summary>English</summary>',
  '',
  "## What's new in Robz Play 0.4.0",
  '',
  '- **A new look.** The new logo.',
  '',
  '</details>',
  '<!-- robzplay:end -->',
  '',
  '## İndir ve kur',
  '',
  'Kurulum dosyasını indir.',
].join('\r\n');

const VISION = [
  '## Hangi dosyayı indirmeliyim?',
  '',
  '| Dosya | Ne için |',
  '|---|---|',
  '',
  '## Bu sürümde',
  '',
  '### Eklendi',
  '',
  '- **Kenar çubuğunda "Güncellemeleri denetle".** Sürüm numarasının',
  '  hemen altında durur.',
].join('\n');

test('dil işaretli notta seçili dilin bloğu gelir', () => {
  const tr = selectNotes(PLAY, { lang: 'tr' });
  assert.equal(tr.language, 'tr');
  assert.equal(tr.markdown, '- **Yeni görünüm.** Yeni logo.');

  const en = selectNotes(PLAY, { lang: 'en' });
  assert.equal(en.language, 'en');
  assert.equal(en.markdown, '- **A new look.** The new logo.');
  assert.ok(!en.markdown.includes('<details>'));
});

test('tek dilli işaretli notta diğer dile düşülür', () => {
  const body = '<!-- tr -->\n- Yalnız Türkçe\n<!-- end -->';
  assert.deepEqual(selectNotes(body, { lang: 'en' }), { markdown: '- Yalnız Türkçe', language: 'tr' });
});

test('katalogdaki başlıktan sonrası gösterilir', () => {
  const notes = selectNotes(VISION, { lang: 'en', from: '## Bu sürümde', languages: ['tr'] });
  assert.equal(notes.language, 'tr');
  assert.ok(notes.markdown.startsWith('### Eklendi'));
  assert.ok(!notes.markdown.includes('Hangi dosyayı'));
});

test('kural tutmazsa notun tamamı gelir', () => {
  assert.deepEqual(selectNotes('Sadece metin', { lang: 'tr', from: '## Yok' }), { markdown: 'Sadece metin', language: null });
  assert.deepEqual(selectNotes(null, { lang: 'tr' }), { markdown: '', language: null });
});

test('özet ilk maddeden düz metin çıkarır', () => {
  const notes = selectNotes(VISION, { lang: 'tr', from: '## Bu sürümde' });
  assert.equal(excerpt(notes.markdown), 'Kenar çubuğunda "Güncellemeleri denetle". Sürüm numarasının hemen altında durur.');
  assert.equal(excerpt('[bağlantı](https://x.y) ve `kod`'), 'bağlantı ve kod');
  assert.ok(excerpt(`- ${'kelime '.repeat(60)}`, 40).endsWith('…'));
  assert.equal(excerpt(''), '');
});
