// Boyut, hız, tarih ve süre biçimleri (geçerli dile göre).
import { lang, t } from './i18n.js';

const locale = () => (lang() === 'tr' ? 'tr-TR' : 'en-US');

function number(value, digits) {
  return new Intl.NumberFormat(locale(), { maximumFractionDigits: digits, minimumFractionDigits: 0 }).format(value);
}

export function bytes(n) {
  if (!Number.isFinite(n) || n <= 0) return '0 MB';
  const mb = n / (1024 * 1024);
  if (mb >= 1024) return `${number(mb / 1024, 2)} GB`;
  if (mb >= 100) return `${number(mb, 0)} MB`;
  if (mb >= 1) return `${number(mb, 1)} MB`;
  return `${number(n / 1024, 0)} KB`;
}

// 12345 -> "12.345" (Türkçe) / "12,345" (İngilizce)
export function count(n) {
  return number(n || 0, 0);
}

export function speed(bytesPerSecond) {
  if (!bytesPerSecond) return '';
  return `${bytes(bytesPerSecond)}${t('unit.perSecond')}`;
}

export function percent(received, total) {
  if (!total) return 0;
  return Math.max(0, Math.min(100, Math.floor((received / total) * 100)));
}

// Kalan süre: "40 sn", "3 dk"
export function remaining(received, total, bytesPerSecond) {
  if (!total || !bytesPerSecond || received >= total) return '';
  const seconds = Math.ceil((total - received) / bytesPerSecond);
  if (seconds < 60) return t('time.secondsLeft', { n: Math.max(1, seconds) });
  return t('time.minutesLeft', { n: Math.ceil(seconds / 60) });
}

export function dateLong(value) {
  if (!value) return '';
  return new Intl.DateTimeFormat(locale(), { day: 'numeric', month: 'long', year: 'numeric' }).format(new Date(value));
}

export function relative(value) {
  if (!value) return '';
  const diff = (new Date(value).getTime() - Date.now()) / 1000;
  const format = new Intl.RelativeTimeFormat(locale(), { numeric: 'auto' });
  const abs = Math.abs(diff);
  if (abs < 60) return t('time.justNow');
  if (abs < 3600) return format.format(Math.round(diff / 60), 'minute');
  if (abs < 86400) return format.format(Math.round(diff / 3600), 'hour');
  if (abs < 86400 * 30) return format.format(Math.round(diff / 86400), 'day');
  return dateLong(value);
}

// Katalogdaki görsel yolu: uygulamayla gelen dosya ("assets/…") ya da https adresi.
export function imageUrl(value) {
  if (!value) return null;
  return value.startsWith('assets/') ? `/${value}` : value;
}
