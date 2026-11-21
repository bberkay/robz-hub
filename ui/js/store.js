// Arayüzün tek durum deposu. Ana süreçten gelen durum (data), indirme ilerlemesi, açık sayfa (route),
// bildirim baloncukları (toasts) ve onay penceresi (dialog) burada tutulur.
import { useEffect, useState } from '../vendor/preact-htm.js';

const listeners = new Set();
let toastId = 0;

export const store = {
  data: null,
  progress: {}, // uygulama kimliği -> { received, total, speed }
  route: { view: 'home' }, // { view: 'home' | 'updates' | 'settings' } | { view: 'app', id, tab }
  toasts: [],
  dialog: null,
  lightbox: null,
};

export function update(patch) {
  Object.assign(store, patch);
  for (const listener of listeners) listener();
}

export function useStore() {
  const [, rerender] = useState(0);
  useEffect(() => {
    const listener = () => rerender((n) => n + 1);
    listeners.add(listener);
    return () => listeners.delete(listener);
  }, []);
  return store;
}

export function navigate(route) {
  update({ route });
  document.querySelector('.content')?.scrollTo(0, 0);
}

export function toast(message, { kind = 'info', action = null, timeout = 6000 } = {}) {
  const id = ++toastId;
  update({ toasts: [...store.toasts, { id, message, kind, action }] });
  if (timeout) setTimeout(() => dismissToast(id), timeout);
}

export function dismissToast(id) {
  update({ toasts: store.toasts.filter((t) => t.id !== id) });
}

export function openDialog(dialog) {
  update({ dialog });
}

export function closeDialog() {
  update({ dialog: null });
}

// Ana süreç çağrısı. Hata olursa { code, detail } fırlatır.
export async function call(method, ...args) {
  const result = await window.hub.call(method, ...args);
  if (!result.ok) throw result.error;
  return result.value;
}
