// Dil: Türkçe ve İngilizce. Ayar "auto" ise Windows'un dili; Türkçe değilse İngilizce.
import { store } from './store.js';
import tr from './locales/tr.js';
import en from './locales/en.js';

const DICTS = { tr, en };

export function lang() {
  const data = store.data;
  // Durum gelmeden önce (açılış ekranı) tarayıcının dili.
  if (!data) return (navigator.language || 'tr').toLowerCase().startsWith('tr') ? 'tr' : 'en';
  const setting = data.settings.language;
  return setting === 'auto' ? data.systemLanguage : setting;
}

// t('nav.home') ya da t('toast.installed', { name: 'Robz Play' }). Sözlükte yoksa anahtarın kendisi döner.
export function t(key, vars) {
  let value = DICTS[lang()][key] ?? DICTS.tr[key] ?? key;
  if (typeof value === 'function') return value(vars || {});
  if (vars) value = value.replace(/\{(\w+)\}/g, (_, name) => (name in vars ? vars[name] : `{${name}}`));
  return value;
}

// Katalogdaki { tr, en } metinlerinden geçerli dildekini seçer.
export function pick(text) {
  if (!text) return '';
  if (typeof text === 'string') return text;
  return text[lang()] ?? text.tr ?? text.en ?? '';
}
