// Sayfadaki canlı bilgiler (son sürüm, indirme boyutu, yıldız sayısı) GitHub'dan güncellenir; diff görününce oynatılır.
// GitHub'a ulaşılamazsa sayfa derleme anındaki bilgilerle kalır.
(() => {
  const REPO = 'kutsalkiremitci/robz-hub';
  const lang = document.documentElement.lang === 'tr' ? 'tr-TR' : 'en-US';
  document.documentElement.classList.add('js');

  // ---- diff: görünür olunca bir kez oynar ----
  const diff = document.querySelector('.diff');
  if (diff && 'IntersectionObserver' in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          diff.classList.add('play');
          observer.disconnect();
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(diff);
  } else if (diff) {
    diff.classList.add('play');
  }

  // ---- GitHub ----
  const cacheKey = 'robzhub:gh:v1';
  async function api(path) {
    let cache = {};
    try {
      cache = JSON.parse(sessionStorage.getItem(cacheKey) || '{}');
    } catch {}
    if (cache[path] && Date.now() - cache[path].t < 10 * 60 * 1000) return cache[path].v;
    const res = await fetch(`https://api.github.com/${path}`, { headers: { Accept: 'application/vnd.github+json' } });
    if (!res.ok) throw new Error(String(res.status));
    const value = await res.json();
    try {
      cache[path] = { t: Date.now(), v: value };
      sessionStorage.setItem(cacheKey, JSON.stringify(cache));
    } catch {}
    return value;
  }
  const number = (n) => new Intl.NumberFormat(lang).format(n);
  const megabytes = (bytes) => `${new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }).format(bytes / 1048576)} MB`;
  const set = (selector, text) =>
    document.querySelectorAll(selector).forEach((node) => {
      node.textContent = text;
      node.hidden = false;
    });

  api(`repos/${REPO}`)
    .then((repo) => set('[data-stars]', number(repo.stargazers_count)))
    .catch(() => {});

  api(`repos/${REPO}/releases/latest`)
    .then((release) => {
      const version = release.tag_name.startsWith('v') ? release.tag_name : `v${release.tag_name}`;
      set('[data-hub-version]', version);
      document.querySelectorAll('[data-release-link]').forEach((a) => (a.href = release.html_url));
      const setup = release.assets.find((a) => a.name === 'RobzHub-Setup.exe');
      if (setup) set('[data-hub-size]', megabytes(setup.size));
    })
    .catch(() => {});

  document.querySelectorAll('[data-app-repo]').forEach((node) => {
    api(`repos/${node.dataset.appRepo}/releases/latest`)
      .then((release) => {
        node.querySelector('[data-version]').textContent = release.tag_name.replace(/^v/, '');
        node.hidden = false;
      })
      .catch(() => {});
  });
})();
