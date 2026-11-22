//! Uygulama kataloğu: hangi uygulamalar var, hangi depoda, nasıl kurulur.
//!
//! Kaynak sırası: GitHub'daki güncel catalog.json -> son başarılı indirmenin kopyası -> uygulamanın içindeki kopya.
//! Böylece kataloğa yeni bir uygulama eklemek için Robz Hub'ın yeni sürümünü yayınlamak gerekmez.
//!
//! Katalog uzaktan geldiği için burada sıkı doğrulanır: kimlikler, depo adları, kayıt anahtarları ve dosya adları yalnız
//! beklenen biçimdeyse kabul edilir. Kurulum parametreleri katalogdan gelmez; kurulum türüne göre kodda sabittir.

use regex::{Regex, RegexBuilder};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

pub const SCHEMA: u64 = 1;
pub const REMOTE_URL: &str = "https://raw.githubusercontent.com/kutsalkiremitci/robz-hub/main/catalog.json";
pub const BUNDLED: &str = include_str!("../../catalog.json");

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("geçerli düzenli ifade")
}

static ID: LazyLock<Regex> = LazyLock::new(|| re(r"^[a-z0-9][a-z0-9-]{0,39}$"));
static REPO: LazyLock<Regex> = LazyLock::new(|| re(r"^[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9._-]{1,100}$"));
static EXE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^[A-Za-z0-9 _.-]{1,60}\.exe$"));
pub static SAFE_KEY: LazyLock<Regex> = LazyLock::new(|| re(r"^[A-Za-z0-9 _.{}-]{1,120}$"));
static DEFAULT_DIR: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)^%(LOCALAPPDATA|APPDATA|PROGRAMFILES|PROGRAMFILES\(X86\))%(\\[A-Za-z0-9 _.-]{1,60}){1,4}$")
});
static HTTPS: LazyLock<Regex> = LazyLock::new(|| re(r#"^https://[^\s"'<>]+$"#));
static LOCAL_IMAGE: LazyLock<Regex> = LazyLock::new(|| re(r"^assets/[A-Za-z0-9/_.-]+\.(png|webp|svg|jpg)$"));
static LINK_KEY: LazyLock<Regex> = LazyLock::new(|| re(r"^[a-z]{2,20}$"));
static DATE: LazyLock<Regex> = LazyLock::new(|| re(r"^\d{4}-\d{2}-\d{2}$"));

/// Türkçe ve İngilizce metin çifti.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Text {
    pub tr: String,
    pub en: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Highlight {
    pub title: Text,
    pub text: Text,
}

#[derive(Debug, Clone, Serialize)]
pub struct Screenshot {
    pub url: String,
    pub caption: Option<Text>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseNotesConf {
    pub from: Option<String>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallerType {
    Inno,
    Nsis,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsConf {
    pub installer: InstallerType,
    pub asset: String,
    pub uninstall_key: String,
    pub default_dir: Option<String>,
    pub exe: String,
    #[serde(skip)]
    pub asset_re: Regex,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDef {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub license: Option<String>,
    pub tagline: Text,
    pub role: Option<Text>,
    pub description: Text,
    pub highlights: Vec<Highlight>,
    pub requirements: Vec<Text>,
    pub notice: Option<Text>,
    pub icon: Option<String>,
    /// Ana sayfadaki uygulama kartının kapak görseli (yoksa ilk ekran görüntüsü).
    pub cover: Option<String>,
    pub screenshots: Vec<Screenshot>,
    pub links: BTreeMap<String, String>,
    pub release_notes: ReleaseNotesConf,
    pub windows: WindowsConf,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub id: String,
    pub title: Text,
    pub description: Option<Text>,
    pub apps: Vec<String>,
    /// Ana sayfadaki tanıtım alanının arka plan görseli.
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub id: String,
    pub title: Text,
    pub body: Option<Text>,
    pub url: Option<String>,
    pub link_label: Option<Text>,
    pub date: Option<String>,
    /// Ana sayfadaki tanıtım alanının arka plan görseli.
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubConf {
    pub repo: String,
    pub asset: String,
    #[serde(skip)]
    pub asset_re: Regex,
}

#[derive(Debug, Clone, Serialize)]
pub struct Catalog {
    pub schema: u64,
    pub revision: u64,
    pub apps: Vec<AppDef>,
    pub bundles: Vec<Bundle>,
    pub announcements: Vec<Announcement>,
    pub links: BTreeMap<String, String>,
    pub hub: Option<HubConf>,
}

impl Catalog {
    pub fn app(&self, id: &str) -> Option<&AppDef> {
        self.apps.iter().find(|a| a.id == id)
    }

    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.bundles.iter().find(|b| b.id == id)
    }
}

// ---- doğrulama yardımcıları ----------------------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn string(value: &Value, max: usize) -> Option<String> {
    value.as_str().map(|s| truncate(s, max))
}

fn matches(value: &Value, pattern: &Regex) -> Option<String> {
    value.as_str().filter(|s| pattern.is_match(s)).map(str::to_string)
}

/// { tr, en } çifti ya da düz metin; ikisi de { tr, en } olarak döner.
fn text(value: &Value, max: usize) -> Option<Text> {
    if let Some(s) = value.as_str() {
        return Some(Text { tr: truncate(s, max), en: truncate(s, max) });
    }
    let tr = value.get("tr").and_then(Value::as_str);
    let en = value.get("en").and_then(Value::as_str);
    let (tr, en) = (tr.or(en)?, en.or(tr)?);
    Some(Text { tr: truncate(tr, max), en: truncate(en, max) })
}

/// Görsel: uygulamanın içindeki dosya ("assets/…") ya da https adresi.
fn image(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    if HTTPS.is_match(s) || (LOCAL_IMAGE.is_match(s) && !s.contains("..")) {
        Some(s.to_string())
    } else {
        None
    }
}

fn pattern(value: &Value) -> Option<(String, Regex)> {
    let s = value.as_str().filter(|s| s.len() <= 200)?;
    let compiled = RegexBuilder::new(s).size_limit(1 << 20).build().ok()?;
    Some((s.to_string(), compiled))
}

fn links(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .filter(|(k, v)| LINK_KEY.is_match(k) && v.as_str().is_some_and(|u| HTTPS.is_match(u)))
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn parse_app(raw: &Value) -> Option<AppDef> {
    let id = matches(&raw["id"], &ID)?;
    let repo = matches(&raw["repo"], &REPO)?;
    let win = &raw["windows"];
    let installer = match win["installer"].as_str()? {
        "inno" => InstallerType::Inno,
        "nsis" => InstallerType::Nsis,
        _ => return None,
    };
    let (asset, asset_re) = pattern(&win["asset"])?;
    let exe = matches(&win["exe"], &EXE)?;
    let uninstall_key = matches(&win["uninstallKey"], &SAFE_KEY)?;
    let default_dir = match &win["defaultDir"] {
        Value::Null => None,
        v => Some(matches(v, &DEFAULT_DIR)?),
    };
    let name = string(&raw["name"], 60)?;
    let tagline = text(&raw["tagline"], 200)?;
    let notes = &raw["releaseNotes"];

    Some(AppDef {
        id,
        name,
        repo,
        license: string(&raw["license"], 40),
        description: text(&raw["description"], 1200).unwrap_or_else(|| tagline.clone()),
        tagline,
        role: text(&raw["role"], 60),
        highlights: array(&raw["highlights"])
            .iter()
            .filter_map(|h| Some(Highlight { title: text(&h["title"], 80)?, text: text(&h["text"], 300)? }))
            .take(8)
            .collect(),
        requirements: array(&raw["requirements"]).iter().filter_map(|r| text(r, 200)).take(10).collect(),
        notice: text(&raw["notice"], 300),
        icon: image(&raw["icon"]),
        cover: image(&raw["cover"]),
        screenshots: array(&raw["screenshots"])
            .iter()
            .filter_map(|s| Some(Screenshot { url: image(&s["url"])?, caption: text(&s["caption"], 200) }))
            .take(12)
            .collect(),
        links: links(&raw["links"]),
        release_notes: ReleaseNotesConf {
            from: string(&notes["from"], 80),
            languages: array(&notes["languages"])
                .iter()
                .filter_map(Value::as_str)
                .filter(|l| *l == "tr" || *l == "en")
                .map(str::to_string)
                .collect(),
        },
        windows: WindowsConf { installer, asset, uninstall_key, default_dir, exe, asset_re },
    })
}

/// Ham JSON'u doğrulanmış kataloğa çevirir. Kullanılamıyorsa None.
pub fn parse(raw: &Value) -> Option<Catalog> {
    if raw["schema"].as_u64() != Some(SCHEMA) {
        return None;
    }
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    for item in array(&raw["apps"]) {
        if let Some(app) = parse_app(item) {
            if seen.insert(app.id.clone()) {
                apps.push(app);
            }
        }
    }
    if apps.is_empty() {
        return None;
    }

    let bundles = array(&raw["bundles"])
        .iter()
        .filter_map(|b| {
            let id = matches(&b["id"], &ID)?;
            let members: Vec<String> = array(&b["apps"])
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| seen.contains(*id))
                .map(str::to_string)
                .collect();
            if members.len() < 2 {
                return None;
            }
            Some(Bundle {
                id,
                title: text(&b["title"], 120)?,
                description: text(&b["description"], 500),
                apps: members,
                image: image(&b["image"]),
            })
        })
        .collect();

    let announcements = array(&raw["announcements"])
        .iter()
        .filter_map(|a| {
            Some(Announcement {
                id: matches(&a["id"], &ID)?,
                title: text(&a["title"], 120)?,
                body: text(&a["body"], 400),
                url: matches(&a["url"], &HTTPS),
                link_label: text(&a["linkLabel"], 40),
                date: matches(&a["date"], &DATE),
                image: image(&a["image"]),
            })
        })
        .take(5)
        .collect();

    let hub = (|| {
        let repo = matches(&raw["hub"]["repo"], &REPO)?;
        let (asset, asset_re) = pattern(&raw["hub"]["asset"])?;
        Some(HubConf { repo, asset, asset_re })
    })();

    Some(Catalog {
        schema: SCHEMA,
        revision: raw["revision"].as_u64().unwrap_or(0),
        apps,
        bundles,
        announcements,
        links: links(&raw["links"]),
        hub,
    })
}

pub fn parse_str(text: &str) -> Option<Catalog> {
    parse(&serde_json::from_str::<Value>(text).ok()?)
}

/// Açılışta: önbellekteki uzak kopya ile uygulamanın içindeki kopyadan "revision" değeri büyük olan.
/// (Robz Hub güncellenince yanında gelen katalog, eski bir önbellek kopyasının gerisinde kalmasın.)
pub fn load_local(cache_path: &Path) -> (Catalog, &'static str) {
    let bundled = parse_str(BUNDLED).expect("uygulamanın içindeki catalog.json geçerli olmalı");
    let cached = std::fs::read_to_string(cache_path).ok().and_then(|s| parse_str(&s));
    match cached {
        Some(cached) if cached.revision >= bundled.revision => (cached, "cache"),
        _ => (bundled, "bundled"),
    }
}

/// Güncel kataloğu GitHub'dan alır; geçerliyse önbelleğe yazar. Başarısızsa None (elde olanla devam edilir).
pub async fn fetch_remote(client: &reqwest::Client, cache_path: &Path) -> Option<Catalog> {
    let res = client
        .get(REMOTE_URL)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    let body = res.text().await.ok()?;
    if body.len() > 512 * 1024 {
        return None;
    }
    let catalog = parse_str(&body)?;
    if let Some(dir) = cache_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(cache_path, body);
    Some(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bundled_catalog_is_valid() {
        let catalog = parse_str(BUNDLED).expect("geçerli katalog");
        let ids: Vec<&str> = catalog.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["robz-play", "robz-vision"]);
        assert_eq!(catalog.bundles.len(), 1);
        assert!(catalog.bundles[0].image.as_deref().is_some_and(|u| u.starts_with("https://")));
        let play = catalog.app("robz-play").unwrap();
        assert!(play.windows.asset_re.is_match("RobzPlay-Setup.exe"));
        assert_eq!(play.windows.installer, InstallerType::Nsis);
        let vision = catalog.app("robz-vision").unwrap();
        assert!(vision.windows.asset_re.is_match("RobzVision-Setup-1.6.0-x64.exe"));
        assert!(!vision.windows.asset_re.is_match("RobzVision-Portable-1.6.0-x64.zip"));
        assert!(catalog.hub.unwrap().asset_re.is_match("RobzHub-Setup-0.2.0.exe"));
    }

    fn app(overrides: Value) -> Value {
        let mut base = json!({
            "id": "demo",
            "name": "Demo",
            "repo": "owner/demo",
            "tagline": "Demo",
            "windows": {
                "installer": "nsis",
                "asset": "^Demo\\.exe$",
                "uninstallKey": "Demo",
                "exe": "Demo.exe"
            }
        });
        for (k, v) in overrides.as_object().unwrap() {
            if k == "windows" {
                for (wk, wv) in v.as_object().unwrap() {
                    base["windows"][wk] = wv.clone();
                }
            } else {
                base[k] = v.clone();
            }
        }
        json!({ "schema": 1, "apps": [base] })
    }

    #[test]
    fn rejects_unsafe_entries() {
        assert!(parse(&app(json!({}))).is_some());
        assert!(parse(&app(json!({ "id": "../etc" }))).is_none());
        assert!(parse(&app(json!({ "repo": "owner/demo/../x" }))).is_none());
        assert!(parse(&app(json!({ "windows": { "exe": "..\\cmd.exe" } }))).is_none());
        assert!(parse(&app(json!({ "windows": { "exe": "run.bat" } }))).is_none());
        assert!(parse(&app(json!({ "windows": { "uninstallKey": "a\" & calc & \"" } }))).is_none());
        assert!(parse(&app(json!({ "windows": { "installer": "msi" } }))).is_none());
        assert!(parse(&app(json!({ "windows": { "defaultDir": "C:\\Windows" } }))).is_none());
        assert!(parse(&app(json!({ "windows": { "asset": "(" } }))).is_none());
        assert!(parse(&json!({ "schema": 2, "apps": [] })).is_none());
    }

    #[test]
    fn drops_unsafe_links_and_images() {
        let catalog = parse(&app(json!({
            "icon": "assets/../../secret.png",
            "links": { "website": "javascript:alert(1)", "guide": "https://example.com/guide" },
            "screenshots": [{ "url": "http://insecure.example/a.png" }, { "url": "https://example.com/b.png" }]
        })))
        .unwrap();
        let demo = &catalog.apps[0];
        assert_eq!(demo.icon, None);
        assert_eq!(demo.links.len(), 1);
        assert_eq!(demo.screenshots.len(), 1);
    }

    #[test]
    fn text_accepts_plain_or_pair() {
        assert_eq!(text(&json!("x"), 10), Some(Text { tr: "x".into(), en: "x".into() }));
        assert_eq!(text(&json!({ "en": "y" }), 10), Some(Text { tr: "y".into(), en: "y".into() }));
        assert_eq!(text(&json!({ "tr": "çok uzun metin" }), 3).unwrap().tr, "çok");
        assert_eq!(text(&json!(5), 10), None);
    }
}
