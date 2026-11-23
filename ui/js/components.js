// Sayfaların paylaştığı parçalar: uygulama simgesi, sürüm etiketleri, işlem düğmeleri, ilerleme, menü, sürüm notları.
import { html, useEffect, useLayoutEffect, useRef, useState } from '../vendor/preact-htm.js';
import { Icon } from './icons.js';
import { lang, pick, t } from './i18n.js';
import { bytes, count, dateLong, imageUrl, percent, remaining, speed } from './format.js';
import { renderMarkdown } from './markdown.js';
import { selectNotes } from './notes.js';
import { navigate, useStore } from './store.js';
import * as actions from './actions.js';

// Simge yoksa ya da yüklenemezse (ör. kendiliğinden bulunan bir uygulamanın deposunda yoksa) adın baş harfleri.
export function AppIcon({ def, size = 40 }) {
  const src = imageUrl(def.icon);
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  if (!src || failed) {
    const letters = def.name
      .split(/\s+/)
      .filter(Boolean)
      .map((w) => w[0].toLocaleUpperCase())
      .slice(0, 2)
      .join('');
    return html`<span class="app-icon app-icon-empty" style=${{ width: size, height: size, fontSize: Math.round(size * 0.34) }} aria-hidden="true"
      >${letters}</span
    >`;
  }
  return html`<img
    class="app-icon"
    src=${src}
    width=${size}
    height=${size}
    alt=""
    draggable=${false}
    onError=${() => setFailed(true)}
  />`;
}

// Sürüm numarası, git etiketi gibi.
export function VersionTag({ version }) {
  return html`<span class="vtag"><${Icon} name="tag" size=${12} />${version}</span>`;
}

// Güncelleme, bir diff gibi: eski sürüm kırmızı satır, yeni sürüm yeşil satır.
export function VersionDiff({ from, to }) {
  return html`<span class="vdiff" aria-label=${`${from} → ${to}`}>
    <del>${from}</del><ins>${to}</ins>
  </span>`;
}

export function Label({ kind = '', children }) {
  return html`<span class=${`label ${kind ? `label-${kind}` : ''}`}>${children}</span>`;
}

// Uygulamanın o anki sürüm durumu: kurulu değil / sürüm / eski → yeni.
export function VersionState({ app }) {
  if (!app.install.installed) return html`<${Label}>${t('status.notInstalled')}<//>`;
  if (app.updateAvailable) return html`<${VersionDiff} from=${app.install.version} to=${app.latest.version} />`;
  if (!app.install.version) return html`<${Label}>${t('status.versionUnknown')}<//>`;
  return html`<${VersionTag} version=${app.install.version} />`;
}

// GitHub'daki indirme sayısı: "↓ 1.234 indirme"
export function DownloadCount({ n }) {
  if (!n) return null;
  return html`<span class="dl-count" title=${t('stat.downloadsTitle')}>
    <${Icon} name="download" size=${12} />${t('stat.downloads', { n: count(n), raw: n })}
  </span>`;
}

// Uygulama bu sistemde kurulamıyorsa nedeniyle: "Windows'ta desteklenmiyor", "Linux'ta desteklenmiyor" ya da
// "Bu dağıtımda desteklenmiyor" (Linux sürümü var ama bu dağıtımın kurabileceği biçimde değil).
export function unsupportedKey(app, platform) {
  if (app.unsupported === 'distro') return 'status.unsupported.distro';
  return platform === 'windows' ? 'status.unsupported.windows' : 'status.unsupported.linux';
}

export function Unsupported({ app, large = false }) {
  const { data } = useStore();
  const key = unsupportedKey(app, data.platform);
  return html`<span class=${`unsupported ${large ? 'unsupported-lg' : ''}`} title=${t(`${key}.title`)}>
    <${Icon} name="info" size=${large ? 16 : 14} />${t(key)}
  </span>`;
}

// Rust'ın düzenli ifadesi JavaScript'e: "(?i)" baştaki büyük/küçük harf bayrağına çevrilir. Çevrilemezse null.
export function assetRegExp(pattern) {
  if (!pattern) return null;
  try {
    return pattern.startsWith('(?i)') ? new RegExp(pattern.slice(4), 'i') : new RegExp(pattern);
  } catch {
    return null;
  }
}

// GitHub yıldızı: sarı yıldız ve sayı.
export function StarCount({ n }) {
  if (!n) return null;
  return html`<span class="star-count" title=${t('stat.starsTitle')}>
    <${Icon} name="star" size=${12} class="icon-star" />${t('stat.stars', { n: count(n), raw: n })}
  </span>`;
}

// ---- dil seçimi: bayraklı açılır kutu -----------------------------------------------------------------------------

// Türk bayrağının yıldızı: merkezi (16.1, 10), bir ucu sola (hilale) bakar.
const STAR = Array.from({ length: 10 }, (_, k) => {
  const angle = Math.PI + (k * Math.PI) / 5;
  const radius = k % 2 ? 0.96 : 2.5;
  return `${(16.1 + radius * Math.cos(angle)).toFixed(2)},${(10 + radius * Math.sin(angle)).toFixed(2)}`;
}).join(' ');

// Windows bayrak emojilerini bayrak olarak çizmez ("TR", "GB" harfleri görünür); bayraklar bu yüzden SVG.
export function Flag({ code, size = 20 }) {
  const height = Math.round((size * 2) / 3);
  if (code === 'tr') {
    return html`<svg class="flag" width=${size} height=${height} viewBox="0 0 30 20" aria-hidden="true">
      <rect width="30" height="20" fill="#E30A17" />
      <circle cx="10.5" cy="10" r="5" fill="#fff" />
      <circle cx="11.75" cy="10" r="4" fill="#E30A17" />
      <polygon points=${STAR} fill="#fff" />
    </svg>`;
  }
  if (code === 'en') {
    return html`<svg class="flag" width=${size} height=${height} viewBox="0 0 45 30" aria-hidden="true">
      <rect width="45" height="30" fill="#012169" />
      <path d="M0,0 L45,30 M45,0 L0,30" stroke="#fff" stroke-width="6" />
      <path d="M0,0 L45,30 M45,0 L0,30" stroke="#C8102E" stroke-width="2.2" />
      <path d="M22.5,0 V30 M0,15 H45" stroke="#fff" stroke-width="9" />
      <path d="M22.5,0 V30 M0,15 H45" stroke="#C8102E" stroke-width="5.4" />
    </svg>`;
  }
  return html`<span class="flag flag-auto" style=${{ width: size, height }}><${Icon} name="globe" size=${height} /></span>`;
}

// Seçim kutusu: düğmede seçili dilin bayrağı ve adı; açılan listede bir seçeneğe tıklamak dili hemen değiştirir.
// options: [{ value, label, flag }]. up: liste yukarı açılır (kenar çubuğunun altında).
export function LanguageSelect({ value, options, onChange, label, up = false }) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const ref = useRef(null);
  const current = options.find((o) => o.value === value) || options[0];

  useEffect(() => {
    if (!open) return undefined;
    setActive(Math.max(0, options.findIndex((o) => o.value === value)));
    const onDown = (event) => {
      if (!ref.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);

  const choose = (option) => {
    setOpen(false);
    if (option.value !== value) onChange(option.value);
  };
  const onKeyDown = (event) => {
    if (!open && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
      event.preventDefault();
      setOpen(true);
    } else if (open && event.key === 'ArrowDown') {
      event.preventDefault();
      setActive((i) => (i + 1) % options.length);
    } else if (open && event.key === 'ArrowUp') {
      event.preventDefault();
      setActive((i) => (i - 1 + options.length) % options.length);
    } else if (open && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault();
      choose(options[active]);
    } else if (event.key === 'Escape') {
      setOpen(false);
    }
  };

  return html`<div class=${`lang-select ${up ? 'lang-select-up' : ''}`} ref=${ref}>
    <button
      class="lang-select-button"
      aria-haspopup="listbox"
      aria-expanded=${open}
      aria-label=${`${label}: ${current.label}`}
      onClick=${() => setOpen(!open)}
      onKeyDown=${onKeyDown}
    >
      <${Flag} code=${current.flag} />
      <span>${current.label}</span>
      <${Icon} name="chevron-down" size=${14} class="lang-select-chevron" />
    </button>
    ${open &&
    html`<ul class="lang-select-list" role="listbox" aria-label=${label}>
      ${options.map(
        (option, i) => html`<li
          role="option"
          aria-selected=${option.value === value}
          class=${`lang-select-option ${i === active ? 'is-active' : ''}`}
          onMouseEnter=${() => setActive(i)}
          onClick=${() => choose(option)}
        >
          <${Flag} code=${option.flag} />
          <span>${option.label}</span>
          ${option.value === value && html`<${Icon} name="check" size=${14} class="lang-select-check" />`}
        </li>`,
      )}
    </ul>`}
  </div>`;
}

export function RunningBadge() {
  return html`<span class="running"><span class="running-dot"></span>${t('status.running')}</span>`;
}

// ---- menü ---------------------------------------------------------------------------------------------------------

export function Menu({ label, items }) {
  const [open, setOpen] = useState(false);
  const ref = useRef(null);
  useEffect(() => {
    if (!open) return undefined;
    const onDown = (event) => {
      if (!ref.current?.contains(event.target)) setOpen(false);
    };
    const onKey = (event) => {
      if (event.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  return html`<div class="menu" ref=${ref}>
    <button
      class="btn btn-icon"
      aria-haspopup="menu"
      aria-expanded=${open}
      aria-label=${label}
      title=${label}
      onClick=${() => setOpen(!open)}
    >
      <${Icon} name="kebab" />
    </button>
    ${open &&
    html`<div class="menu-list" role="menu">
      ${items.map((item) =>
        item === 'divider'
          ? html`<div class="menu-divider" role="separator"></div>`
          : html`<button
              class=${`menu-item ${item.danger ? 'menu-item-danger' : ''}`}
              role="menuitem"
              onClick=${() => {
                setOpen(false);
                item.onSelect();
              }}
            >
              <${Icon} name=${item.icon} />${item.label}
            </button>`,
      )}
    </div>`}
  </div>`;
}

function appMenuItems(def, app) {
  const items = [];
  if (app.install.installed) {
    items.push({ icon: 'folder', label: t('action.openFolder'), onSelect: () => actions.run('openFolder', def.id) });
    if (app.latest) items.push({ icon: 'repair', label: t('action.reinstall'), onSelect: () => actions.install(def) });
    items.push('divider');
  }
  if (def.links.website) {
    items.push({ icon: 'globe', label: t('action.website'), onSelect: () => actions.openExternal(def.links.website) });
  }
  items.push({
    icon: 'github',
    label: t('action.repo'),
    onSelect: () => actions.openExternal(`https://github.com/${def.repo}`),
  });
  items.push({
    icon: 'issue',
    label: t('action.reportIssue'),
    onSelect: () => actions.openExternal(`https://github.com/${def.repo}/issues`),
  });
  if (app.install.installed) {
    items.push('divider');
    items.push({ icon: 'trash', label: t('action.uninstall'), danger: true, onSelect: () => actions.uninstall(def) });
  }
  return items;
}

// ---- iş ilerlemesi ------------------------------------------------------------------------------------------------

function jobLabel(def, job) {
  if (job.phase === 'queued') return t('job.queued');
  if (job.phase === 'downloading') return t('job.downloading');
  if (job.phase === 'closing') return t('job.closing', { name: def.name });
  if (job.phase === 'uninstalling') return t('job.uninstalling');
  return t('job.installing');
}

export function JobProgress({ def, job, compact = false }) {
  const { progress } = useStore();
  const live = progress[def.id];
  const downloading = job.phase === 'downloading';
  // Sıradaki iş için çubuk boş durur; kapatma, kurma ve kaldırmada süresi belirsiz olduğu için hareket eder.
  const working = job.phase === 'closing' || job.phase === 'installing' || job.phase === 'uninstalling';
  const received = downloading ? live?.received || 0 : 0;
  const total = live?.total || job.total || 0;
  const pct = percent(received, total);
  const cancellable = job.phase === 'queued' || downloading;
  const detail = downloading
    ? [`${bytes(received)} / ${bytes(total)}`, speed(live?.speed), remaining(received, total, live?.speed)].filter(Boolean)
    : [];

  return html`<div class=${`job ${compact ? 'job-compact' : ''}`} role="status">
    <div class="job-line">
      <span class="job-label">
        ${jobLabel(def, job)}${downloading ? html` <span class="job-pct mono">${pct}%</span>` : ''}
      </span>
      ${!compact && html`<span class="job-detail mono">${detail.map((part) => html`<span>${part}</span>`)}</span>`}
      ${cancellable &&
      html`<button class="btn btn-sm btn-invisible job-cancel" onClick=${() => actions.cancel(def.id)}>
        ${t('action.cancel')}
      </button>`}
    </div>
    <div
      class=${`progress ${working ? 'progress-indeterminate' : ''}`}
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=${downloading ? pct : working ? undefined : 0}
    >
      <span class="progress-fill" style=${{ width: downloading ? `${pct}%` : working ? '' : '0%' }}></span>
    </div>
    ${!compact &&
    job.verified &&
    html`<div class="job-verified"><${Icon} name="shield" size=${14} />${t('job.verified')}</div>`}
  </div>`;
}

export function JobError({ def, job }) {
  const message = actions.errorText(job.error);
  const closeAndRetry = job.error?.code === 'app-running';
  return html`<div class="flash flash-danger" role="alert">
    <${Icon} name="alert" />
    <div class="flash-body">
      <div>${message}</div>
      ${job.error?.detail &&
      job.error.code !== 'app-running' &&
      html`<div class="flash-detail mono selectable">${t('error.detail', { detail: job.error.detail })}</div>`}
    </div>
    <div class="flash-actions">
      <button class="btn btn-sm" onClick=${() => actions.retry(def, job)}>
        ${closeAndRetry ? t('action.closeAndRetry') : t('action.retry')}
      </button>
      <button class="btn btn-sm btn-invisible" onClick=${() => actions.dismissJob(def.id)}>${t('action.dismiss')}</button>
    </div>
  </div>`;
}

// ---- işlem düğmeleri ----------------------------------------------------------------------------------------------

// Bir uygulama için o an anlamlı olan düğmeler. large: uygulama sayfasındaki büyük düğmeler.
export function AppActions({ def, large = false }) {
  const { data } = useStore();
  const app = data.apps[def.id];
  const job = data.jobs[def.id];
  const size = large ? 'btn-lg' : '';
  const busy = job && job.phase !== 'error';

  if (busy && !large) return html`<div class="actions"><${JobProgress} def=${def} job=${job} compact /></div>`;

  // Bu sistemde kurulamayan uygulama (Linux'ta paketi olmayan): düğme yerine kısa bir not.
  if (!app.supported && !app.install.installed) {
    return html`<div class="actions">
      <${Unsupported} app=${app} />
      <${Menu} label=${t('action.more')} items=${appMenuItems(def, app)} />
    </div>`;
  }

  const buttons = [];
  if (!busy) {
    if (!app.install.installed) {
      buttons.push(html`<button
        class=${`btn btn-primary ${size}`}
        disabled=${!app.latest}
        title=${app.latest ? '' : t('app.noReleases')}
        onClick=${() => actions.install(def)}
      >
        <${Icon} name="download" />${t('action.install')}
        ${large && app.latest && html`<span class="btn-meta">${bytes(app.latest.size)}</span>`}
      </button>`);
    } else {
      if (app.updateAvailable) {
        buttons.push(html`<button class=${`btn btn-primary ${size}`} onClick=${() => actions.install(def)}>
          <${Icon} name="download" />${t('action.update')}
          ${large && html`<span class="btn-meta">${bytes(app.latest.size)}</span>`}
        </button>`);
      }
      if (app.running) {
        buttons.push(html`<button class=${`btn ${size}`} onClick=${() => actions.stop(def)}>
          <${Icon} name="stop" />${t('action.stop')}
        </button>`);
      } else {
        buttons.push(html`<button
          class=${`btn ${app.updateAvailable ? '' : 'btn-primary'} ${size}`}
          onClick=${() => actions.launch(def)}
        >
          <${Icon} name="play" />${t('action.launch')}
        </button>`);
      }
    }
  }

  return html`<div class="actions">
    ${buttons}
    <${Menu} label=${t('action.more')} items=${appMenuItems(def, app)} />
  </div>`;
}

// ---- sürüm notları ------------------------------------------------------------------------------------------------

export function Markdown({ source, repo }) {
  if (!source.trim()) return html`<p class="muted">${t('app.emptyNotes')}</p>`;
  return html`<div class="markdown selectable" dangerouslySetInnerHTML=${{ __html: renderMarkdown(source, repo) }}></div>`;
}

// Uzun içeriği kısaltır; "Tümünü göster" ile açılır.
function Collapsible({ maxHeight = 340, children }) {
  const ref = useRef(null);
  const [overflow, setOverflow] = useState(false);
  const [open, setOpen] = useState(false);
  useLayoutEffect(() => {
    if (ref.current) setOverflow(ref.current.scrollHeight > maxHeight + 80);
  }, [children]);
  const clipped = overflow && !open;
  return html`<div class=${`collapsible ${clipped ? 'collapsible-clipped' : ''}`}>
    <div ref=${ref} style=${{ maxHeight: clipped ? `${maxHeight}px` : '' }} class="collapsible-inner">${children}</div>
    ${overflow &&
    html`<button class="btn btn-sm btn-invisible collapsible-toggle" onClick=${() => setOpen(!open)}>
      <${Icon} name=${open ? 'chevron-down' : 'chevron-right'} />
      ${open ? t('action.showLess') : t('action.showAll')}
    </button>`}
  </div>`;
}

export function ReleaseNotes({ def, release, collapse = true }) {
  const { markdown, language } = selectNotes(release.body, {
    lang: lang(),
    from: def.releaseNotes.from,
    languages: def.releaseNotes.languages,
  });
  const otherLanguage = language && language !== lang();
  const body = html`<${Markdown} source=${markdown} repo=${def.repo} />`;
  return html`<div class="notes" lang=${language || undefined}>
    ${otherLanguage && html`<p class="notes-language muted"><${Icon} name="info" size=${14} />${t(`app.notesOnly.${language}`)}</p>`}
    ${collapse ? html`<${Collapsible}>${body}<//>` : body}
  </div>`;
}

// Sürüm geçmişi: solda tarih ve etiket, sağda notlar (GitHub'ın sürümler sayfası düzeni).
export function ReleaseTimeline({ def, app, releases }) {
  // Bu sistemde kullanılan kurulum dosyası (boyutu ve indirme sayısı için); kurulamıyorsa yok.
  const pattern = assetRegExp(app.assetPattern) || { test: () => false };
  return html`<ol class="timeline">
    ${releases.map((release) => {
      const asset = release.assets.find((a) => pattern.test(a.name));
      const isLatest = app.latest?.tag === release.tag;
      const isInstalled = app.install.installed && app.install.version === release.version;
      const isNew =
        app.install.installed && app.install.version && compareVersions(release.version, app.install.version) > 0;
      return html`<li class="release" key=${release.id}>
        <div class="release-meta">
          <div class="release-date">${dateLong(release.publishedAt)}</div>
          <${VersionTag} version=${release.version} />
          ${asset && html`<div class="release-size muted">${bytes(asset.size)}</div>`}
          <${DownloadCount} n=${release.assets.reduce((sum, a) => sum + (a.downloads || 0), 0)} />
        </div>
        <article class="release-card">
          <header class="release-head">
            <h3>${release.name}</h3>
            <div class="release-labels">
              ${isLatest && html`<${Label} kind="success">${t('status.latest')}<//>`}
              ${isInstalled && html`<${Label} kind="accent">${t('status.installed')}<//>`}
              ${isNew && !release.prerelease && html`<${Label} kind="attention">${t('status.new')}<//>`}
              ${release.prerelease && html`<${Label} kind="attention">${t('status.prerelease')}<//>`}
            </div>
            <a class="release-link" href=${release.url} title=${t('action.viewOnGithub')}>
              <${Icon} name="external" /><span class="sr-only">${t('action.viewOnGithub')}</span>
            </a>
          </header>
          <${ReleaseNotes} def=${def} release=${release} />
        </article>
      </li>`;
    })}
  </ol>`;
}

// "1.10.0" > "1.9.2" (ana süreçteki version.js'in arayüzdeki karşılığı; yalnız sayılar).
export function compareVersions(a, b) {
  const pa = String(a).split(/[.-]/).map((n) => parseInt(n, 10) || 0);
  const pb = String(b).split(/[.-]/).map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d) return d < 0 ? -1 : 1;
  }
  return 0;
}

// ---- küçük parçalar -----------------------------------------------------------------------------------------------

export function AppLink({ def, tab, children }) {
  return html`<a
    class="app-link"
    href="#"
    onClick=${(event) => {
      event.preventDefault();
      navigate({ view: 'app', id: def.id, tab: tab || 'overview' });
    }}
    >${children || def.name}</a
  >`;
}

export function Toggle({ checked, onChange, label, disabled = false }) {
  return html`<button
    class="toggle"
    role="switch"
    aria-checked=${checked}
    aria-label=${label}
    disabled=${disabled}
    onClick=${() => onChange(!checked)}
  >
    <span class="toggle-knob"></span>
  </button>`;
}

export function Spinner({ size = 16 }) {
  return html`<span class="spinner" style=${{ width: size, height: size }} aria-hidden="true"></span>`;
}

export { pick };
