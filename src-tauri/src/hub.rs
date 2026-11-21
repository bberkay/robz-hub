//! Robz Hub'ın çekirdeği: katalog, sürümler, kurulum durumu ve işler (kur / güncelle / kaldır) tek yerde tutulur.
//!
//! Arayüze giden olaylar:
//!   "hub:state"    tüm durum (her değişiklikte)
//!   "hub:progress" { appId, received, total, speed }  indirme ilerlemesi (sık gelir, durumu yeniden göndermez)
//!   "hub:notice"   { type: "installed" | "updated" | "uninstalled" | "updates-found", … }  kullanıcıya gösterilecek sonuç
//!
//! İşler tek bir kuyrukta sırayla çalışır: iki kurulum programı aynı anda açılmaz, indirmeler birbirini yavaşlatmaz.

use crate::catalog::{self, AppDef, Catalog};
use crate::download::{self, Progress};
use crate::error::{HubError, HubResult};
use crate::github::{self, now_ms, Asset, FetchStatus, Release};
use crate::installs::{self, InstallInfo};
use crate::settings::Settings;
use crate::{shell, version};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub const HUB_ID: &str = "robz-hub";
const CATALOG_MAX_AGE: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    pub version: String,
    pub tag: String,
    pub published_at: String,
    pub size: u64,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub install: InstallInfo,
    pub running: bool,
    pub releases: Vec<Release>,
    pub latest: Option<Latest>,
    pub update_available: bool,
    pub fetch_status: Option<&'static str>,
    /// Bütün sürümlerin bütün dosyalarının toplam indirme sayısı (GitHub'daki "toplam indirme" rozetiyle aynı hesap).
    pub downloads: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobState {
    pub kind: &'static str,
    pub phase: &'static str,
    pub version: Option<String>,
    pub received: u64,
    pub total: u64,
    pub speed: f64,
    pub verified: bool,
    pub error: Option<HubError>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfLatest {
    pub version: String,
    pub published_at: String,
    pub size: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfState {
    pub latest: Option<SelfLatest>,
    pub update_available: bool,
    pub notes: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckState {
    pub checking: bool,
    pub last_checked: Option<u64>,
    pub problem: Option<&'static str>,
    pub retry_at: Option<u64>,
}

pub struct Paths {
    pub catalog_cache: PathBuf,
    pub releases: PathBuf,
    pub downloads: PathBuf,
    pub settings: PathBuf,
}

impl Paths {
    pub fn new(data: PathBuf) -> Self {
        Self {
            catalog_cache: data.join("catalog-cache.json"),
            releases: data.join("releases"),
            downloads: data.join("downloads"),
            settings: data.join("settings.json"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Install,
    Uninstall,
}

pub struct QueuedJob {
    id: String,
    kind: JobKind,
    close_if_running: bool,
    cancel: Arc<AtomicBool>,
}

struct Inner {
    catalog: Catalog,
    apps: BTreeMap<String, AppState>,
    jobs: BTreeMap<String, JobState>,
    cancels: HashMap<String, Arc<AtomicBool>>,
    self_state: SelfState,
    self_pick: Option<(Release, Asset)>,
    check: CheckState,
    settings: Settings,
    cache_bytes: u64,
    catalog_fetched: Option<Instant>,
    installs_refreshed: Option<Instant>,
}

pub struct Hub {
    pub app: AppHandle,
    pub paths: Paths,
    pub dry_run: bool,
    pub version: String,
    pub system_language: &'static str,
    client: reqwest::Client,
    inner: Mutex<Inner>,
    queue: mpsc::UnboundedSender<QueuedJob>,
    refresh_lock: tokio::sync::Mutex<()>,
    progress: Mutex<HashMap<String, Progress>>,
}

fn derive(def: &AppDef, app: &mut AppState) {
    app.latest = github::pick_latest(&app.releases, &def.windows.asset_re).map(|(r, a)| Latest {
        version: r.version.clone(),
        tag: r.tag.clone(),
        published_at: r.published_at.clone(),
        size: a.size,
        verified: a.sha256.is_some(),
    });
    app.update_available = app.install.installed
        && matches!((&app.latest, &app.install.version), (Some(l), Some(v)) if version::compare(&l.version, v).is_gt());
    app.downloads = app.releases.iter().flat_map(|r| &r.assets).map(|a| a.downloads).sum();
}

/// Kurulu kopyanın exe yolundan çalışan bir süreç var mı (running_paths sonucuna göre)?
fn is_running_in(running: &HashMap<String, Vec<u32>>, info: &InstallInfo) -> bool {
    info.installed && info.exe_path.as_deref().is_some_and(|exe| running.contains_key(&installs::norm_path(exe)))
}

fn tag_dir(tag: &str) -> String {
    tag.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect()
}

impl Hub {
    pub fn new(app: AppHandle, data_dir: PathBuf, dry_run: bool) -> HubResult<(Arc<Self>, mpsc::UnboundedReceiver<QueuedJob>)> {
        let version = app.package_info().version.to_string();
        let paths = Paths::new(data_dir);
        let client = reqwest::Client::builder()
            .user_agent(format!("RobzHub/{version}"))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| HubError::with("unknown", e))?;
        let settings = Settings::load(&paths.settings);
        let (catalog, _source) = catalog::load_local(&paths.catalog_cache);
        // Robz Hub'ın kendi güncellemesi için indirilen kurulum dosyası: Robz Hub yeniden açıldıysa işi bitmiştir.
        let _ = std::fs::remove_dir_all(paths.downloads.join(HUB_ID));
        let (queue, rx) = mpsc::unbounded_channel();

        let hub = Arc::new(Self {
            app,
            dry_run,
            version,
            system_language: shell::system_language(),
            client,
            inner: Mutex::new(Inner {
                catalog: catalog.clone(),
                apps: BTreeMap::new(),
                jobs: BTreeMap::new(),
                cancels: HashMap::new(),
                self_state: SelfState::default(),
                self_pick: None,
                check: CheckState::default(),
                settings,
                cache_bytes: download::dir_size(&paths.downloads),
                catalog_fetched: None,
                installs_refreshed: None,
            }),
            paths,
            queue,
            refresh_lock: tokio::sync::Mutex::new(()),
            progress: Mutex::new(HashMap::new()),
        });
        {
            let mut inner = hub.lock();
            hub.set_catalog(&mut inner, catalog);
        }
        Ok((hub, rx))
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // ---- durum ------------------------------------------------------------------------------------------------------

    pub fn snapshot(&self) -> Value {
        let inner = self.lock();
        json!({
            "hubVersion": self.version,
            "dryRun": self.dry_run,
            "packaged": !cfg!(debug_assertions),
            "systemLanguage": self.system_language,
            "catalog": inner.catalog,
            "apps": inner.apps,
            "jobs": inner.jobs,
            "self": inner.self_state,
            "check": inner.check,
            "settings": inner.settings.public(),
            "cacheBytes": inner.cache_bytes,
        })
    }

    pub fn changed(&self) {
        let _ = self.app.emit("hub:state", self.snapshot());
        shell::state_changed(self);
    }

    fn notice(&self, notice: Value) {
        let _ = self.app.emit("hub:notice", notice);
    }

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    /// Kurulu uygulamalar (kimlik, ad), katalog sırasıyla.
    pub fn installed_apps(&self) -> Vec<(String, String)> {
        let inner = self.lock();
        inner
            .catalog
            .apps
            .iter()
            .filter(|d| inner.apps.get(&d.id).is_some_and(|a| a.install.installed))
            .map(|d| (d.id.clone(), d.name.clone()))
            .collect()
    }

    pub fn language(&self) -> &'static str {
        match self.lock().settings.language.as_str() {
            "tr" => "tr",
            "en" => "en",
            _ => self.system_language,
        }
    }

    fn set_catalog(&self, inner: &mut Inner, catalog: Catalog) {
        let mut apps = BTreeMap::new();
        for def in &catalog.apps {
            let mut state = inner.apps.remove(&def.id).unwrap_or_else(|| AppState {
                install: InstallInfo::default(),
                running: false,
                releases: github::cached_releases(&self.paths.releases, &def.repo),
                latest: None,
                update_available: false,
                fetch_status: None,
                downloads: 0,
            });
            derive(def, &mut state);
            apps.insert(def.id.clone(), state);
        }
        inner.apps = apps;
        inner.catalog = catalog;
    }

    /// Kurulum durumlarını yeniden okur (kayıt defteri, klasörler, çalışan süreçler).
    pub fn refresh_installs(&self) {
        let defs = self.lock().catalog.apps.clone();
        let running = installs::running_paths(&defs.iter().map(|d| d.windows.exe.clone()).collect::<Vec<_>>());
        let detected: Vec<InstallInfo> = defs.iter().map(installs::detect).collect();
        {
            let mut inner = self.lock();
            inner.installs_refreshed = Some(Instant::now());
            for (def, info) in defs.iter().zip(detected) {
                if let Some(app) = inner.apps.get_mut(&def.id) {
                    app.running = is_running_in(&running, &info);
                    app.install = info;
                    derive(def, app);
                }
            }
        }
        self.changed();
    }

    /// Yalnız "çalışıyor mu" bilgisini yeniler; değişiklik varsa haber verir.
    pub fn poll_running(&self) {
        let names: Vec<String> = self.lock().catalog.apps.iter().map(|d| d.windows.exe.clone()).collect();
        let running = installs::running_paths(&names);
        let changed = {
            let mut inner = self.lock();
            let Inner { catalog, apps, .. } = &mut *inner;
            let mut changed = false;
            for def in &catalog.apps {
                if let Some(app) = apps.get_mut(&def.id) {
                    let now = is_running_in(&running, &app.install);
                    changed |= now != app.running;
                    app.running = now;
                }
            }
            changed
        };
        if changed {
            self.changed();
        }
    }

    /// Pencere öne gelince: uygulamalar Robz Hub dışında kurulmuş, güncellenmiş ya da kaldırılmış olabilir.
    pub fn on_focus(self: &Arc<Self>) {
        let (stale, check_due) = {
            let inner = self.lock();
            let stale = inner.installs_refreshed.is_none_or(|t| t.elapsed() > Duration::from_secs(10));
            let check_due = inner.settings.auto_check
                && inner.check.last_checked.is_none_or(|t| now_ms().saturating_sub(t) > 30 * 60 * 1000);
            (stale, check_due)
        };
        if stale {
            self.refresh_installs();
        }
        if check_due {
            let hub = self.clone();
            tauri::async_runtime::spawn(async move { hub.refresh(false).await });
        }
    }

    // ---- GitHub denetimi --------------------------------------------------------------------------------------------

    /// Katalog ve sürümleri GitHub'dan yeniler. Aynı anda tek denetim çalışır; ikinci çağrı ilkinin bitmesini bekler.
    pub async fn refresh(self: &Arc<Self>, manual: bool) {
        let _guard = match self.refresh_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                let _ = self.refresh_lock.lock().await;
                return;
            }
        };

        // Oran sınırındayken GitHub yine reddeder; süre dolana kadar istek gönderilmez.
        let limited = {
            let inner = self.lock();
            inner.check.problem == Some("limited") && inner.check.retry_at.is_some_and(|t| t > now_ms())
        };
        if limited {
            self.refresh_installs();
            return;
        }

        let (known, fetch_catalog) = {
            let mut inner = self.lock();
            inner.check.checking = true;
            let known: HashMap<String, Option<String>> =
                inner.apps.iter().map(|(id, a)| (id.clone(), a.latest.as_ref().map(|l| l.version.clone()))).collect();
            (known, manual || inner.catalog_fetched.is_none_or(|t| t.elapsed() > CATALOG_MAX_AGE))
        };
        self.changed();

        if fetch_catalog {
            let remote = catalog::fetch_remote(&self.client, &self.paths.catalog_cache).await;
            let mut inner = self.lock();
            inner.catalog_fetched = Some(Instant::now());
            if let Some(remote) = remote.filter(|r| r.revision >= inner.catalog.revision) {
                self.set_catalog(&mut inner, remote);
            }
        }

        let repos: Vec<(String, String)> =
            self.lock().catalog.apps.iter().map(|d| (d.id.clone(), d.repo.clone())).collect();
        let results = futures_util::future::join_all(
            repos.iter().map(|(_, repo)| github::fetch_releases(&self.client, repo, &self.paths.releases)),
        )
        .await;

        let (mut problem, mut retry_at, mut reached) = (None, None::<u64>, false);
        {
            let mut inner = self.lock();
            for ((id, _), result) in repos.iter().zip(results) {
                let Some(app) = inner.apps.get_mut(id) else { continue };
                app.fetch_status = Some(result.status.as_str());
                match result.status {
                    FetchStatus::Ok | FetchStatus::Cached => reached = true,
                    FetchStatus::Limited => {
                        problem = Some("limited");
                        retry_at = Some(retry_at.unwrap_or(0).max(result.retry_at.unwrap_or(0)));
                    }
                    FetchStatus::Offline if problem != Some("limited") => problem = Some("offline"),
                    FetchStatus::Error if problem.is_none() => problem = Some("error"),
                    _ => {}
                }
                if matches!(result.status, FetchStatus::Ok | FetchStatus::Cached | FetchStatus::Missing)
                    || !result.releases.is_empty()
                {
                    app.releases = result.releases;
                }
            }
        }

        self.refresh_self().await;
        self.refresh_installs();

        let fresh: Vec<Value> = {
            let mut inner = self.lock();
            inner.check = CheckState {
                checking: false,
                last_checked: if reached { Some(now_ms()) } else { inner.check.last_checked },
                problem,
                retry_at,
            };
            let Inner { catalog, apps, .. } = &*inner;
            catalog
                .apps
                .iter()
                .filter_map(|def| {
                    let app = apps.get(&def.id)?;
                    let latest = app.latest.as_ref()?;
                    (app.update_available && known.get(&def.id).cloned().flatten().as_ref() != Some(&latest.version))
                        .then(|| json!({ "id": def.id, "name": def.name, "version": latest.version }))
                })
                .collect()
        };
        self.changed();

        if !fresh.is_empty() {
            shell::updates_found(self, &fresh);
            self.notice(json!({ "type": "updates-found", "apps": fresh }));
        }
    }

    /// Robz Hub'ın kendi yeni sürümü var mı? Depo henüz yoksa ya da sürüm yayınlanmadıysa sessizce geçilir.
    async fn refresh_self(&self) {
        let Some(hub) = self.lock().catalog.hub.clone() else { return };
        let result = github::fetch_releases(&self.client, &hub.repo, &self.paths.releases).await;
        let pick = github::pick_latest(&result.releases, &hub.asset_re).map(|(r, a)| (r.clone(), a.clone()));
        let mut inner = self.lock();
        inner.self_state = match &pick {
            Some((release, asset)) => SelfState {
                latest: Some(SelfLatest {
                    version: release.version.clone(),
                    published_at: release.published_at.clone(),
                    size: asset.size,
                }),
                update_available: version::compare(&release.version, &self.version).is_gt(),
                notes: Some(release.body.clone()),
                url: Some(release.url.clone()),
            },
            None => SelfState::default(),
        };
        inner.self_pick = pick;
    }

    // ---- işler ------------------------------------------------------------------------------------------------------

    fn update_job(&self, id: &str, change: impl FnOnce(&mut JobState)) {
        if let Some(job) = self.lock().jobs.get_mut(id) {
            change(job);
        }
        self.changed();
    }

    pub fn clear_job(&self, id: &str) {
        {
            let mut inner = self.lock();
            inner.jobs.remove(id);
            inner.cancels.remove(id);
        }
        self.progress.lock().unwrap_or_else(|p| p.into_inner()).remove(id);
        self.changed();
    }

    fn busy(inner: &Inner, id: &str) -> bool {
        inner.jobs.get(id).is_some_and(|job| job.phase != "error")
    }

    pub fn any_busy(&self) -> bool {
        self.lock().jobs.values().any(|job| job.phase != "error")
    }

    pub fn enqueue(&self, id: &str, kind: JobKind, close_if_running: bool) -> HubResult<()> {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut inner = self.lock();
            if inner.catalog.app(id).is_none() {
                return Err(HubError::with("unknown-app", id));
            }
            if Self::busy(&inner, id) {
                return Ok(());
            }
            let label = match kind {
                JobKind::Uninstall => "uninstall",
                JobKind::Install => match inner.apps.get(id) {
                    Some(app) if !app.install.installed => "install",
                    Some(app) if app.update_available => "update",
                    _ => "reinstall",
                },
            };
            inner.cancels.insert(id.to_string(), cancel.clone());
            inner.jobs.insert(id.to_string(), JobState { kind: label, phase: "queued", ..JobState::default() });
        }
        let _ = self.queue.send(QueuedJob { id: id.to_string(), kind, close_if_running, cancel });
        self.changed();
        Ok(())
    }

    pub fn install_bundle(&self, bundle_id: &str) -> HubResult<()> {
        let members = {
            let inner = self.lock();
            let bundle = inner.catalog.bundle(bundle_id).ok_or_else(|| HubError::with("unknown-app", bundle_id))?;
            bundle
                .apps
                .iter()
                .filter(|id| inner.apps.get(*id).is_some_and(|a| !a.install.installed || a.update_available))
                .cloned()
                .collect::<Vec<_>>()
        };
        for id in members {
            self.enqueue(&id, JobKind::Install, true)?;
        }
        Ok(())
    }

    /// İndirme sürerken ya da sıradayken iptal edilebilir; kurulum programı başladıktan sonra edilemez.
    pub fn cancel(&self, id: &str) {
        let inner = self.lock();
        if inner.jobs.get(id).is_some_and(|j| j.phase == "queued" || j.phase == "downloading") {
            if let Some(flag) = inner.cancels.get(id) {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn dismiss_job(&self, id: &str) {
        let is_error = self.lock().jobs.get(id).is_some_and(|j| j.phase == "error");
        if is_error {
            self.clear_job(id);
        }
    }

    /// Kuyruktaki işleri sırayla çalıştırır (uygulama açıkken hep çalışır).
    pub async fn worker(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<QueuedJob>) {
        while let Some(job) = rx.recv().await {
            if job.cancel.load(Ordering::Relaxed) {
                self.clear_job(&job.id);
                continue;
            }
            let result = match job.kind {
                JobKind::Install => self.run_install(&job).await,
                JobKind::Uninstall => self.run_uninstall(&job).await,
            };
            match result {
                Ok(notice) => {
                    self.clear_job(&job.id);
                    self.notice(notice);
                }
                Err(error) if error.is("cancelled") => self.clear_job(&job.id),
                Err(error) => self.update_job(&job.id, |j| {
                    j.phase = "error";
                    j.error = Some(error);
                }),
            }
            self.refresh_installs();
            let size = download::dir_size(&self.paths.downloads);
            self.lock().cache_bytes = size;
            self.changed();
        }
    }

    fn on_progress(&self, id: &str, progress: Progress) {
        self.progress.lock().unwrap_or_else(|p| p.into_inner()).insert(id.to_string(), progress);
        let _ = self.app.emit(
            "hub:progress",
            json!({ "appId": id, "received": progress.received, "total": progress.total, "speed": progress.speed }),
        );
        shell::update_taskbar(self);
    }

    /// Görev çubuğu için: indirmelerin toplam yüzdesi; kurulum sürüyorsa belirsiz; iş yoksa None.
    pub fn taskbar_progress(&self) -> Option<Option<u64>> {
        let inner = self.lock();
        let progress = self.progress.lock().unwrap_or_else(|p| p.into_inner());
        let (mut received, mut total) = (0u64, 0u64);
        for (id, job) in &inner.jobs {
            if job.phase == "downloading" {
                if let Some(p) = progress.get(id) {
                    received += p.received;
                    total += p.total;
                }
            }
        }
        if total > 0 {
            return Some(Some((received * 100 / total).min(100)));
        }
        let working = inner.jobs.values().any(|j| matches!(j.phase, "installing" | "uninstalling" | "closing"));
        working.then_some(None)
    }

    /// Kurulu kopya açıksa: izin varsa kapatır, yoksa "app-running" hatası. Başka yerden çalışan aynı adlı kopyalara
    /// (ör. geliştirme derlemesi) dokunulmaz.
    async fn close_if_needed(&self, job: &QueuedJob, info: &InstallInfo) -> HubResult<()> {
        if !installs::is_running(info) {
            return Ok(());
        }
        if !job.close_if_running {
            return Err(HubError::new("app-running"));
        }
        self.update_job(&job.id, |j| j.phase = "closing");
        if !self.dry_run && !installs::close_app(info).await {
            return Err(HubError::new("close-failed"));
        }
        Ok(())
    }

    /// Kur, güncelle ya da yeniden kur: hepsi en son sürümün kurulum dosyasını indirip sessizce çalıştırır.
    async fn run_install(&self, job: &QueuedJob) -> HubResult<Value> {
        let (def, pick, keep, kind) = {
            let inner = self.lock();
            let def = inner.catalog.app(&job.id).cloned().ok_or_else(|| HubError::new("unknown-app"))?;
            let pick = inner
                .apps
                .get(&job.id)
                .and_then(|a| github::pick_latest(&a.releases, &def.windows.asset_re))
                .map(|(r, a)| (r.clone(), a.clone()));
            let kind = inner.jobs.get(&job.id).map(|j| j.kind).unwrap_or("install");
            (def, pick, inner.settings.keep_installers, kind)
        };
        let (release, asset) = pick.ok_or_else(|| HubError::new("no-release"))?;
        self.update_job(&job.id, |j| {
            j.phase = "downloading";
            j.version = Some(release.version.clone());
            j.total = asset.size;
        });

        let dir = self.paths.downloads.join(&def.id).join(tag_dir(&release.tag));
        let result = self.download_and_install(job, &def, &asset, &dir).await;
        // İptalde ve başarılı kurulumda dosya silinir; hata olursa kalır (yeniden denemede tekrar inmez).
        if result.as_ref().map_or_else(|e| e.is("cancelled"), |_| !keep) {
            let _ = tokio::fs::remove_dir_all(&dir).await;
            if let Some(parent) = dir.parent() {
                let _ = tokio::fs::remove_dir(parent).await; // yalnız boşsa silinir
            }
        }
        result?;
        Ok(json!({
            "type": if kind == "install" { "installed" } else { "updated" },
            "appId": def.id,
            "name": def.name,
            "version": release.version,
        }))
    }

    async fn download_and_install(&self, job: &QueuedJob, def: &AppDef, asset: &Asset, dir: &std::path::Path) -> HubResult<()> {
        let id = job.id.as_str();
        let (file, verified) =
            download::download_asset(&self.client, asset, &def.repo, dir, &job.cancel, |p| self.on_progress(id, p)).await?;
        self.progress.lock().unwrap_or_else(|p| p.into_inner()).remove(id);
        self.update_job(id, |j| j.verified = verified);

        self.close_if_needed(job, &installs::detect(def)).await?;
        self.update_job(id, |j| j.phase = "installing");
        if self.dry_run {
            tokio::time::sleep(Duration::from_millis(2500)).await;
            return Ok(());
        }
        installs::run_installer(def, &file).await?;
        for _ in 0..20 {
            if installs::detect(def).installed {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(HubError::with("installer-failed", "kurulumdan sonra uygulama bulunamadı"))
    }

    async fn run_uninstall(&self, job: &QueuedJob) -> HubResult<Value> {
        let def = self.lock().catalog.app(&job.id).cloned().ok_or_else(|| HubError::new("unknown-app"))?;
        let info = installs::detect(&def);
        if !info.installed {
            return Err(HubError::new("not-installed"));
        }
        self.close_if_needed(job, &info).await?;
        self.update_job(&job.id, |j| {
            j.phase = "uninstalling";
            j.version = info.version.clone();
        });
        if self.dry_run {
            tokio::time::sleep(Duration::from_millis(2000)).await;
        } else {
            installs::run_uninstaller(&def, &info).await?;
        }
        Ok(json!({ "type": "uninstalled", "appId": def.id, "name": def.name }))
    }

    // ---- uygulamayı başlat / kapat ----------------------------------------------------------------------------------

    pub fn launch(self: &Arc<Self>, id: &str) -> HubResult<()> {
        let def = self.lock().catalog.app(id).cloned().ok_or_else(|| HubError::with("unknown-app", id))?;
        installs::launch(&installs::detect(&def))?;
        // Pencere açılana kadar birkaç kez bak: "Çalışıyor" hemen görünsün.
        let hub = self.clone();
        tauri::async_runtime::spawn(async move {
            for delay in [800, 1200, 2000] {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                hub.poll_running();
            }
        });
        Ok(())
    }

    pub fn launch_bundle(self: &Arc<Self>, bundle_id: &str) -> HubResult<()> {
        let members: Vec<String> = {
            let inner = self.lock();
            let bundle = inner.catalog.bundle(bundle_id).ok_or_else(|| HubError::with("unknown-app", bundle_id))?;
            bundle
                .apps
                .iter()
                .filter(|id| inner.apps.get(*id).is_some_and(|a| a.install.installed && !a.running))
                .cloned()
                .collect()
        };
        for id in members {
            self.launch(&id)?;
        }
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> HubResult<()> {
        let def = self.lock().catalog.app(id).cloned().ok_or_else(|| HubError::with("unknown-app", id))?;
        let closed = installs::close_app(&installs::detect(&def)).await;
        self.poll_running();
        if closed {
            Ok(())
        } else {
            Err(HubError::new("close-failed"))
        }
    }

    pub fn install_dir(&self, id: &str) -> Option<String> {
        self.lock().apps.get(id).and_then(|a| a.install.dir.clone())
    }

    // ---- ayarlar ve önbellek ----------------------------------------------------------------------------------------

    pub fn set_setting(&self, key: &str, value: &Value) -> bool {
        let ok = {
            let mut inner = self.lock();
            let ok = inner.settings.set(key, value);
            if ok {
                inner.settings.save(&self.paths.settings);
            }
            ok
        };
        if ok {
            self.changed();
        }
        ok
    }

    /// Aynı sürüm için bir kez bildirim: daha önce bildirilmemiş olanları döndürür ve kaydeder.
    pub fn take_unnotified(&self, apps: &[Value]) -> Vec<Value> {
        let mut inner = self.lock();
        let fresh: Vec<Value> = apps
            .iter()
            .filter(|a| {
                let (Some(id), Some(version)) = (a["id"].as_str(), a["version"].as_str()) else { return false };
                inner.settings.notified.get(id).map(String::as_str) != Some(version)
            })
            .cloned()
            .collect();
        for a in &fresh {
            if let (Some(id), Some(version)) = (a["id"].as_str(), a["version"].as_str()) {
                inner.settings.notified.insert(id.to_string(), version.to_string());
            }
        }
        if !fresh.is_empty() {
            inner.settings.save(&self.paths.settings);
        }
        fresh
    }

    pub async fn clear_cache(&self) {
        if self.any_busy() {
            return;
        }
        let _ = tokio::fs::remove_dir_all(&self.paths.downloads).await;
        self.lock().cache_bytes = 0;
        self.changed();
    }

    // ---- Robz Hub'ın kendi güncellemesi -----------------------------------------------------------------------------

    /// Yeni kurulum dosyası indirilip doğrulanır; dönüş değeri dosya yoludur (çalıştırmak main.rs'in işidir).
    pub async fn download_self_update(&self) -> HubResult<Option<PathBuf>> {
        let cancel = Arc::new(AtomicBool::new(false));
        let (hub, release, asset) = {
            let mut inner = self.lock();
            let hub = inner.catalog.hub.clone().ok_or_else(|| HubError::new("no-release"))?;
            let (release, asset) = inner.self_pick.clone().ok_or_else(|| HubError::new("no-release"))?;
            if !inner.self_state.update_available {
                return Err(HubError::new("no-release"));
            }
            if Self::busy(&inner, HUB_ID) {
                return Ok(None);
            }
            inner.cancels.insert(HUB_ID.to_string(), cancel.clone());
            inner.jobs.insert(
                HUB_ID.to_string(),
                JobState {
                    kind: "update",
                    phase: "downloading",
                    version: Some(release.version.clone()),
                    total: asset.size,
                    ..JobState::default()
                },
            );
            (hub, release, asset)
        };
        self.changed();

        let dir = self.paths.downloads.join(HUB_ID).join(tag_dir(&release.tag));
        match download::download_asset(&self.client, &asset, &hub.repo, &dir, &cancel, |p| self.on_progress(HUB_ID, p)).await {
            Ok((file, _)) => {
                self.update_job(HUB_ID, |j| j.phase = "installing");
                Ok(Some(file))
            }
            Err(error) if error.is("cancelled") => {
                self.clear_job(HUB_ID);
                Ok(None)
            }
            Err(error) => {
                self.update_job(HUB_ID, |j| {
                    j.phase = "error";
                    j.error = Some(error);
                });
                Ok(None)
            }
        }
    }
}
