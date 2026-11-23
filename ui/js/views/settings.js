// Ayarlar: dil, başlangıç, güncelleme denetimi, indirilen dosyalar ve hakkında.
import { html } from '../../vendor/preact-htm.js';
import { Icon } from '../icons.js';
import { t } from '../i18n.js';
import { bytes } from '../format.js';
import { toast, useStore } from '../store.js';
import * as actions from '../actions.js';
import { LanguageSelect, Toggle } from '../components.js';

const set = (key, value) => actions.run('setSetting', key, value);

function Row({ title, desc, children }) {
  return html`<div class="setting">
    <div class="setting-text">
      <div class="setting-title">${title}</div>
      ${desc && html`<p class="muted">${desc}</p>`}
    </div>
    <div class="setting-control">${children}</div>
  </div>`;
}

function ToggleRow({ name, title, desc, disabled = false }) {
  const { data } = useStore();
  return html`<${Row} title=${title} desc=${desc}>
    <${Toggle} checked=${data.settings[name]} label=${title} disabled=${disabled} onChange=${(value) => set(name, value)} />
  <//>`;
}

export function Settings() {
  const { data } = useStore();
  const linux = data.platform === 'linux';
  const { settings } = data;
  const links = data.catalog.links;
  const busy = Object.values(data.jobs).some((job) => job.phase !== 'error');

  const clearCache = async () => {
    await actions.run('clearCache');
    toast(t('toast.cacheCleared'));
  };

  return html`<div class="page page-narrow">
    <header class="page-head"><h1>${t('settings.title')}</h1></header>

    ${data.dryRun && html`<div class="flash flash-attention"><${Icon} name="alert" /><div class="flash-body">${t('settings.dryRun')}</div></div>`}

    <section class="section">
      <h2 class="section-title">${t('settings.language')}</h2>
      <div class="box">
        <${Row} title=${t('settings.language')} desc=${t('settings.language.desc')}>
          <${LanguageSelect}
            label=${t('settings.language')}
            value=${settings.language}
            onChange=${(value) => set('language', value)}
            options=${[
              { value: 'auto', label: t(linux ? 'settings.language.auto.linux' : 'settings.language.auto'), flag: 'auto' },
              { value: 'tr', label: t('lang.tr'), flag: 'tr' },
              { value: 'en', label: t('lang.en'), flag: 'en' },
            ]}
          />
        <//>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">${t('settings.startup')}</h2>
      <div class="box">
        <${ToggleRow}
          name="launchAtLogin"
          title=${t(linux ? 'settings.launchAtLogin.linux' : 'settings.launchAtLogin')}
          desc=${data.packaged ? t('settings.launchAtLogin.desc') : t('settings.launchAtLogin.dev')}
          disabled=${!data.packaged}
        />
        <${ToggleRow} name="closeToTray" title=${t('settings.closeToTray')} desc=${t('settings.closeToTray.desc')} />
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">${t('settings.updates')}</h2>
      <div class="box">
        <${ToggleRow} name="autoCheck" title=${t('settings.autoCheck')} desc=${t('settings.autoCheck.desc')} />
        <${ToggleRow} name="notifyUpdates" title=${t('settings.notifyUpdates')} desc=${t(linux ? 'settings.notifyUpdates.desc.linux' : 'settings.notifyUpdates.desc')} />
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">${t('settings.downloads')}</h2>
      <div class="box">
        <${ToggleRow} name="keepInstallers" title=${t('settings.keepInstallers')} desc=${t('settings.keepInstallers.desc')} />
        <${Row}
          title=${t('settings.cache')}
          desc=${data.cacheBytes > 0 ? t('settings.cache.desc', { size: bytes(data.cacheBytes) }) : t('settings.cache.empty')}
        >
          <button class="btn" disabled=${busy || data.cacheBytes === 0} onClick=${clearCache}>
            <${Icon} name="trash" />${t('settings.cache.clear')}
          </button>
        <//>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">${t('settings.about')}</h2>
      <div class="box about-box">
        <img class="app-icon" src="/assets/hub/icon.png" width="56" height="56" alt="" />
        <div>
          <div class="about-name">Robz Hub <span class="mono muted">${data.hubVersion}</span></div>
          <p>${t('settings.about.body')}</p>
          <p class="muted">${t('settings.about.disclaimer')}</p>
          <ul class="about-links">
            ${links.github && html`<li><a href=${links.github}><${Icon} name="github" />${t('link.github')}</a></li>`}
            ${links.discord && html`<li><a href=${links.discord}><${Icon} name="discord" />${t('link.discord')}</a></li>`}
            ${links.kick && html`<li><a href=${links.kick}><${Icon} name="kick" />${t('link.kick')}</a></li>`}
          </ul>
        </div>
      </div>
    </section>
  </div>`;
}
