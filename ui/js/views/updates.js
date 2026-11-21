// Güncellemeler: bekleyen güncellemeler (kurulu sürümden sonraki bütün notlarla) ve güncel uygulamalar.
import { html } from '../../vendor/preact-htm.js';
import { Icon } from '../icons.js';
import { lang, t } from '../i18n.js';
import { selectNotes } from '../notes.js';
import { bytes, relative } from '../format.js';
import { navigate, useStore } from '../store.js';
import * as actions from '../actions.js';
import {
  AppActions,
  AppIcon,
  AppLink,
  JobError,
  JobProgress,
  Label,
  Markdown,
  ReleaseNotes,
  Spinner,
  VersionDiff,
  VersionTag,
  compareVersions,
} from '../components.js';

const HUB_ID = 'robz-hub';

function CheckStatus() {
  const { data } = useStore();
  const { check } = data;
  let problem = null;
  if (check.problem === 'offline') problem = t('updates.offline');
  else if (check.problem === 'limited') problem = t('updates.limited', { time: relative(check.retryAt) });
  else if (check.problem === 'error') problem = t('updates.error');

  return html`<header class="page-head">
      <div>
        <h1>${t('updates.title')}</h1>
        <p class="muted">
          ${check.checking
            ? t('updates.checking')
            : check.lastChecked
              ? t('updates.lastChecked', { time: relative(check.lastChecked) })
              : t('updates.never')}
        </p>
      </div>
      <button class="btn" disabled=${check.checking} onClick=${actions.checkNow}>
        ${check.checking ? html`<${Spinner} />` : html`<${Icon} name="sync" />`}${t('action.check')}
      </button>
    </header>
    ${problem && html`<div class="flash flash-attention"><${Icon} name="alert" /><div class="flash-body">${problem}</div></div>`}`;
}

function HubUpdate() {
  const { data } = useStore();
  if (!data.self.updateAvailable) return null;
  const job = data.jobs[HUB_ID];
  const busy = job && job.phase !== 'error';
  const def = { id: HUB_ID, name: 'Robz Hub' };
  return html`<section class="update-card update-card-hub">
    <header class="update-head">
      <img class="app-icon" src="/assets/hub/icon.png" width="44" height="44" alt="" />
      <div class="update-title">
        <h2>${t('updates.hub.title', { version: data.self.latest.version })}</h2>
        <p class="muted">${t('updates.hub.body')}</p>
      </div>
      ${!busy &&
      html`<button class="btn btn-primary" onClick=${() => actions.run('updateSelf')}>
        <${Icon} name="download" />${t('updates.hub.action')}
      </button>`}
    </header>
    ${busy && html`<${JobProgress} def=${def} job=${job} />`}
    ${job?.phase === 'error' && html`<${JobError} def=${def} job=${job} />`}
    ${data.self.notes &&
    html`<div class="update-notes">
      <${Markdown} source=${selectNotes(data.self.notes, { lang: lang() }).markdown} repo=${data.catalog.hub?.repo} />
    </div>`}
  </section>`;
}

function UpdateCard({ def }) {
  const { data } = useStore();
  const app = data.apps[def.id];
  const job = data.jobs[def.id];
  const busy = job && job.phase !== 'error';
  // Kurulu sürümden yeni, kararlı sürümler: en yenisi açık, aradakiler kapalı gösterilir.
  const newer = app.releases.filter(
    (r) => !r.prerelease && compareVersions(r.version, app.install.version) > 0 && compareVersions(r.version, app.latest.version) <= 0,
  );
  const [latest, ...between] = newer;

  return html`<section class="update-card">
    <header class="update-head">
      <${AppIcon} def=${def} size=${44} />
      <div class="update-title">
        <h2><${AppLink} def=${def} /></h2>
        <div class="update-meta">
          <${VersionDiff} from=${app.install.version} to=${app.latest.version} />
          <span class="muted">${bytes(app.latest.size)}</span>
        </div>
      </div>
      ${!busy &&
      html`<button class="btn btn-primary" onClick=${() => actions.install(def)}>
        <${Icon} name="download" />${t('action.update')}
      </button>`}
    </header>
    ${busy && html`<${JobProgress} def=${def} job=${job} />`}
    ${job?.phase === 'error' && html`<${JobError} def=${def} job=${job} />`}
    ${latest &&
    html`<div class="update-notes">
      <h3>${latest.name}</h3>
      <${ReleaseNotes} def=${def} release=${latest} />
      ${between.length > 0 &&
      html`<details class="between">
        <summary><${Icon} name="chevron-right" />${t('updates.olderReleases', { n: between.length })}</summary>
        ${between.map(
          (release) => html`<div class="between-item" key=${release.id}>
            <h3>${release.name}</h3>
            <${ReleaseNotes} def=${def} release=${release} collapse=${false} />
          </div>`,
        )}
      </details>`}
    </div>`}
  </section>`;
}

export function Updates() {
  const { data } = useStore();
  const installed = data.catalog.apps.filter((def) => data.apps[def.id].install.installed);
  const pending = installed.filter((def) => data.apps[def.id].updateAvailable);
  const current = installed.filter((def) => !data.apps[def.id].updateAvailable);
  const idle = pending.filter((def) => !data.jobs[def.id] || data.jobs[def.id].phase === 'error');

  return html`<div class="page">
    <${CheckStatus} />
    <${HubUpdate} />

    ${installed.length === 0 &&
    html`<div class="blank">
      <${Icon} name="package" size=${32} />
      <h2>${t('updates.noneInstalled.title')}</h2>
      <p class="muted">${t('updates.noneInstalled.body')}</p>
      <button class="btn btn-primary" onClick=${() => navigate({ view: 'home' })}>${t('updates.goHome')}</button>
    </div>`}

    ${pending.length > 0 &&
    html`<section class="section">
      <div class="section-head">
        <h2 class="section-title">${t('updates.pending')} <span class="counter">${pending.length}</span></h2>
        ${idle.length > 1 &&
        html`<button class="btn btn-sm" onClick=${() => idle.forEach((def) => actions.install(def))}>
          ${t('action.updateAll')}
        </button>`}
      </div>
      ${pending.map((def) => html`<${UpdateCard} def=${def} key=${def.id} />`)}
    </section>`}

    ${installed.length > 0 &&
    pending.length === 0 &&
    html`<div class="flash flash-success"><${Icon} name="check" /><div class="flash-body">${t('updates.allCurrent')}</div></div>`}

    ${current.length > 0 &&
    html`<section class="section">
      <h2 class="section-title">${t('updates.current')}</h2>
      <ul class="box">
        ${current.map((def) => {
          const app = data.apps[def.id];
          return html`<li class="app-row" key=${def.id}>
            <div class="app-row-main">
              <${AppIcon} def=${def} size=${36} />
              <div class="app-row-text">
                <div class="app-row-title">
                  <${AppLink} def=${def} />
                  ${app.install.version
                    ? html`<${VersionTag} version=${app.install.version} />`
                    : html`<${Label}>${t('status.versionUnknown')}<//>`}
                </div>
                ${app.latest && html`<p class="muted">${t('home.released', { date: relative(app.latest.publishedAt) })}</p>`}
              </div>
              <${AppActions} def=${def} />
            </div>
            ${data.jobs[def.id]?.phase === 'error' && html`<${JobError} def=${def} job=${data.jobs[def.id]} />`}
          </li>`;
        })}
      </ul>
    </section>`}
  </div>`;
}
