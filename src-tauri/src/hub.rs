//! Robz Hub'ın çekirdeği: katalog, sürümler, kurulum durumu ve işler (kur / güncelle / kaldır) tek yerde tutulur.
//!
//! Arayüze giden olaylar:
//!   "hub:state"    tüm durum (her değişiklikte)
//!   "hub:progress" { appId, received, total, speed }  indirme ilerlemesi (sık gelir, durumu yeniden göndermez)
//!   "hub:notice"   { type: "installed" | "updated" | "uninstalled" | "updates-found", … }  kullanıcıya gösterilecek sonuç
//!
//! İşler tek bir kuyrukta sırayla çalışır: iki kurulum programı aynı anda açılmaz, indirmeler birbirini yavaşlatmaz.

use crate::catalog::{self, AppDef, Catalog};
use crate::discover::{self, ListResult, RepoInfo};
use crate::download::{self, Progress};
use crate::error::{HubError, HubResult};
use crate::github::{self, now_ms, Asset, FetchStatus, Release};
use crate::installs::{self, InstallInfo};
use crate::settings::Settings;
use crate::{log, platform, shell, version};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Deponun GitHub yıldızı (sahibin depo listesinden; henüz okunmadıysa None).
    pub stars: Option<u64>,
    /// Bu sistemde kurulabilir mi.
    pub supported: bool,
    /// Kurulamıyorsa nedeni: "os" (bu işletim sistemi için sürümü yok) ya da "distro" (Linux sürümü var ama bu
    /// dağıtımda kurulamıyor).
    pub unsupported: Option<platform::Unsupported>,
    /// Bu sistemde kullanılan kurulum dosyasının adı için düzenli ifade (sürüm notlarında dosya boyutu için).
    pub asset_pattern: Option<String>,
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
    /// Kurulu Robz Hub'dan yeni kararlı sürümler, en yenisi önce. Güncelleme kartında en yenisinin notu açık, aradakiler
    /// kapalı gösterilir: birkaç sürüm atlayan kullanıcı da neler geldiğini görür.
    pub releases: Vec<Release>,
    pub url: Option<String>,
    /// Robz Hub kendini bu sistemde güncelleyebilir mi; değilse yeni sürüm sayfası açılır.
    pub installable: bool,
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
    pub discovery: PathBuf,
}

impl Paths {
    pub fn new(data: PathBuf) -> Self {
        Self {
            catalog_cache: data.join("catalog-cache.json"),
            releases: data.join("releases"),
            downloads: data.join("downloads"),
            settings: data.join("settings.json"),
            discovery: data.join("discovered.json"),
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
    /// GitHub'da kendiliğinden bulunan, katalogda olmayan uygulamalar (discover.rs).
    found: Vec<AppDef>,
    discovery: discover::State,
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
    /// Arayüze giden durumun sıra numarası; her yayında artar.
    seq: AtomicU64,
    /// Durum sırayla yayınlansın diye: iki iş parçacığı aynı anda yayınlarsa eski durum sonra gidip yenisini ezmesin.
    emit_lock: Mutex<()>,
    /// Aynı anda tek kurulum algılaması çalışsın diye.
    detect_lock: Mutex<()>,
}

fn derive(def: &AppDef, app: &mut AppState) {
    let plan = platform::plan(def, &app.install, &app.releases).map(|p| p.asset);
    derive_with(app, plan);
}

/// `plan`: bu sistemde kurulum dosyasını seçen ifade ya da kurulamama nedeni.
fn derive_with(app: &mut AppState, plan: Result<&regex::Regex, platform::Unsupported>) {
    app.supported = plan.is_ok();
    app.unsupported = plan.err();
    app.asset_pattern = plan.ok().map(|re| re.as_str().to_string());
    app.latest = match plan {
        Ok(asset) => github::pick_latest(&app.releases, asset).map(|(r, a)| Latest {
            version: r.version.clone(),
            tag: r.tag.clone(),
            published_at: r.published_at.clone(),
            size: a.size,
            verified: a.sha256.is_some(),
        }),
        // Bu sistemde kurulamasa da son sürüm bilgi olarak gösterilir.
        Err(_) => app.releases.iter().find(|r| !r.prerelease).map(|r| Latest {
            version: r.version.clone(),
            tag: r.tag.clone(),
            published_at: r.published_at.clone(),
            size: 0,
            verified: false,
        }),
    };
    app.update_available = app.supported
        && app.install.installed
        && matches!((&app.latest, &app.install.version), (Some(l), Some(v)) if version::compare(&l.version, v).is_gt());
    app.downloads = app.releases.iter().flat_map(|r| &r.assets).map(|a| a.downloads).sum();
}

/// Kurulu kopyanın exe yolundan çalışan bir süreç var mı (running_paths sonucuna göre)?
fn is_running_in(running: &HashMap<String, Vec<u32>>, info: &InstallInfo) -> bool {
    info.installed && info.exe_path.as_deref().is_some_and(|exe| running.contains_key(&installs::norm_path(exe)))
}

/// `current`'tan yeni, `latest`'ten eski ya da ona eşit kararlı sürümler; en yenisi önce.
fn newer_than(releases: &[Release], current: &str, latest: &str) -> Vec<Release> {
    let mut newer: Vec<Release> = releases
        .iter()
        .filter(|r| {
            !r.prerelease && version::compare(&r.version, current).is_gt() && version::compare(&r.version, latest).is_le()
        })
        .cloned()
        .collect();
    newer.sort_by(|a, b| version::compare(&b.version, &a.version));
    newer
}

/// Yeni sürüm listesini yazar ve ondan türetilenleri ("en son sürüm", "güncelleme var", indirme sayısı) hemen yeniden
/// hesaplar. Kurulum durumu değişmese de gerekir: yoksa yeni sürüm ancak bir uygulama açılıp kapanınca görünürdü.
fn set_releases(def: &AppDef, app: &mut AppState, releases: Vec<Release>) {
    app.releases = releases;
    derive(def, app);
}

/// Yeni algılama sonucunu uygulama durumlarına yazar ("güncelleme var" gibi türetilenler dahil).
/// Dönüş: kurulum bilgisi ya da çalışıyor durumu değişti mi.
fn apply_detection(
    apps: &mut BTreeMap<String, AppState>,
    defs: &[AppDef],
    detected: Vec<InstallInfo>,
    running: &HashMap<String, Vec<u32>>,
) -> bool {
    let mut changed = false;
    for (def, info) in defs.iter().zip(detected) {
        let Some(app) = apps.get_mut(&def.id) else { continue };
        let now_running = is_running_in(running, &info);
        if app.install != info || app.running != now_running {
            changed = true;
            app.install = info;
            app.running = now_running;
            derive(def, app);
        }
    }
    changed
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
            .connect_timeout(Duration::from_secs(6))
            .build()
            .map_err(|e| HubError::with("unknown", e))?;
        let settings = Settings::load(&paths.settings);
        let (catalog, _source) = catalog::load_local(&paths.catalog_cache);
        // Daha önce bulunan uygulamalar: önbellekteki sürüm listeleriyle, ağ beklemeden.
        let discovery = discover::State::load(&paths.discovery);
        let found: Vec<AppDef> = discovery
            .repos
            .iter()
            .filter_map(|info| discover::app_def(info, &github::cached_releases(&paths.releases, &info.repo)))
            .collect();
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
                found,
                discovery,
            }),
            paths,
            queue,
            refresh_lock: tokio::sync::Mutex::new(()),
            progress: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            emit_lock: Mutex::new(()),
            detect_lock: Mutex::new(()),
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
            "seq": self.seq.load(Ordering::SeqCst),
            "hubVersion": self.version,
            "dryRun": self.dry_run,
            "platform": platform::OS,
            "linuxFormat": platform::linux_format(),
            "appImage": platform::appimage_path().is_some(),
            "portable": platform::portable_dir().map(|d| d.display().to_string()),
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

    /// Durum değişti: arayüze yeni durum gider. Durumlar sıra numarasıyla ve sırayla yayınlanır; arayüz küçük
    /// numaralı (eski) durumu yok sayar.
    pub fn changed(&self) {
        {
            let _order = self.emit_lock.lock().unwrap_or_else(|p| p.into_inner());
            self.seq.fetch_add(1, Ordering::SeqCst);
            let _ = self.app.emit("hub:state", self.snapshot());
        }
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

    /// Kataloğu (kendiliğinden bulunan uygulamalar eklenerek) uygular; uygulama durumları korunur.
    fn set_catalog(&self, inner: &mut Inner, catalog: Catalog) {
        let catalog = discover::merge(catalog, &inner.found);
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
                stars: None,
                supported: false,
                unsupported: None,
                asset_pattern: None,
            });
            state.stars = inner.discovery.stars_of(&def.repo).or(state.stars);
            derive(def, &mut state);
            apps.insert(def.id.clone(), state);
        }
        inner.apps = apps;
        inner.catalog = catalog;
    }

    /// Kurulum durumlarını (kayıt defteri, klasörler, exe sürümü) ve çalışan uygulamaları yeniden okur.
    /// Dönüş: bir şey değişti mi.
    fn detect_installs(&self) -> bool {
        // Aynı anda tek algılama: iki algılama yarışırsa eski sonuç yenisinin üzerine yazılabilirdi (ör. kurulum biter
        // bitmez, 5 sn'lik denetimin kurulumdan önce okuduğu "kurulu değil" sonucu).
        let _one = self.detect_lock.lock().unwrap_or_else(|p| p.into_inner());
        let defs = self.lock().catalog.apps.clone();
        let detected: Vec<InstallInfo> = defs.iter().map(installs::detect).collect();
        // Çalışan süreçler kurulu exe'lerin adıyla aranır (kendiliğinden bulunanlarda exe adı katalogda yazmaz).
        let names: Vec<String> = detected
            .iter()
            .filter_map(|i| i.exe_path.as_deref())
            .filter_map(|p| std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        let running = installs::running_paths(&names);
        let mut inner = self.lock();
        inner.installs_refreshed = Some(Instant::now());
        apply_detection(&mut inner.apps, &defs, detected, &running)
    }

    /// Kurulum durumlarını yeniden okur ve arayüze her durumda yeni durumu gönderir.
    pub fn refresh_installs(&self) {
        self.detect_installs();
        self.changed();
    }

    /// Arka planda 5 sn'de bir (pencere kapalı ya da tepsideyken de): kurulu sürüm, kurulum/kaldırma ve çalışıyor
    /// bilgisi yeniden okunur; yalnız bir şey değiştiyse arayüze haber verilir. Böylece bir uygulama kendi içinden
    /// güncellendiğinde, Robz Hub dışında kurulduğunda ya da kaldırıldığında Robz Hub birkaç saniye içinde görür.
    pub fn poll(&self) {
        if self.detect_installs() {
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
            // Algılama diske ve süreç listesine bakar; pencereyi çizen ana iş parçacığını bekletmesin.
            let hub = self.clone();
            tauri::async_runtime::spawn_blocking(move || hub.refresh_installs());
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

        // İstekler birbirini beklemez: GitHub'a ulaşılamıyorsa zaman aşımları üst üste binmesin. Önce katalog ile
        // depo listesi (yeni Robz uygulamaları ve yıldızlar; saatte en çok bir kez) birlikte, sonra bütün sürüm
        // listeleri ve Robz Hub'ın kendi sürümü birlikte istenir.
        let (remote, (mut candidates, first_discovery)) = futures_util::future::join(
            async {
                if fetch_catalog {
                    catalog::fetch_remote(&self.client, &self.paths.catalog_cache).await
                } else {
                    None
                }
            },
            self.discover(),
        )
        .await;
        if fetch_catalog {
            let mut inner = self.lock();
            inner.catalog_fetched = Some(Instant::now());
            if let Some(remote) = remote.filter(|r| r.revision >= inner.catalog.revision) {
                self.set_catalog(&mut inner, remote);
            }
            // Yeni katalog adayı zaten içeriyorsa onun sürümleri aşağıda bir kez okunur.
            candidates.retain(|c| !inner.catalog.apps.iter().any(|a| a.repo.eq_ignore_ascii_case(&c.repo)));
        }

        let repos: Vec<(String, String)> =
            self.lock().catalog.apps.iter().map(|d| (d.id.clone(), d.repo.clone())).collect();
        let (results, candidate_results, ()) = futures_util::future::join3(
            futures_util::future::join_all(
                repos.iter().map(|(_, repo)| github::fetch_releases(&self.client, repo, &self.paths.releases)),
            ),
            futures_util::future::join_all(
                candidates.iter().map(|info| github::fetch_releases(&self.client, &info.repo, &self.paths.releases)),
            ),
            self.refresh_self(),
        )
        .await;
        let found_now = self.add_found(&candidates, candidate_results);

        let (mut problem, mut retry_at, mut reached) = (None, None::<u64>, false);
        {
            let mut inner = self.lock();
            let Inner { catalog, apps, .. } = &mut *inner;
            for ((id, _), result) in repos.iter().zip(results) {
                let (Some(def), Some(app)) = (catalog.app(id), apps.get_mut(id)) else { continue };
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
                    set_releases(def, app, result.releases);
                }
            }
        }

        if let Some(problem) = problem {
            log::write(format!("GitHub denetimi: {problem}"));
        }
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
        // İlk denetimde bulunanlar zaten listededir; sonradan çıkan yeni bir uygulama haber verilir.
        if !found_now.is_empty() && !first_discovery {
            self.notice(json!({ "type": "apps-found", "apps": found_now }));
        }
    }

    /// Sahibin depo listesini (vakti geldiyse) okur: yıldız sayılarını günceller, listeden kalkan bulunmuş uygulamaları
    /// çıkarır ve sürüm listesine bakılacak yeni robz- depolarını döndürür. İkinci değer: bu ilk denetim mi.
    async fn discover(&self) -> (Vec<RepoInfo>, bool) {
        let (hub_repo, state) = {
            let inner = self.lock();
            (inner.catalog.hub.as_ref().map(|h| h.repo.clone()), inner.discovery.clone())
        };
        let now = now_ms();
        let Some(hub_repo) = hub_repo.filter(|_| state.due(now)) else { return (Vec::new(), false) };
        let Some(owner) = discover::owner_of(&hub_repo).map(str::to_string) else { return (Vec::new(), false) };
        let result = discover::fetch_list(&self.client, &owner, &hub_repo, state.etag.as_deref()).await;

        let mut inner = self.lock();
        match result {
            ListResult::Changed { repos, stars, etag } => {
                inner.discovery.repos = repos;
                inner.discovery.stars = stars;
                inner.discovery.etag = etag;
            }
            ListResult::Same => {}
            ListResult::Failed => return (Vec::new(), false),
        }
        inner.discovery.checked_at = now;
        let listed: HashSet<String> = inner.discovery.repos.iter().map(|r| r.repo.to_ascii_lowercase()).collect();
        inner.discovery.save(&self.paths.discovery);
        inner.found.retain(|d| listed.contains(&d.repo.to_ascii_lowercase()));
        let catalog = inner.catalog.clone();
        self.set_catalog(&mut inner, catalog);

        let known: HashSet<String> = inner.catalog.apps.iter().map(|a| a.repo.to_ascii_lowercase()).collect();
        let candidates = inner
            .discovery
            .repos
            .iter()
            .filter(|r| !known.contains(&r.repo.to_ascii_lowercase()))
            .cloned()
            .collect();
        (candidates, state.checked_at == 0)
    }

    /// Yeni robz- depolarını sürüm listeleriyle birlikte uygulama olarak ekler (adı sürüm adından gelir).
    /// Dönüş: eklenen uygulamalar ({ id, name }).
    fn add_found(&self, candidates: &[RepoInfo], results: Vec<github::FetchResult>) -> Vec<Value> {
        if candidates.is_empty() {
            return Vec::new();
        }
        let mut added = Vec::new();
        let mut inner = self.lock();
        for (info, result) in candidates.iter().zip(results) {
            if !matches!(result.status, FetchStatus::Ok | FetchStatus::Cached | FetchStatus::Missing) {
                continue; // internet ya da sınır: sonraki denetimde yeniden denenir
            }
            if let Some(def) = discover::app_def(info, &result.releases) {
                added.push(json!({ "id": def.id, "name": def.name }));
                inner.found.retain(|d| d.id != def.id);
                inner.found.push(def);
            }
        }
        let catalog = inner.catalog.clone();
        self.set_catalog(&mut inner, catalog);
        added
    }

    /// Robz Hub'ın kendi yeni sürümü var mı? Depo henüz yoksa ya da sürüm yayınlanmadıysa sessizce geçilir.
    async fn refresh_self(&self) {
        let Some(hub) = self.lock().catalog.hub.clone() else { return };
        let result = github::fetch_releases(&self.client, &hub.repo, &self.paths.releases).await;
        // Bu sistemde kendini güncelleyebileceği dosya; yoksa (ör. elle derlenmiş Linux kopyası) yeni sürüm yine
        // gösterilir, indirme sürüm sayfasından yapılır.
        let pick = platform::hub_asset(&hub)
            .and_then(|re| github::pick_latest(&result.releases, re))
            .map(|(r, a)| (r.clone(), a.clone()));
        let shown = pick.clone().or_else(|| {
            github::pick_latest(&result.releases, &hub.asset_re).map(|(r, a)| (r.clone(), a.clone()))
        });
        let mut inner = self.lock();
        inner.self_state = match &shown {
            Some((release, asset)) => SelfState {
                latest: Some(SelfLatest {
                    version: release.version.clone(),
                    published_at: release.published_at.clone(),
                    size: asset.size,
                }),
                update_available: version::compare(&release.version, &self.version).is_gt(),
                releases: newer_than(&result.releases, &self.version, &release.version),
                url: Some(release.url.clone()),
                installable: pick.is_some(),
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
            let what = if job.kind == JobKind::Install { "kur/güncelle" } else { "kaldır" };
            // Önce kurulum durumu yeniden okunur, sonra iş kapanır: arayüz "iş bitti" ile "kuruldu"yu birlikte görür;
            // arada "Kur" düğmesi yeniden belirmez.
            self.detect_installs();
            match result {
                Ok(notice) => {
                    log::write(format!("{} {what}: tamam {notice}", job.id));
                    self.clear_job(&job.id);
                    self.notice(notice);
                }
                Err(error) if error.is("cancelled") => {
                    log::write(format!("{} {what}: iptal edildi", job.id));
                    self.clear_job(&job.id);
                }
                Err(error) => {
                    log::write(format!("{} {what}: HATA {error}", job.id));
                    self.update_job(&job.id, |j| {
                        j.phase = "error";
                        j.error = Some(error);
                    });
                }
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
        let def = self.lock().catalog.app(&job.id).cloned().ok_or_else(|| HubError::new("unknown-app"))?;
        let installed = installs::detect(&def);
        let (method, pick, keep, kind) = {
            let inner = self.lock();
            let releases = inner.apps.get(&job.id).map(|a| a.releases.as_slice()).unwrap_or_default();
            // Bu sistemde hangi dosya, hangi yoldan (kuruluysa kurulduğu yoldan) kurulur.
            let plan = platform::plan(&def, &installed, releases).map_err(|_| HubError::new("unsupported"))?;
            let pick = github::pick_latest(releases, plan.asset).map(|(r, a)| (r.clone(), a.clone()));
            let kind = inner.jobs.get(&job.id).map(|j| j.kind).unwrap_or("install");
            (plan.method, pick, inner.settings.keep_installers, kind)
        };
        let (release, asset) = pick.ok_or_else(|| HubError::new("no-release"))?;
        self.update_job(&job.id, |j| {
            j.phase = "downloading";
            j.version = Some(release.version.clone());
            j.total = asset.size;
        });

        let dir = self.paths.downloads.join(&def.id).join(tag_dir(&release.tag));
        let result = self.download_and_install(job, &def, &asset, &dir, method, &release.version).await;
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

    async fn download_and_install(
        &self,
        job: &QueuedJob,
        def: &AppDef,
        asset: &Asset,
        dir: &std::path::Path,
        method: platform::Method,
        version: &str,
    ) -> HubResult<()> {
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
        let icon = if method == platform::Method::AppImage { self.app_icon(def).await } else { None };
        installs::run_installer(def, &file, method, version, icon).await?;
        for _ in 0..20 {
            if installs::detect(def).installed {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(HubError::with("installer-failed", "kurulumdan sonra uygulama bulunamadı"))
    }

    /// Uygulamanın simgesi (AppImage'ın uygulama menüsü girişi için): Robz Hub'ın içindeki dosya ya da https adresi.
    /// Yalnız PNG ve SVG; alınamazsa simgesiz giriş yazılır.
    async fn app_icon(&self, def: &AppDef) -> Option<(Vec<u8>, &'static str)> {
        let source = def.icon.as_deref()?;
        let ext = if source.ends_with(".svg") {
            "svg"
        } else if source.ends_with(".png") {
            "png"
        } else {
            return None;
        };
        let bytes = if let Some(local) = source.strip_prefix("assets/") {
            self.app.asset_resolver().get(format!("assets/{local}"))?.bytes
        } else {
            let res = self.client.get(source).timeout(Duration::from_secs(10)).send().await.ok()?;
            if !res.status().is_success() {
                return None;
            }
            res.bytes().await.ok()?.to_vec()
        };
        (bytes.len() <= 1 << 20).then_some((bytes, ext))
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
                hub.poll();
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
        self.poll();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState {
            install: InstallInfo::default(),
            running: false,
            releases: Vec::new(),
            latest: None,
            update_available: false,
            fetch_status: None,
            downloads: 0,
            stars: None,
            supported: true,
            unsupported: None,
            asset_pattern: None,
        }
    }

    fn installed(version: &str, exe: &str) -> InstallInfo {
        InstallInfo {
            installed: true,
            version: Some(version.to_string()),
            dir: Some(r"C:\Apps\Robz Vision".to_string()),
            exe_path: Some(exe.to_string()),
            uninstaller: None,
            scope: Some("user"),
            source: Some("registry"),
        }
    }

    #[test]
    fn new_releases_update_latest_version_right_away() {
        let catalog = catalog::parse_str(catalog::BUNDLED).unwrap();
        let def = catalog.app("robz-vision").unwrap();
        let mut app = state();
        app.install = installed("1.3.2", r"C:\Apps\Robz Vision\RobzVision.exe");
        let mut release = release("1.6.0", false);
        release.assets = vec![Asset {
            name: "RobzVision-Setup-1.6.0-x64.exe".into(),
            size: 1,
            url: String::new(),
            sha256: None,
            downloads: 97,
        }];
        // Kurulum durumu değişmeden yalnız GitHub'dan yeni liste geldi: güncelleme ve indirme sayısı hemen görünmeli.
        set_releases(def, &mut app, vec![release.clone()]);
        assert_eq!(app.latest.as_ref().map(|l| l.version.as_str()), Some("1.6.0"));
        assert!(app.update_available);
        assert_eq!(app.downloads, 97);

        // Bu sistemde kurulamayan uygulama (ör. Linux'ta Robz Vision): son sürüm bilgi olarak görünür, güncelleme önerilmez.
        derive_with(&mut app, Err(platform::Unsupported::Distro));
        assert!(!app.supported);
        assert_eq!(app.unsupported, Some(platform::Unsupported::Distro));
        assert_eq!(app.latest.as_ref().map(|l| l.version.as_str()), Some("1.6.0"));
        assert!(!app.update_available);
    }

    #[test]
    fn notices_updates_made_outside_robz_hub() {
        let catalog = catalog::parse_str(catalog::BUNDLED).unwrap();
        let defs: Vec<AppDef> = catalog.apps.iter().filter(|d| d.id == "robz-vision").cloned().collect();
        let exe = r"C:\Apps\Robz Vision\RobzVision.exe";
        let mut apps = BTreeMap::from([("robz-vision".to_string(), state())]);
        // GitHub'daki son sürüm 1.6.0 ("güncelleme var" bu listeden türetilir).
        apps.get_mut("robz-vision").unwrap().releases = vec![Release {
            id: 1,
            tag: "v1.6.0".into(),
            version: "1.6.0".into(),
            name: "Robz Vision 1.6.0".into(),
            published_at: String::new(),
            prerelease: false,
            url: String::new(),
            body: String::new(),
            assets: vec![Asset {
                name: "RobzVision-Setup-1.6.0-x64.exe".into(),
                size: 1,
                url: String::new(),
                sha256: None,
                downloads: 34,
            }],
        }];
        let nothing_running = HashMap::new();

        // Kurulu 1.3.2: güncelleme var.
        assert!(apply_detection(&mut apps, &defs, vec![installed("1.3.2", exe)], &nothing_running));
        assert!(apps["robz-vision"].update_available);

        // Aynı şey yeniden okunursa değişiklik yok (arayüze boşuna haber gitmez).
        assert!(!apply_detection(&mut apps, &defs, vec![installed("1.3.2", exe)], &nothing_running));

        // Uygulama kendi içinden 1.6.0'a güncellendi: sürüm değişir, "güncelleme var" kalkar.
        assert!(apply_detection(&mut apps, &defs, vec![installed("1.6.0", exe)], &nothing_running));
        assert_eq!(apps["robz-vision"].install.version.as_deref(), Some("1.6.0"));
        assert!(!apps["robz-vision"].update_available);

        // Kurulu kopya açıldı: yalnız çalışıyor bilgisi değişse de haber verilir.
        let running = HashMap::from([(installs::norm_path(exe), vec![42])]);
        assert!(apply_detection(&mut apps, &defs, vec![installed("1.6.0", exe)], &running));
        assert!(apps["robz-vision"].running);

        // Windows Ayarlar'dan kaldırıldı.
        assert!(apply_detection(&mut apps, &defs, vec![InstallInfo::default()], &nothing_running));
        assert!(!apps["robz-vision"].install.installed && !apps["robz-vision"].running);
    }

    fn release(version: &str, prerelease: bool) -> Release {
        Release {
            id: 1,
            tag: format!("v{version}"),
            version: version.into(),
            name: format!("Robz Hub {version}"),
            published_at: String::new(),
            prerelease,
            url: String::new(),
            body: String::new(),
            assets: Vec::new(),
        }
    }

    #[test]
    fn lists_every_hub_release_since_the_installed_one() {
        let releases = vec![
            release("0.1.2", false),
            release("0.2.0-rc1", true),
            release("0.1.4", false),
            release("0.1.1", false),
            release("0.1.3", false),
            release("0.1.0", false),
        ];
        let versions = |list: Vec<Release>| list.into_iter().map(|r| r.version).collect::<Vec<_>>();
        // 0.1.1 kurulu, son kararlı 0.1.4: aradaki üç sürüm, en yenisi önce; ön sürüm ve eskiler yok.
        assert_eq!(versions(newer_than(&releases, "0.1.1", "0.1.4")), ["0.1.4", "0.1.3", "0.1.2"]);
        assert!(newer_than(&releases, "0.1.4", "0.1.4").is_empty());
    }
}
