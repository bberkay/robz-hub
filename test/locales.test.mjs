// Türkçe ve İngilizce arayüz metinleri aynı anahtarlara sahip olmalı ve arayüzde kullanılan her anahtar bulunmalı.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import tr from '../ui/js/locales/tr.js';
import en from '../ui/js/locales/en.js';

const root = new URL('../ui/js/', import.meta.url);

function sources(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === 'locales' ? [] : sources(path);
    return entry.name.endsWith('.js') ? [readFileSync(path, 'utf8')] : [];
  });
}

test('iki dilde aynı anahtarlar var', () => {
  assert.deepEqual(Object.keys(en).sort(), Object.keys(tr).sort());
});

test('yer tutucular iki dilde aynı', () => {
  for (const key of Object.keys(tr)) {
    if (typeof tr[key] !== 'string') continue;
    const vars = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    assert.deepEqual(vars(en[key]), vars(tr[key]), key);
  }
});

test("arayüzde t('…') ile kullanılan her anahtar sözlükte var", () => {
  const used = new Set();
  for (const text of sources(root.pathname.replace(/^\/([A-Za-z]:)/, '$1'))) {
    for (const m of text.matchAll(/\bt\('([a-zA-Z0-9.-]+)'/g)) used.add(m[1]);
  }
  const missing = [...used].filter((key) => !(key in tr));
  assert.deepEqual(missing, []);
  assert.ok(used.size > 50);
});
