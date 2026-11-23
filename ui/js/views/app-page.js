// Uygulama sayfası: başlık ve işlem düğmeleri, altında "Genel bakış" ve "Sürüm notları" sekmeleri.
import { html } from '../../vendor/preact-htm.js';
import { Icon } from '../icons.js';
import { pick, t } from '../i18n.js';
import { bytes, count, dateLong, imageUrl } from '../format.js';
import { navigate, update, useStore } from '../store.js';
import * as actions from '../actions.js';
import {
  AppActions,
  AppIcon,
  AppLink,
  DownloadCount,
  StarCount,
  JobError,
  JobProgress,
  Label,
  ReleaseTimeline,
  RunningBadge,
  VersionDiff,
  VersionState,
  VersionTag,
} from '../components.js';

function Screenshots({ shots }) {
  if (!shots.length) return null;
  const open = (index) => update({ lightbox: { images: shots, index } });
  return html`<section class="block">
    <h2 class="block-title">${t('app.screenshots')}</h2>
    <div class="shots">
      ${shots.map(
        (shot, index) => html`<button class="shot" onClick=${() => open(index)} title=${pick(shot.caption)}>
          <img src=${imageUrl(shot.url)} alt=${pick(shot.caption)} loading="lazy" draggable=${false} />
        </button>`,
      )}
    </div>
  </section>`;
}

function About({ def, app }) {
  const links = [
    def.links.website && { icon: 'globe', label: t('action.website'), url: def.links.website },
    def.links.guide && { icon: 'book', label: t('action.guide'), url: def.links.guide },
    { icon: 'github', label: t('action.repo'), url: `https://github.com/${def.repo}` },
    { icon: 'issue', label: t('action.reportIssue'), url: `https://github.com/${def.repo}/issues` },
  ].filter(Boolean);

  return html`<aside class="about">
    <h2 class="block-title">${t('app.about')}</h2>
    <dl class="facts">
      ${app.install.installed &&
      html`<div>
        <dt>${t('app.installedVersion')}</dt>
        <dd>
          ${app.install.version
            ? html`<${VersionTag} version=${app.install.version} />`
            : html`<span class="muted">${t('status.versionUnknown')}</span>`}
        </dd>
      </div>`}
      ${app.latest &&
      html`<div>
          <dt>${t('app.latestVersion')}</dt>
          <dd>
            <${VersionTag} version=${app.latest.version} />
            <span class="muted fact-note">${dateLong(app.latest.publishedAt)}</span>
          </dd>
        </div>
        ${app.supported &&
        html`<div>
          <dt>${t('app.downloadSize')}</dt>
          <dd>
            ${bytes(app.latest.size)}
            ${app.latest.verified &&
            html`<span class="fact-verified" title=${t('app.verifiedHint')}><${Icon} name="shield" size=${14} /></span>`}
          </dd>
        </div>`}`}
      ${app.downloads > 0 &&
      html`<div>
        <dt>${t('app.totalDownloads')}</dt>
        <dd title=${t('stat.downloadsTitle')}>${count(app.downloads)}</dd>
      </div>`}
      ${def.license &&
      html`<div>
        <dt>${t('app.license')}</dt>
        <dd>${def.license}</dd>
      </div>`}
      ${app.install.installed &&
      html`<div class="fact-wide">
        <dt>${t('app.location')}</dt>
        <dd>
          <button class="path mono" title=${t('action.openFolder')} onClick=${() => actions.run('openFolder', def.id)}>
            <${Icon} name="folder" size=${14} /><span>${app.install.dir}</span>
          </button>
        </dd>
      </div>`}
    </dl>

    <ul class="link-list">
      ${links.map(
        (link) => html`<li>
          <a href=${link.url}><${Icon} name=${link.icon} />${link.label}</a>
        </li>`,
      )}
    </ul>

    ${def.requirements.length > 0 &&
    html`<h2 class="block-title">${t('app.requirements')}</h2>
      <ul class="requirements">
        ${def.requirements.map((item) => html`<li>${pick(item)}</li>`)}
      </ul>`}
  </aside>`;
}

// Aynı pakette yer alan diğer uygulamalar.
function WorksWith({ def }) {
  const { data } = useStore();
  const partners = data.catalog.bundles
    .filter((bundle) => bundle.apps.includes(def.id))
    .flatMap((bundle) => bundle.apps)
    .filter((id, index, all) => id !== def.id && all.indexOf(id) === index)
    .map((id) => data.catalog.apps.find((a) => a.id === id));
  if (!partners.length) return null;
  return html`<section class="block">
    <h2 class="block-title">${t('app.worksWith')}</h2>
    <ul class="box">
      ${partners.map(
        (partner) => html`<li class="app-row" key=${partner.id}>
          <div class="app-row-main">
            <${AppIcon} def=${partner} size=${36} />
            <div class="app-row-text">
              <div class="app-row-title">
                <${AppLink} def=${partner} />
                <${VersionState} app=${data.apps[partner.id]} />
              </div>
              <p class="muted">${pick(partner.tagline)}</p>
            </div>
            <${AppActions} def=${partner} />
          </div>
        </li>`,
      )}
    </ul>
  </section>`;
}

function Overview({ def, app }) {
  return html`<div class="overview">
    <div class="overview-main">
      ${def.notice && html`<div class="flash"><${Icon} name="info" /><div class="flash-body">${pick(def.notice)}</div></div>`}
      <p class="lead selectable">${pick(def.description)}</p>

      ${def.highlights.length > 0 &&
      html`<section class="block">
        <h2 class="block-title">${t('app.highlights')}</h2>
        <dl class="highlights">
          ${def.highlights.map(
            (item) => html`<div>
              <dt>${pick(item.title)}</dt>
              <dd class="muted">${pick(item.text)}</dd>
            </div>`,
          )}
        </dl>
      </section>`}

      <${Screenshots} shots=${def.screenshots} />
      <${WorksWith} def=${def} />
    </div>
    <${About} def=${def} app=${app} />
  </div>`;
}

export function AppPage({ id, tab = 'overview' }) {
  const { data } = useStore();
  const def = data.catalog.apps.find((a) => a.id === id);
  if (!def) {
    navigate({ view: 'home' });
    return null;
  }
  const app = data.apps[id];
  const job = data.jobs[id];
  const busy = job && job.phase !== 'error';
  const setTab = (next) => update({ route: { view: 'app', id, tab: next } });

  return html`<div class="page">
    <header class="app-head">
      <${AppIcon} def=${def} size=${72} />
      <div class="app-head-text">
        <h1>${def.name}</h1>
        <p class="muted">${pick(def.tagline)}</p>
        <div class="app-head-state">
          ${app.install.installed && app.updateAvailable
            ? html`<${VersionDiff} from=${app.install.version} to=${app.latest.version} />`
            : app.install.installed
              ? html`<${VersionState} app=${app} /><${Label} kind="success">${t('status.upToDate')}<//>`
              : html`<${Label}>${t('status.notInstalled')}<//>
                  ${app.latest && html`<${VersionTag} version=${app.latest.version} />`}`}
          ${app.running && html`<${RunningBadge} />`}
          ${def.discovered && html`<span title=${t('app.discoveredTitle')}><${Label}>${t('app.discovered')}<//></span>`}
        </div>
        <div class="app-stats">
          <${DownloadCount} n=${app.downloads} />
          <${StarCount} n=${app.stars} />
        </div>
      </div>
      <${AppActions} def=${def} large />
    </header>

    ${busy && html`<${JobProgress} def=${def} job=${job} />`}
    ${job?.phase === 'error' && html`<${JobError} def=${def} job=${job} />`}

    <nav class="tabs" role="tablist">
      <button class="tab" role="tab" aria-selected=${tab === 'overview'} onClick=${() => setTab('overview')}>
        ${t('app.tab.overview')}
      </button>
      <button class="tab" role="tab" aria-selected=${tab === 'changelog'} onClick=${() => setTab('changelog')}>
        ${t('app.tab.changelog')}
        ${app.releases.length > 0 && html`<span class="counter">${app.releases.length}</span>`}
      </button>
    </nav>

    ${tab === 'changelog'
      ? app.releases.length
        ? html`<${ReleaseTimeline} def=${def} app=${app} releases=${app.releases} />`
        : html`<p class="empty muted">${t('app.noReleases')}</p>`
      : html`<${Overview} def=${def} app=${app} />`}
  </div>`;
}
