// Arayüzün tek durum deposu. Ana süreçten gelen durum (data), indirme ilerlemesi, açık sayfa (route),
// bildirim baloncukları (toasts) ve onay penceresi (dialog) burada tutulur.
import { useLayoutEffect, useState } from '../vendor/preact-htm.js';

const listeners = new Set();
let toastId = 0;
let version = 0; // her güncellemede artar

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
  version += 1;
  for (const listener of listeners) listener();
}

// Ana süreçten gelen durum. Her durumun bir sıra numarası (seq) vardır: geç gelen eski bir durum (ör. açılıştaki ilk
// isteğin cevabı ya da sırası karışmış bir olay) yenisinin üzerine yazılmaz.
export function applyState(data) {
  if (!data || typeof data.seq !== 'number') return;
  if (store.data && data.seq <= store.data.seq) return;
  update({ data });
}

export function useStore() {
  const [, rerender] = useState(0);
  const seen = version;
  // Dinleyici çizimle aynı anda kaydedilir (useLayoutEffect). useEffect bir sonraki kareye, en geç 100 ms sonraya
  // ertelenir; o arada gelen güncelleme kaçardı. Linux'ta açılıştaki ilk durum 20 ms'de gelir: ekran, bir sonraki
  // güncellemeye (GitHub denetiminin bitişine) kadar çarkta kalıyordu.
  useLayoutEffect(() => {
    const listener = () => rerender((n) => n + 1);
    listeners.add(listener);
    if (version !== seen) listener(); // çizim ile kayıt arasında bir güncelleme geldiyse
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
