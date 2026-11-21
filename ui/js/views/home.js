// Ana sayfa: birlikte kullanılan uygulamalar için tek tıklık paket, uygulama listesi ve son sürümler.
import { html } from '../../vendor/preact-htm.js';
import { Icon } from '../icons.js';
import { lang, pick, t } from '../i18n.js';
import { bytes, relative } from '../format.js';
import { excerpt, selectNotes } from '../notes.js';
import { navigate, useStore } from '../store.js';
import * as actions from '../actions.js';
import { AppActions, AppIcon, AppLink, DownloadCount, JobError, RunningBadge, Spinner, VersionState } from '../components.js';

// Paketin anlattığı şeyin küçük bir çizimi: 60 FPS sayacı ve İngilizce altyazının üstünde Türkçesi.
// Cümleler Robz Vision'ın kendi örnek cümleleridir; oyundan alıntı değildir.
function BundleScreen() {
  return html`<figure class="screen" role="img" aria-label=${t('bundle.screen.alt')}>
    <div class="screen-sky"></div>
    <div class="screen-sun"></div>
    <div class="screen-ground"></div>
    <div class="screen-fps mono">
      <strong>${t('bundle.screen.fps')}</strong>
      <span>${t('bundle.screen.source')}</span>
    </div>
    <div class="screen-subs">
      <p class="screen-tr">Yanımdan ayrılma. İş ters giderse marinada buluşuruz.</p>
      <p class="screen-en">Stay close. If this goes wrong, we meet at the marina.</p>
    </div>
  </figure>`;
}

function BundleHero({ bundle }) {
  const { data } = useStore();
  const members = bundle.apps.map((id) => ({
    def: data.catalog.apps.find((a) => a.id === id),
    app: data.apps[id],
    job: data.jobs[id],
  }));
  const n = members.length;
  const missing = members.filter((m) => !m.app.install.installed);
  const outdated = members.filter((m) => m.app.install.installed && m.app.updateAvailable);
  const busy = members.some((m) => m.job && m.job.phase !== 'error');
  const allRunning = members.every((m) => m.app.running);
  const pending = [...missing, ...outdated];
  const downloadSize = pending.reduce((sum, m) => sum + (m.app.latest?.size || 0), 0);
  const installable = pending.length > 0 && pending.every((m) => m.app.latest);

  let cta;
  if (busy) {
    cta = html`<button class="btn btn-primary btn-lg" disabled><${Spinner} />${t('bundle.working')}</button>`;
  } else if (missing.length) {
    cta = html`<button
        class="btn btn-primary btn-lg"
        disabled=${!installable}
        onClick=${() => actions.run('installBundle', bundle.id)}
      >
        <${Icon} name="download" />
        ${missing.length === n
          ? t('bundle.installAll', { n })
          : outdated.length
            ? t('bundle.installAndUpdate')
            : t('bundle.installMissing', { n: missing.length })}
      </button>
      ${downloadSize > 0 && html`<span class="hero-size muted">${t('bundle.totalSize', { size: bytes(downloadSize) })}</span>`}`;
  } else if (allRunning) {
    cta = html`<${RunningBadge} /><span class="muted">${t('bundle.allRunning', { n })}</span>`;
  } else {
    cta = html`<button class="btn btn-primary btn-lg" onClick=${() => actions.run('launchBundle', bundle.id)}>
        <${Icon} name="play" />${t('bundle.launchAll', { n })}
      </button>
      ${outdated.length > 0 &&
      html`<button class="btn btn-lg" disabled=${!installable} onClick=${() => actions.run('installBundle', bundle.id)}>
        <${Icon} name="download" />${t('action.update')}
      </button>`}`;
  }

  return html`<section class="hero">
    <div class="hero-text">
      <h1>${pick(bundle.title)}</h1>
      <p class="hero-desc">${pick(bundle.description)}</p>
      <ul class="hero-apps">
        ${members.map(
          ({ def, app, job }) => html`<li key=${def.id}>
            <${AppIcon} def=${def} size=${28} />
            <${AppLink} def=${def} />
            ${def.role && html`<span class="muted">${pick(def.role)}</span>`}
            <span class="hero-app-state">
              ${job && job.phase !== 'error'
                ? html`<${Spinner} size=${14} />`
                : app.running
                  ? html`<${RunningBadge} />`
                  : html`<${VersionState} app=${app} />`}
            </span>
          </li>`,
        )}
      </ul>
      <div class="hero-cta">${cta}</div>
    </div>
    <${BundleScreen} />
  </section>`;
}

function AppRow({ def }) {
  const { data } = useStore();
  const app = data.apps[def.id];
  const job = data.jobs[def.id];
  return html`<li class="app-row">
    <div class="app-row-main">
      <${AppIcon} def=${def} size=${44} />
      <div class="app-row-text">
        <div class="app-row-title">
          <${AppLink} def=${def} />
          <${VersionState} app=${app} />
          ${app.running && html`<${RunningBadge} />`}
          <${DownloadCount} n=${app.downloads} />
        </div>
        <p class="muted">${pick(def.tagline)}</p>
      </div>
      <${AppActions} def=${def} />
    </div>
    ${job?.phase === 'error' && html`<${JobError} def=${def} job=${job} />`}
  </li>`;
}

function Announcement({ item }) {
  return html`<div class="flash flash-accent">
    <${Icon} name="info" />
    <div class="flash-body">
      <strong>${pick(item.title)}</strong>
      ${item.body && html`<div>${pick(item.body)}</div>`}
    </div>
    ${item.url &&
    html`<div class="flash-actions">
      <a class="btn btn-sm" href=${item.url}>${pick(item.linkLabel) || t('toast.show')}<${Icon} name="external" /></a>
    </div>`}
  </div>`;
}

// Bütün uygulamaların sürümleri tek akışta, en yeni üstte.
function RecentReleases() {
  const { data } = useStore();
  const entries = data.catalog.apps
    .flatMap((def) => data.apps[def.id].releases.filter((r) => !r.prerelease).map((release) => ({ def, release })))
    .sort((a, b) => new Date(b.release.publishedAt) - new Date(a.release.publishedAt))
    .slice(0, 6);

  if (!entries.length) return html`<p class="empty muted">${t('app.noReleases')}</p>`;

  return html`<ol class="feed">
    ${entries.map(({ def, release }) => {
      const { markdown } = selectNotes(release.body, {
        lang: lang(),
        from: def.releaseNotes.from,
        languages: def.releaseNotes.languages,
      });
      return html`<li class="feed-item" key=${`${def.id}-${release.id}`}>
        <span class="feed-marker"><${Icon} name="tag" size=${14} /></span>
        <div class="feed-body">
          <div class="feed-title">
            <${AppIcon} def=${def} size=${20} />
            <a
              href="#"
              onClick=${(event) => {
                event.preventDefault();
                navigate({ view: 'app', id: def.id, tab: 'changelog' });
              }}
              >${release.name}</a
            >
            <span class="muted feed-date">${relative(release.publishedAt)}</span>
          </div>
          <p class="feed-excerpt muted">${excerpt(markdown)}</p>
        </div>
      </li>`;
    })}
  </ol>`;
}

export function Home() {
  const { data } = useStore();
  const { catalog } = data;
  return html`<div class="page">
    ${catalog.announcements.map((item) => html`<${Announcement} item=${item} key=${item.id} />`)}
    ${catalog.bundles.map((bundle) => html`<${BundleHero} bundle=${bundle} key=${bundle.id} />`)}

    <section class="section">
      <h2 class="section-title">${t('home.apps')}</h2>
      <ul class="box">
        ${catalog.apps.map((def) => html`<${AppRow} def=${def} key=${def.id} />`)}
      </ul>
    </section>

    <section class="section">
      <h2 class="section-title">${t('home.recent')}</h2>
      <${RecentReleases} />
    </section>
  </div>`;
}
