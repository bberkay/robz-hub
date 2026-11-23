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

  // ---- dil menüsü: dışarı tıklayınca ya da Esc ile kapanır ----
  const langMenu = document.querySelector('[data-lang-menu]');
  if (langMenu) {
    document.addEventListener('click', (event) => {
      if (langMenu.open && !langMenu.contains(event.target)) langMenu.open = false;
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && langMenu.open) {
        langMenu.open = false;
        langMenu.querySelector('summary').focus();
      }
    });
  }

  // ---- indirme: ziyaretçinin sistemine göre tek düğme ----
  // Windows ve tanınmayan sistemlerde Windows kurulumu; Linux'ta Ubuntu/Debian için .deb, Fedora/SUSE için .rpm,
  // diğer dağıtımlar (Arch, Manjaro…) için AppImage. Öteki biçimler düğmenin altında kalır.
  const ua = navigator.userAgent || '';
  const platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';
  const linux = /linux|x11/i.test(`${platform} ${ua}`) && !/android|cros/i.test(ua);
  const linuxKind = /ubuntu|debian|mint/i.test(ua) ? 'deb' : /fedora|suse|red hat/i.test(ua) ? 'rpm' : 'appimage';
  let downloadKind = 'windows';
  if (linux) {
    downloadKind = linuxKind;
    document.querySelectorAll('[data-download]').forEach((button) => {
      button.href = button.dataset[`${linuxKind}Href`];
      button.querySelector('[data-download-label]').textContent = button.dataset.linuxLabel;
      button.querySelector('[data-download-size]').textContent = button.dataset[`${linuxKind}Size`] || '';
    });
    document.querySelectorAll('[data-other-formats]').forEach((row) => (row.hidden = true));
    document.querySelectorAll('[data-other-formats-linux]').forEach((row) => {
      row.hidden = false;
      // Öne çıkan biçim altta tekrar edilmez.
      row.querySelectorAll(`[data-other="${linuxKind}"]`).forEach((a) => {
        const prev = a.previousElementSibling;
        if (prev && prev.getAttribute('aria-hidden') === 'true') prev.remove();
        else if (a.nextElementSibling && a.nextElementSibling.getAttribute('aria-hidden') === 'true') a.nextElementSibling.remove();
        a.remove();
      });
    });
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
      const names = {
        windows: 'RobzHub-Setup.exe',
        deb: 'RobzHub-Linux-amd64.deb',
        rpm: 'RobzHub-Linux-x86_64.rpm',
        appimage: 'RobzHub-Linux-amd64.AppImage',
      };
      const file = release.assets.find((a) => a.name === names[downloadKind]);
      if (file) set('[data-download-size]', megabytes(file.size));
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
