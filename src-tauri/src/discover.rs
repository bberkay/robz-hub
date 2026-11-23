//! Yeni Robz uygulamalarını kendiliğinden bulur: GitHub'da "robz-" ile başlayan herkese açık bir depo, kataloğa
//! eklenmeden ve Robz Hub güncellenmeden Robz Hub'da görünür.
//!
//! GitHub'a saatte en çok bir kez tek bir istek gider: sahibin depo listesi (/users/<sahip>/repos). Kimliksiz
//! isteklerde 304 yanıtı da saatlik 60 isteklik sınırdan düşer (ETag yalnız veriyi küçültür); bu yüzden liste bir
//! saatten sık sorulmaz. Bulunan her robz- deposu listede görünür; hangi sistemde kurulabileceğine sürümündeki
//! dosyalar karar verir (platform::plan): Windows kurulum programı, .deb, .rpm ya da AppImage. Henüz sürümü yoksa
//! "henüz sürüm yok" görünür.
//!
//! Bulunan uygulamanın kayıt anahtarı ve exe adı önceden bilinmez: kurulum, Windows'un "Yüklü uygulamalar" kaydında
//! adıyla aranır (installs::detect), kurulum türü (Inno Setup ya da NSIS) indirilen dosyanın içinden okunur. Linux'ta
//! paket ve program adı depo adı sayılır (Tauri'nin adlandırması).
//! Yalnız sahibin kendi depoları alınır; çatallar (fork), arşivlenmiş depolar ve Robz Hub'ın kendisi atlanır.

use crate::catalog::{self, AppDef, Catalog, ReleaseNotesConf, Text};
use crate::github::Release;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

pub const PREFIX: &str = "robz-";
/// Depo listesi en çok bu sıklıkta sorulur.
pub const CHECK_EVERY_MS: u64 = 60 * 60 * 1000;
static TRAILING_VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)[\s_-]*v?\d+(\.\d+){0,3}\s*$").expect("geçerli düzenli ifade"));

/// Listede görülen bir robz- deposu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    /// "sahip/robz-…"
    pub repo: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
}

/// Diskte saklanan durum (discovered.json): açılışta ağ beklemeden bulunan uygulamalar gösterilir.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// Son başarılı liste denetimi (ms). 0: hiç denetlenmedi.
    pub checked_at: u64,
    pub etag: Option<String>,
    pub repos: Vec<RepoInfo>,
    /// Sahibin bütün depolarının yıldız sayısı (anahtar: küçük harfle "sahip/depo"). Aynı listeden gelir; ek istek yok.
    #[serde(default)]
    pub stars: BTreeMap<String, u64>,
}

impl State {
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, file: &Path) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(self) {
            let _ = std::fs::write(file, text);
        }
    }

    pub fn due(&self, now: u64) -> bool {
        now.saturating_sub(self.checked_at) >= CHECK_EVERY_MS
    }

    pub fn stars_of(&self, repo: &str) -> Option<u64> {
        self.stars.get(&repo.to_ascii_lowercase()).copied()
    }
}

pub enum ListResult {
    /// Yeni liste: robz- depoları, bütün depoların yıldızları ve ETag.
    Changed { repos: Vec<RepoInfo>, stars: BTreeMap<String, u64>, etag: Option<String> },
    /// Liste değişmemiş (304).
    Same,
    /// İnternet yok, oran sınırı ya da hata: elde olanla devam edilir.
    Failed,
}

/// "sahip/robz-hub" -> "sahip"
pub fn owner_of(repo: &str) -> Option<&str> {
    repo.split_once('/').map(|(owner, _)| owner).filter(|o| !o.is_empty())
}

/// Sahibin herkese açık depoları: tek istek, en çok 100 depo.
pub async fn fetch_list(client: &reqwest::Client, owner: &str, hub_repo: &str, etag: Option<&str>) -> ListResult {
    let mut request = client
        .get(format!("https://api.github.com/users/{owner}/repos?type=owner&per_page=100&sort=created"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .timeout(Duration::from_secs(10));
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let res = match request.send().await {
        Ok(res) => res,
        Err(_) => return ListResult::Failed,
    };
    match res.status().as_u16() {
        304 => return ListResult::Same,
        403 | 429 => return ListResult::Failed,
        code if !(200..300).contains(&code) => return ListResult::Failed,
        _ => {}
    }
    let etag = res.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_string);
    let Some(list) = res.text().await.ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
        return ListResult::Failed;
    };
    ListResult::Changed { repos: candidates(&list, owner, hub_repo), stars: stars(&list), etag }
}

/// Listedeki her deponun yıldız sayısı.
pub fn stars(list: &Value) -> BTreeMap<String, u64> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|r| Some((r["full_name"].as_str()?.to_ascii_lowercase(), r["stargazers_count"].as_u64()?)))
                .collect()
        })
        .unwrap_or_default()
}

/// Depo listesinden robz- ile başlayan, sahibin kendi, herkese açık ve etkin depoları (Robz Hub hariç).
pub fn candidates(list: &Value, owner: &str, hub_repo: &str) -> Vec<RepoInfo> {
    let Some(items) = list.as_array() else { return Vec::new() };
    let clip = |v: &Value, max: usize| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.chars().take(max).collect());
    items
        .iter()
        .filter(|r| {
            let name = r["name"].as_str().unwrap_or_default();
            name.to_ascii_lowercase().starts_with(PREFIX)
                && r["owner"]["login"].as_str().is_some_and(|o| o.eq_ignore_ascii_case(owner))
                && r["fork"].as_bool() == Some(false)
                && r["archived"].as_bool() != Some(true)
                && r["disabled"].as_bool() != Some(true)
                && r["private"].as_bool() != Some(true)
        })
        .filter_map(|r| {
            let repo = r["full_name"].as_str().filter(|n| catalog::REPO.is_match(n))?;
            if repo.eq_ignore_ascii_case(hub_repo) {
                return None;
            }
            Some(RepoInfo {
                repo: repo.to_string(),
                description: clip(&r["description"], 300),
                homepage: r["homepage"].as_str().filter(|u| catalog::HTTPS.is_match(u)).map(str::to_string),
                license: clip(&r["license"]["spdx_id"], 40).filter(|l: &String| l != "NOASSERTION"),
            })
        })
        .collect()
}

/// Gösterilecek ad: son sürümün adı sürüm numarası atılarak ("Robz DPI 0.2.0" -> "Robz DPI"); olmazsa depo adı
/// ("robz-dpi" -> "Robz Dpi").
pub fn display_name(releases: &[Release], repo: &str) -> String {
    let slug = repo.rsplit('/').next().unwrap_or(repo);
    let from_release = releases
        .iter()
        .find(|r| !r.prerelease)
        .map(|r| TRAILING_VERSION.replace(r.name.trim(), "").trim().to_string())
        .filter(|n| (3..=60).contains(&n.chars().count()) && normalize(n).starts_with("robz"));
    from_release.unwrap_or_else(|| {
        slug.split(['-', '_'])
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut chars = w.chars();
                chars.next().map(|c| c.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Ad karşılaştırması için: yalnız küçük harf ve rakamlar ("Robz DPI" -> "robzdpi").
pub fn normalize(text: &str) -> String {
    text.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Depodan uygulama tanımı. Sürümü olmasa da bulunan her robz- deposu bir uygulamadır; hangi sistemde kurulacağına
/// sürümdeki dosyalar karar verir (catalog::fill_defaults, platform::plan).
pub fn app_def(info: &RepoInfo, releases: &[Release]) -> Option<AppDef> {
    let slug = info.repo.rsplit('/').next()?.to_ascii_lowercase();
    if !catalog::ID.is_match(&slug) {
        return None;
    }
    let name = display_name(releases, &info.repo);
    let tagline = info
        .description
        .as_deref()
        .and_then(|d| catalog::text(&Value::String(d.to_string()), 200))
        .unwrap_or_else(|| Text { tr: name.clone(), en: name.clone() });
    let description = info
        .description
        .as_deref()
        .and_then(|d| catalog::text(&Value::String(d.to_string()), 1200))
        .unwrap_or_else(|| tagline.clone());
    let mut links = BTreeMap::new();
    if let Some(homepage) = &info.homepage {
        links.insert("website".to_string(), homepage.clone());
    }
    let mut def = AppDef {
        id: slug,
        name,
        repo: info.repo.clone(),
        license: info.license.clone(),
        tagline,
        role: None,
        description,
        highlights: Vec::new(),
        requirements: Vec::new(),
        notice: None,
        // Tauri ile yazılmış Robz uygulamalarının simgesi bu yolda durur; yoksa arayüz baş harfleri gösterir.
        icon: Some(format!("https://raw.githubusercontent.com/{}/HEAD/src-tauri/icons/128x128@2x.png", info.repo)),
        cover: None,
        screenshots: Vec::new(),
        links,
        release_notes: ReleaseNotesConf { from: None, languages: Vec::new() },
        windows: None,
        linux: None,
        discovered: true,
    };
    catalog::fill_defaults(&mut def);
    Some(def)
}

/// Bulunan uygulamaları kataloğun sonuna ekler. Katalogda aynı depo (ya da aynı kimlik) varsa katalogdaki kalır:
/// bir uygulama kataloğa elle eklenince açıklaması, görselleri ve kurulum bilgileri oradan gelir.
pub fn merge(mut catalog: Catalog, found: &[AppDef]) -> Catalog {
    catalog.apps.retain(|a| !a.discovered);
    let mut repos: HashSet<String> = catalog.apps.iter().map(|a| a.repo.to_ascii_lowercase()).collect();
    let mut ids: HashSet<String> = catalog.apps.iter().map(|a| a.id.clone()).collect();
    for def in found {
        if repos.insert(def.repo.to_ascii_lowercase()) && ids.insert(def.id.clone()) {
            catalog.apps.push(def.clone());
        }
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::Asset;
    use serde_json::json;

    fn repo(name: &str, extra: Value) -> Value {
        let mut base = json!({
            "name": name,
            "full_name": format!("robz/{name}"),
            "owner": { "login": "robz" },
            "fork": false,
            "archived": false,
            "private": false,
            "description": "Açıklama",
            "homepage": "https://robz.example/app",
            "license": { "spdx_id": "MIT" },
        });
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        base
    }

    fn release(name: &str, version: &str, assets: &[&str]) -> Release {
        Release {
            id: 1,
            tag: format!("v{version}"),
            version: version.to_string(),
            name: name.to_string(),
            published_at: "2026-11-23T00:00:00Z".into(),
            prerelease: false,
            url: String::new(),
            body: String::new(),
            assets: assets
                .iter()
                .map(|n| Asset { name: n.to_string(), size: 1, url: String::new(), sha256: None, downloads: 0 })
                .collect(),
        }
    }

    #[test]
    fn keeps_only_own_public_robz_repos() {
        let list = json!([
            repo("robz-dpi", json!({})),
            repo("robz-hub", json!({})),
            repo("Robz-Upper", json!({})),
            repo("robz-fork", json!({ "fork": true })),
            repo("robz-old", json!({ "archived": true })),
            repo("robz-other", json!({ "owner": { "login": "someone" } })),
            repo("noobgg", json!({})),
            repo("robz-bad", json!({ "homepage": "http://insecure.example", "license": { "spdx_id": "NOASSERTION" } })),
        ]);
        let found = candidates(&list, "robz", "robz/robz-hub");
        assert_eq!(stars(&json!([{ "full_name": "Robz/Robz-Play", "stargazers_count": 7 }]))["robz/robz-play"], 7);
        let names: Vec<&str> = found.iter().map(|r| r.repo.as_str()).collect();
        assert_eq!(names, ["robz/robz-dpi", "robz/Robz-Upper", "robz/robz-bad"]);
        assert_eq!(found[0].homepage.as_deref(), Some("https://robz.example/app"));
        assert_eq!(found[2].homepage, None);
        assert_eq!(found[2].license, None);
    }

    #[test]
    fn names_apps_from_release_or_repo() {
        assert_eq!(display_name(&[release("Robz DPI 0.2.0", "0.2.0", &[])], "robz/robz-dpi"), "Robz DPI");
        assert_eq!(display_name(&[release("Robz Vision v1.6.0", "1.6.0", &[])], "robz/robz-vision"), "Robz Vision");
        assert_eq!(display_name(&[release("v0.3.0", "0.3.0", &[])], "robz/robz-new-tool"), "Robz New Tool");
        assert_eq!(display_name(&[], "robz/robz-dpi"), "Robz Dpi");
    }

    #[test]
    fn every_robz_repo_is_an_app() {
        let info = RepoInfo { repo: "robz/robz-dpi".into(), description: Some("DPI aracı".into()), homepage: None, license: None };
        // Henüz sürümü ya da kurulum dosyası olmayan depo da listede görünür.
        let bare = app_def(&info, &[]).expect("sürümsüz depo da uygulama");
        assert_eq!(bare.name, "Robz Dpi");
        let def = app_def(&info, &[release("Robz DPI 0.1.0", "0.1.0", &["RobzDPI-Setup-0.1.0.exe", "SHA256SUMS.txt"])]).unwrap();
        assert_eq!(def.id, "robz-dpi");
        assert_eq!(def.name, "Robz DPI");
        assert_eq!(def.tagline.tr, "DPI aracı");
        assert!(def.discovered);
        let windows = def.windows.as_ref().unwrap();
        assert_eq!(windows.installer, catalog::InstallerType::Auto);
        assert!(windows.asset_re.is_match("RobzDPI-Setup.exe"));
        assert!(windows.asset_re.is_match("robz-new_install-x64.exe"));
        assert!(!windows.asset_re.is_match("RobzDPI-Portable.exe"));
        let linux = def.linux.as_ref().unwrap();
        assert_eq!((linux.package.as_deref(), linux.exe.as_deref()), (Some("robz-dpi"), Some("robz-dpi")));
        assert!(linux.deb_re.as_ref().unwrap().is_match("RobzDPI-0.2.0-amd64.deb"));
        assert!(!linux.rpm_re.as_ref().unwrap().is_match("RobzDPI-0.2.0-aarch64.rpm"));
        assert!(linux.appimage_re.as_ref().unwrap().is_match("Robz_DPI_0.3.0_amd64.AppImage"));
    }

    #[test]
    fn catalog_entries_win_over_discovered_ones() {
        let catalog = catalog::parse_str(catalog::BUNDLED).unwrap();
        let info = |repo: &str| RepoInfo { repo: repo.into(), description: None, homepage: None, license: None };
        let assets = [release("Robz X 1.0.0", "1.0.0", &["RobzX-Setup.exe"])];
        let found = vec![
            app_def(&info("kutsalkiremitci/robz-dpi"), &assets).unwrap(),
            app_def(&info("kutsalkiremitci/robz-new"), &assets).unwrap(),
        ];
        let merged = merge(catalog.clone(), &found);
        let ids: Vec<&str> = merged.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["robz-play", "robz-vision", "robz-dpi", "robz-new"]);
        assert!(!merged.app("robz-dpi").unwrap().discovered);
        assert!(merged.app("robz-new").unwrap().discovered);
        // Yeniden birleştirmek eskileri çoğaltmaz.
        assert_eq!(merge(merged, &found).apps.len(), 4);
    }
}
