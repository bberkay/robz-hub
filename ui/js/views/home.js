// Ana sayfa (Vitrin): üstte sırayla dönen tanıtım alanı (paketler ve duyurular, catalog.json'dan), altında kapak
// görselli uygulama kartları, en altta son sürümler ve topluluk. Tanıtım alanında ne gösterileceği catalog.json'dan
// değiştirilir; yeni bir Robz Hub sürümü gerekmez.
import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { Icon } from '../icons.js';
import { lang, pick, t } from '../i18n.js';
import { bytes, imageUrl, relative } from '../format.js';
import { excerpt, selectNotes } from '../notes.js';
import { navigate, useStore, store } from '../store.js';
import * as actions from '../actions.js';
import { AppActions, AppIcon, AppLink, DownloadCount, StarCount, Unsupported, unsupportedKey, JobError, RunningBadge, Spinner, VersionState } from '../components.js';

const SLIDE_MS = 8000;
const reducedMotion = () => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

// Görsel yüklenemezse (internet yok gibi) gizlenir; alan koyu zeminle kalır.
function Backdrop({ src }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  if (!src || failed) return null;
  return html`<img class="promo-image" src=${imageUrl(src)} alt="" draggable=${false} onError=${() => setFailed(true)} />`;
}

// ---- tanıtım alanı: paket --------------------------------------------------------------------------------------------

function memberStatus({ def, app, job }) {
  if (job && job.phase !== 'error') return { tone: 'busy', text: def.name };
  if (!app.supported && !app.install.installed) return { tone: 'muted', text: `${def.name}: ${t(unsupportedKey(app, store.data.platform))}` };
  if (!app.install.installed) return { tone: 'muted', text: t('bundle.status.missing', { name: def.name }) };
  if (app.running) return { tone: 'success', text: t('bundle.status.running', { name: def.name }) };
  if (app.updateAvailable) {
    return { tone: 'accent', text: t('bundle.status.update', { name: def.name, version: app.latest.version }) };
  }
  return { tone: 'muted', text: t('bundle.status.ready', { name: def.name }) };
}

function BundleSlide({ bundle }) {
  const { data } = useStore();
  const members = bundle.apps
    .map((id) => ({ def: data.catalog.apps.find((a) => a.id === id), app: data.apps[id], job: data.jobs[id] }))
    .filter((m) => m.def && m.app);
  const n = members.length;
  const missing = members.filter((m) => !m.app.install.installed);
  const outdated = members.filter((m) => m.app.install.installed && m.app.updateAvailable);
  const busy = members.some((m) => m.job && m.job.phase !== 'error');
  const allRunning = members.every((m) => m.app.running);
  const pending = [...missing, ...outdated];
  const downloadSize = pending.reduce((sum, m) => sum + (m.app.latest?.size || 0), 0);
  const installable = pending.length > 0 && pending.every((m) => m.app.latest);
  const names = members.map((m) => m.def.name);

  const unsupported = members.find((m) => !m.app.supported && !m.app.install.installed);

  let cta;
  if (unsupported) {
    cta = html`<${Unsupported} app=${unsupported.app} large />`;
  } else if (busy) {
    cta = html`<button class="btn btn-primary btn-xl" disabled><${Spinner} />${t('bundle.working')}</button>`;
  } else if (missing.length) {
    cta = html`<button class="btn btn-primary btn-xl" disabled=${!installable} onClick=${() => actions.run('installBundle', bundle.id)}>
        <${Icon} name="download" />
        ${missing.length === n
          ? t('bundle.installAll', { n })
          : outdated.length
            ? t('bundle.installAndUpdate')
            : t('bundle.installMissing', { n: missing.length })}
      </button>
      ${downloadSize > 0 && html`<span class="promo-note">${t('bundle.totalSize', { size: bytes(downloadSize) })}</span>`}`;
  } else if (allRunning) {
    cta = html`<span class="promo-running"><${RunningBadge} /><span>${t('bundle.allRunning', { n })}</span></span>`;
  } else {
    cta = html`<button class="btn btn-primary btn-xl" onClick=${() => actions.run('launchBundle', bundle.id)}>
        <${Icon} name="play" />${t('bundle.launchAll', { n })}
      </button>
      ${outdated.length > 0 &&
      html`<button class="btn btn-xl btn-glass" disabled=${!installable} onClick=${() => actions.run('installBundle', bundle.id)}>
        <${Icon} name="download" />${t('action.update')}
        <span class="btn-meta">${bytes(downloadSize)}</span>
      </button>`}`;
  }

  return html`<div class="promo-body">
    <div class="promo-kicker">
      <span class="promo-icons">${members.map(({ def }) => html`<${AppIcon} def=${def} size=${28} key=${def.id} />`)}</span>
      <span>${t('bundle.together', { names })}</span>
    </div>
    <h1>${pick(bundle.title)}</h1>
    ${bundle.description && html`<p class="promo-text">${pick(bundle.description)}</p>`}
    <div class="promo-cta">${cta}</div>
    <ul class="promo-status">
      ${members.map((member) => {
        const status = memberStatus(member);
        return html`<li class=${`promo-status-${status.tone}`} key=${member.def.id}>
          ${status.tone === 'busy' ? html`<${Spinner} size=${12} />` : html`<span class="promo-dot"></span>`}
          <${AppLink} def=${member.def}>${status.text}<//>
        </li>`;
      })}
    </ul>
  </div>`;
}

// ---- tanıtım alanı: duyuru -------------------------------------------------------------------------------------------

function AnnouncementSlide({ item }) {
  return html`<div class="promo-body">
    ${item.date &&
    html`<div class="promo-kicker"><${Icon} name="info" size=${14} /><span>${new Date(item.date).toLocaleDateString(lang() === 'tr' ? 'tr-TR' : 'en-US', { day: 'numeric', month: 'long' })}</span></div>`}
    <h1>${pick(item.title)}</h1>
    ${item.body && html`<p class="promo-text">${pick(item.body)}</p>`}
    ${item.url &&
    html`<div class="promo-cta">
      <a class="btn btn-primary btn-xl" href=${item.url}>${pick(item.linkLabel) || t('hero.more')}<${Icon} name="external" /></a>
    </div>`}
  </div>`;
}

// Paketler ve duyurular sırayla döner; fare üzerindeyken, odaktayken ve "hareketi azalt" açıksa durur.
function Promo({ slides }) {
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);
  const count = slides.length;
  const current = slides[index % count];

  useEffect(() => {
    if (count < 2 || paused || reducedMotion()) return undefined;
    const timer = setTimeout(() => setIndex((i) => (i + 1) % count), SLIDE_MS);
    return () => clearTimeout(timer);
  }, [index, count, paused]);

  return html`<section
    class="promo"
    aria-roledescription="carousel"
    aria-label=${t('hero.label')}
    onMouseEnter=${() => setPaused(true)}
    onMouseLeave=${() => setPaused(false)}
    onFocusIn=${() => setPaused(true)}
    onFocusOut=${() => setPaused(false)}
  >
    <${Backdrop} src=${current.image} />
    <div class="promo-scrim"></div>
    ${current.kind === 'bundle'
      ? html`<${BundleSlide} bundle=${current.bundle} key=${current.id} />`
      : html`<${AnnouncementSlide} item=${current.item} key=${current.id} />`}
    ${count > 1 &&
    html`<div class="promo-dots">
      ${slides.map(
        (slide, i) => html`<button
          class="promo-dot-button"
          key=${slide.id}
          aria-label=${t('hero.slide', { n: i + 1 })}
          aria-current=${i === index % count ? 'true' : undefined}
          onClick=${() => setIndex(i)}
        ></button>`,
      )}
    </div>`}
  </section>`;
}

// ---- uygulama kartları -----------------------------------------------------------------------------------------------

function AppCard({ def }) {
  const { data } = useStore();
  const app = data.apps[def.id];
  const job = data.jobs[def.id];
  const cover = def.cover || def.screenshots[0]?.url;
  return html`<article class="app-card">
    <div class="app-card-cover">
      ${cover && html`<${Backdrop} src=${cover} />`}
      <span class="app-card-icon"><${AppIcon} def=${def} size=${52} /></span>
    </div>
    <div class="app-card-body">
      <div class="app-card-title">
        <h3><${AppLink} def=${def} /></h3>
        <${VersionState} app=${app} />
        <${DownloadCount} n=${app.downloads} />
        <${StarCount} n=${app.stars} />
      </div>
      <p class="muted">${pick(def.tagline)}</p>
      <div class="app-card-footer">
        ${app.running && html`<${RunningBadge} />`}
        <${AppActions} def=${def} />
      </div>
      ${job?.phase === 'error' && html`<${JobError} def=${def} job=${job} />`}
    </div>
  </article>`;
}

function NextAppCard({ discord }) {
  return html`<article class="app-card app-card-next">
    <span class="app-card-next-icon"><${Icon} name="package" size=${22} /></span>
    <h3>${t('home.next.title')}</h3>
    <p class="muted">${t('home.next.body')}</p>
    ${discord && html`<a class="app-card-next-link" href=${discord}><${Icon} name="discord" size=${14} />${t('home.next.link')}</a>`}
  </article>`;
}

// ---- son sürümler ve topluluk ----------------------------------------------------------------------------------------

// Bütün uygulamaların sürümleri tek akışta, en yeni üstte.
function RecentReleases() {
  const { data } = useStore();
  const entries = data.catalog.apps
    .flatMap((def) => data.apps[def.id].releases.filter((r) => !r.prerelease).map((release) => ({ def, release })))
    .sort((a, b) => new Date(b.release.publishedAt) - new Date(a.release.publishedAt))
    .slice(0, 4);

  if (!entries.length) return html`<p class="muted">${t('app.noReleases')}</p>`;

  return html`<ol class="releases-list">
    ${entries.map(({ def, release }) => {
      const { markdown } = selectNotes(release.body, {
        lang: lang(),
        from: def.releaseNotes.from,
        languages: def.releaseNotes.languages,
      });
      return html`<li key=${`${def.id}-${release.id}`}>
        <${AppIcon} def=${def} size=${28} />
        <div class="releases-list-body">
          <div class="releases-list-title">
            <a
              href="#"
              onClick=${(event) => {
                event.preventDefault();
                navigate({ view: 'app', id: def.id, tab: 'changelog' });
              }}
              >${release.name}</a
            >
            <span class="muted">${relative(release.publishedAt)}</span>
          </div>
          <p class="muted">${excerpt(markdown, 120)}</p>
        </div>
      </li>`;
    })}
  </ol>`;
}

function Community({ links }) {
  return html`<div class="community">
    ${links.discord &&
    html`<div class="community-row community-discord">
      <${Icon} name="discord" size=${28} />
      <div class="community-text">
        <strong>${t('home.discord.title')}</strong>
        <span class="muted">${t('home.discord.body')}</span>
      </div>
      <a class="btn btn-community" href=${links.discord}>${t('home.discord.action')}</a>
    </div>`}
    ${links.kick &&
    html`<div class="community-row community-kick">
      <${Icon} name="kick" size=${28} />
      <div class="community-text">
        <strong>${t('home.kick.title')}</strong>
        <span class="muted">${t('home.kick.body')}</span>
      </div>
      <a class="btn btn-community" href=${links.kick}>${t('home.kick.action')}</a>
    </div>`}
  </div>`;
}

export function Home() {
  const { data } = useStore();
  const { catalog } = data;
  const slides = [
    ...catalog.bundles.map((bundle) => ({ kind: 'bundle', id: `b-${bundle.id}`, image: bundle.image, bundle })),
    ...catalog.announcements.map((item) => ({ kind: 'announcement', id: `a-${item.id}`, image: item.image, item })),
  ];
  const hasCommunity = catalog.links.kick || catalog.links.discord;

  return html`<div class="page page-home">
    ${slides.length > 0 && html`<${Promo} slides=${slides} />`}

    <section class="section">
      <h2 class="section-title">${t('home.apps')}</h2>
      <div class="app-cards">
        ${catalog.apps.map((def) => html`<${AppCard} def=${def} key=${def.id} />`)}
        <${NextAppCard} discord=${catalog.links.discord} />
      </div>
    </section>

    <div class="home-panels">
      <section class="panel">
        <h2 class="section-title">${t('home.recent')}</h2>
        <${RecentReleases} />
      </section>
      ${hasCommunity &&
      html`<section class="panel">
        <h2 class="section-title">${t('home.community')}</h2>
        <${Community} links=${catalog.links} />
      </section>`}
    </div>
  </div>`;
}
