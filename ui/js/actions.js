// Kullanıcı eylemleri: gerekirse onay ister, ana süreci çağırır, hatayı okunur bir bildirime çevirir.
import { call, openDialog, store, toast } from './store.js';
import { t } from './i18n.js';

export function errorText(error) {
  const code = error?.code || 'unknown';
  const key = `error.${code}`;
  const message = t(key);
  return message === key ? t('error.unknown') : message;
}

export async function run(method, ...args) {
  try {
    return await call(method, ...args);
  } catch (error) {
    toast(errorText(error), { kind: 'error' });
    return undefined;
  }
}

// Kur / güncelle / yeniden kur. Uygulama açıksa önce kapatılacağını sorar.
export function install(def) {
  const app = store.data.apps[def.id];
  if (!app.running) return run('install', def.id, {});
  return openDialog({
    title: t('dialog.closeFirst.title', { name: def.name }),
    body: t('dialog.closeFirst.body', { name: def.name }),
    confirm: t('dialog.closeFirst.confirm'),
    onConfirm: () => run('install', def.id, { closeIfRunning: true }),
  });
}

export function uninstall(def) {
  const app = store.data.apps[def.id];
  const body = t('dialog.uninstall.body', { name: def.name });
  openDialog({
    title: t('dialog.uninstall.title', { name: def.name }),
    body: app.running ? `${body} ${t('dialog.uninstall.running')}` : body,
    confirm: t('action.uninstall'),
    danger: true,
    onConfirm: () => run('uninstall', def.id, { closeIfRunning: true }),
  });
}

// Hata veren işi yeniden dener; "uygulama açık" hatasında bu kez kapatarak.
export function retry(def, job) {
  const closeIfRunning = job.error?.code === 'app-running';
  if (job.kind === 'uninstall') return run('uninstall', def.id, { closeIfRunning: true });
  return run('install', def.id, { closeIfRunning });
}

export const launch = (def) => run('launch', def.id);
export const stop = (def) => run('stop', def.id);
export const cancel = (id) => run('cancel', id);
export const dismissJob = (id) => run('dismissJob', id);
export const openExternal = (url) => run('openExternal', url);
export const checkNow = () => run('check');
