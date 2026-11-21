// Arayüzün girişi: Rust çekirdeğinden durumu alır, kabuğu (kenar çubuğu, üst çubuk, sayfa) çizer.
import './bridge.js';
import { html, render, useEffect, useRef } from '../vendor/preact-htm.js';
import { Icon } from './icons.js';
import { lang, pick, t } from './i18n.js';
import { imageUrl, relative } from './format.js';
import { closeDialog, dismissToast, navigate, store, toast, update, useStore } from './store.js';
import * as actions from './actions.js';
import { AppIcon, LanguageSelect, Spinner } from './components.js';
import { Home } from './views/home.js';
import { AppPage } from './views/app-page.js';
import { Updates } from './views/updates.js';
import { Settings } from './views/settings.js';

// ---- kenar çubuğu -------------------------------------------------------------------------------------------------

function NavItem({ icon, label, active, onClick, trailing, children }) {
  return html`<button class="nav-item" aria-current=${active ? 'page' : undefined} onClick=${onClick}>
    ${children || html`<${Icon} name=${icon} />`}
    <span class="nav-label">${label}</span>
    ${trailing}
  </button>`;
}

function Sidebar() {
  const { data, route } = useStore();
  const pending = data.catalog.apps.filter((def) => data.apps[def.id].updateAvailable).length + (data.self.updateAvailable ? 1 : 0);
  const links = data.catalog.links;

  return html`<aside class="sidebar">
    <div class="brand">
      <img src="/assets/hub/icon.png" width="24" height="24" alt="" />
      <span>Robz Hub</span>
    </div>

    <nav class="nav">
      <${NavItem} icon="home" label=${t('nav.home')} active=${route.view === 'home'} onClick=${() => navigate({ view: 'home' })} />
      <${NavItem}
        icon="download"
        label=${t('nav.updates')}
        active=${route.view === 'updates'}
        onClick=${() => navigate({ view: 'updates' })}
        trailing=${pending > 0 && html`<span class="counter counter-accent">${pending}</span>`}
      />
    </nav>

    <div class="nav-heading">${t('nav.apps')}</div>
    <nav class="nav">
      ${data.catalog.apps.map((def) => {
        const app = data.apps[def.id];
        const job = data.jobs[def.id];
        let trailing = null;
        if (job && job.phase !== 'error') trailing = html`<${Spinner} size=${14} />`;
        else if (app.running) trailing = html`<span class="dot dot-success" title=${t('status.running')}></span>`;
        else if (app.updateAvailable) trailing = html`<span class="dot dot-accent" title=${t('status.updateAvailable')}></span>`;
        return html`<${NavItem}
          key=${def.id}
          label=${def.name}
          active=${route.view === 'app' && route.id === def.id}
          onClick=${() => navigate({ view: 'app', id: def.id, tab: 'overview' })}
          trailing=${trailing}
        >
          <${AppIcon} def=${def} size=${20} />
        <//>`;
      })}
    </nav>

    <div class="sidebar-foot">
      <nav class="nav">
        <${NavItem}
          icon="sliders"
          label=${t('nav.settings')}
          active=${route.view === 'settings'}
          onClick=${() => navigate({ view: 'settings' })}
        />
      </nav>
      <div class="sidebar-meta">
        <${LanguageSelect}
          up
          label=${t('settings.language')}
          value=${lang()}
          onChange=${(value) => actions.run('setSetting', 'language', value)}
          options=${[
            { value: 'tr', label: t('lang.tr'), flag: 'tr' },
            { value: 'en', label: t('lang.en'), flag: 'en' },
          ]}
        />
        <div class="sidebar-links">
          ${links.github && html`<a href=${links.github} title=${t('link.github')}><${Icon} name="github" /></a>`}
          ${links.discord && html`<a href=${links.discord} title=${t('link.discord')}><${Icon} name="chat" /></a>`}
          ${links.kick && html`<a href=${links.kick} title=${t('link.kick')}><${Icon} name="live" /></a>`}
        </div>
      </div>
    </div>
  </aside>`;
}

// ---- üst çubuk ----------------------------------------------------------------------------------------------------

function Topbar() {
  const { data, route } = useStore();
  const { check } = data;
  let title = t('nav.home');
  if (route.view === 'updates') title = t('nav.updates');
  if (route.view === 'settings') title = t('nav.settings');
  const def = route.view === 'app' ? data.catalog.apps.find((a) => a.id === route.id) : null;

  return html`<header class="topbar">
    <div class="crumbs">
      ${def
        ? html`<button class="crumb" onClick=${() => navigate({ view: 'home' })}>${t('app.breadcrumb')}</button>
            <span class="crumb-sep">/</span>
            <span class="crumb-current">${def.name}</span>`
        : html`<span class="crumb-current">${title}</span>`}
    </div>
    <div class="topbar-status">
      ${check.problem === 'offline' && !check.checking
        ? html`<span class="status-offline"><${Icon} name="alert" size=${14} />${t('topbar.offline')}</span>`
        : html`<span class="muted">
            ${check.checking
              ? t('updates.checking')
              : check.lastChecked
                ? t('updates.lastChecked', { time: relative(check.lastChecked) })
                : ''}
          </span>`}
      <button
        class="btn btn-icon btn-invisible"
        title=${t('action.check')}
        aria-label=${t('action.check')}
        disabled=${check.checking}
        onClick=${actions.checkNow}
      >
        ${check.checking ? html`<${Spinner} />` : html`<${Icon} name="sync" />`}
      </button>
    </div>
  </header>`;
}

// ---- onay penceresi, görsel penceresi, bildirimler ----------------------------------------------------------------

function ConfirmDialog() {
  const { dialog } = useStore();
  const ref = useRef(null);
  useEffect(() => {
    if (dialog && ref.current && !ref.current.open) ref.current.showModal();
  }, [dialog]);
  if (!dialog) return null;
  const confirm = () => {
    closeDialog();
    dialog.onConfirm();
  };
  return html`<dialog class="dialog" ref=${ref} onClose=${closeDialog} onCancel=${closeDialog}>
    <h2>${dialog.title}</h2>
    <p class="muted">${dialog.body}</p>
    <div class="dialog-actions">
      <button class="btn" onClick=${closeDialog}>${t('dialog.cancel')}</button>
      <button class=${`btn ${dialog.danger ? 'btn-danger-solid' : 'btn-primary'}`} onClick=${confirm}>${dialog.confirm}</button>
    </div>
  </dialog>`;
}

function Lightbox() {
  const { lightbox } = useStore();
  const ref = useRef(null);
  useEffect(() => {
    if (lightbox && ref.current && !ref.current.open) ref.current.showModal();
  }, [lightbox]);
  if (!lightbox) return null;
  const { images, index } = lightbox;
  const close = () => update({ lightbox: null });
  const go = (step) => update({ lightbox: { images, index: (index + step + images.length) % images.length } });
  const onKey = (event) => {
    if (event.key === 'ArrowLeft') go(-1);
    if (event.key === 'ArrowRight') go(1);
  };
  const image = images[index];
  return html`<dialog
    class="lightbox"
    ref=${ref}
    onClose=${close}
    onKeyDown=${onKey}
    onClick=${(event) => {
      if (event.target === ref.current) close();
    }}
  >
    <figure>
      <img src=${imageUrl(image.url)} alt=${pick(image.caption)} />
      <figcaption>
        <span>${pick(image.caption)}</span>
        <span class="lightbox-nav">
          ${images.length > 1 &&
          html`<button class="btn btn-icon" aria-label=${t('lightbox.prev')} onClick=${() => go(-1)}>
              <${Icon} name="chevron-right" class="flip" />
            </button>
            <span class="mono muted">${index + 1} / ${images.length}</span>
            <button class="btn btn-icon" aria-label=${t('lightbox.next')} onClick=${() => go(1)}>
              <${Icon} name="chevron-right" />
            </button>`}
          <button class="btn btn-icon" aria-label=${t('lightbox.close')} onClick=${close}><${Icon} name="x" /></button>
        </span>
      </figcaption>
    </figure>
  </dialog>`;
}

function Toasts() {
  const { toasts } = useStore();
  return html`<div class="toasts" aria-live="polite">
    ${toasts.map(
      (item) => html`<div class=${`toast toast-${item.kind}`} key=${item.id}>
        <${Icon} name=${item.kind === 'error' ? 'alert' : item.kind === 'success' ? 'check' : 'info'} />
        <span class="toast-text">${item.message}</span>
        ${item.action &&
        html`<button
          class="btn btn-sm"
          onClick=${() => {
            dismissToast(item.id);
            item.action.onSelect();
          }}
        >
          ${item.action.label}
        </button>`}
        <button class="btn btn-icon btn-invisible btn-sm" aria-label=${t('lightbox.close')} onClick=${() => dismissToast(item.id)}>
          <${Icon} name="x" size=${14} />
        </button>
      </div>`,
    )}
  </div>`;
}

// ---- kök ----------------------------------------------------------------------------------------------------------

function View() {
  const { route } = useStore();
  if (route.view === 'app') return html`<${AppPage} id=${route.id} tab=${route.tab} key=${route.id} />`;
  if (route.view === 'updates') return html`<${Updates} />`;
  if (route.view === 'settings') return html`<${Settings} />`;
  return html`<${Home} />`;
}

function App() {
  const { data } = useStore();
  useEffect(() => {
    if (data) document.documentElement.lang = lang();
  });
  if (!data) return html`<div class="loading"><${Spinner} size=${20} /></div>`;
  return html`<div class="shell">
    <${Sidebar} />
    <div class="main">
      <${Topbar} />
      <main class="content"><${View} /></main>
    </div>
    <${Toasts} />
    <${ConfirmDialog} />
    <${Lightbox} />
  </div>`;
}

// Ana süreçten gelen sonuçlar: kuruldu / güncellendi / kaldırıldı / yeni sürüm bulundu.
function onNotice(notice) {
  const def = store.data?.catalog.apps.find((a) => a.id === notice.appId);
  if (notice.type === 'installed' || notice.type === 'updated') {
    toast(t(`toast.${notice.type}`, { name: notice.name, version: notice.version }), {
      kind: 'success',
      timeout: 10000,
      action: def && { label: t('action.launch'), onSelect: () => actions.launch(def) },
    });
  } else if (notice.type === 'uninstalled') {
    toast(t('toast.uninstalled', { name: notice.name }), { kind: 'success' });
  } else if (notice.type === 'updates-found' && store.route.view !== 'updates') {
    toast(t('toast.updatesFound', { n: notice.apps.length }), {
      action: { label: t('toast.show'), onSelect: () => navigate({ view: 'updates' }) },
    });
  }
}

window.hub.on('state', (data) => update({ data }));
window.hub.on('progress', (p) => update({ progress: { ...store.progress, [p.appId]: p } }));
window.hub.on('notice', onNotice);
window.hub.on('navigate', (route) => navigate(route));

// Dış bağlantılar (sürüm notlarındakiler dahil) varsayılan tarayıcıda açılır.
document.addEventListener('click', (event) => {
  const link = event.target.closest?.('a[href]');
  if (!link) return;
  const href = link.getAttribute('href');
  if (/^https:\/\//i.test(href)) {
    event.preventDefault();
    actions.openExternal(href);
  } else if (href !== '#') {
    event.preventDefault();
  }
});

// Tarayıcının sağ tık menüsü (Geri, Yenile, İncele) yalnız seçilebilir metinde açılır.
document.addEventListener('contextmenu', (event) => {
  if (!event.target.closest?.('.selectable, input, textarea')) event.preventDefault();
});

// "3 dakika önce" gibi göreli zamanlar güncel kalsın.
setInterval(() => update({}), 60000);

render(html`<${App} />`, document.getElementById('root'));
window.hub.call('getState').then((result) => {
  if (result.ok) update({ data: result.value });
});
