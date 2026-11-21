// Arayüz ile Rust çekirdeği arasındaki tek köprü (Tauri).
//   window.hub.call(method, ...args)  -> { ok: true, value } | { ok: false, error: { code, detail } }
//   window.hub.on(event, callback)    -> 'state' | 'progress' | 'notice' | 'navigate'; dinlemeyi bırakan işlev döner
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const EVENTS = new Set(['state', 'progress', 'notice', 'navigate']);

window.hub = {
  async call(method, ...args) {
    try {
      return { ok: true, value: await invoke('hub_call', { method, args }) };
    } catch (error) {
      const known = error && typeof error === 'object' && 'code' in error;
      return { ok: false, error: known ? error : { code: 'unknown', detail: String(error) } };
    }
  },
  on(event, callback) {
    if (!EVENTS.has(event)) return () => {};
    const unlisten = listen(`hub:${event}`, (e) => callback(e.payload));
    return () => unlisten.then((stop) => stop());
  },
};
