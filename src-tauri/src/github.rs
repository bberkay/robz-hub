//! GitHub sürümleri: bir deponun yayınlanmış sürümlerini okur ve diske önbellekler.
//!
//! Kimlik doğrulamasız API saatte 60 istekle sınırlıdır (aynı ağdaki bütün cihazlar için ortak). Her yanıtın ETag'i
//! saklanır ve sonraki istekte If-None-Match ile gönderilir: değişiklik yoksa GitHub boş bir 304 döner. Kimliksiz
//! isteklerde 304 de sınırdan düşer (ölçüldü); kazanç veri boyutundadır, bu yüzden denetimler seyrek tutulur. Hata, sınır
//! ya da internet yokken önbellekteki son liste kullanılır. Kişisel bilgi gönderilmez; User-Agent yalnız
//! "RobzHub/<sürüm>"dür.

use crate::version;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API: &str = "https://api.github.com";
const PER_PAGE: u32 = 30;
const MAX_BODY_CHARS: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
    pub sha256: Option<String>,
    /// GitHub'ın saydığı indirme sayısı (download_count).
    #[serde(default)]
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: u64,
    pub tag: String,
    pub version: String,
    pub name: String,
    pub published_at: String,
    pub prerelease: bool,
    pub url: String,
    pub body: String,
    pub assets: Vec<Asset>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    etag: Option<String>,
    saved_at: u64,
    releases: Vec<Release>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchStatus {
    Ok,
    Cached,
    Offline,
    Limited,
    Missing,
    Error,
}

impl FetchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Cached => "cached",
            Self::Offline => "offline",
            Self::Limited => "limited",
            Self::Missing => "missing",
            Self::Error => "error",
        }
    }
}

pub struct FetchResult {
    pub releases: Vec<Release>,
    pub status: FetchStatus,
    /// Oran sınırının kalkacağı zaman (ms).
    pub retry_at: Option<u64>,
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn cache_file(cache_dir: &Path, repo: &str) -> PathBuf {
    // ".v2": indirme sayılarıyla birlikte saklanan biçim. Eski önbellek okunmaz; ilk denetimde baştan alınır.
    cache_dir.join(format!("{}.v2.json", repo.replace('/', "__")))
}

fn read_cache(file: &Path) -> Option<CacheFile> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// Açılışta ağ beklemeden gösterebilmek için önbellekteki liste.
pub fn cached_releases(cache_dir: &Path, repo: &str) -> Vec<Release> {
    read_cache(&cache_file(cache_dir, repo)).map(|c| c.releases).unwrap_or_default()
}

pub fn slim_release(raw: &Value) -> Option<Release> {
    if raw["draft"].as_bool() == Some(true) {
        return None;
    }
    let tag = raw["tag_name"].as_str()?.to_string();
    let version = version::clean(&tag)?;
    let str_of = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let body: String = raw["body"].as_str().unwrap_or_default().chars().take(MAX_BODY_CHARS).collect();
    let assets = raw["assets"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    let digest = a["digest"].as_str().unwrap_or_default();
                    let sha256 = digest
                        .strip_prefix("sha256:")
                        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
                        .map(str::to_ascii_lowercase);
                    Some(Asset {
                        name: a["name"].as_str()?.to_string(),
                        size: a["size"].as_u64().unwrap_or(0),
                        url: a["browser_download_url"].as_str()?.to_string(),
                        sha256,
                        downloads: a["download_count"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let published = raw["published_at"].as_str().or_else(|| raw["created_at"].as_str()).unwrap_or_default();
    Some(Release {
        id: raw["id"].as_u64().unwrap_or(0),
        name: raw["name"].as_str().filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or_else(|| tag.clone()),
        tag,
        version,
        published_at: published.to_string(),
        prerelease: raw["prerelease"].as_bool().unwrap_or(false),
        url: str_of(&raw["html_url"]),
        body,
        assets,
    })
}

pub async fn fetch_releases(client: &reqwest::Client, repo: &str, cache_dir: &Path) -> FetchResult {
    let file = cache_file(cache_dir, repo);
    let cached = read_cache(&file);
    let fallback = |status| FetchResult {
        releases: cached.as_ref().map(|c| c.releases.clone()).unwrap_or_default(),
        status,
        retry_at: None,
    };

    let mut request = client
        .get(format!("{API}/repos/{repo}/releases?per_page={PER_PAGE}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .timeout(Duration::from_secs(10));
    if let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_deref()) {
        request = request.header("If-None-Match", etag);
    }

    let res = match request.send().await {
        Ok(res) => res,
        Err(_) => return fallback(FetchStatus::Offline),
    };
    let status = res.status().as_u16();
    if status == 304 && cached.is_some() {
        return fallback(FetchStatus::Cached);
    }
    if status == 403 || status == 429 {
        let header = |name: &str| res.headers().get(name).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
        let now = now_ms();
        let reset = header("x-ratelimit-reset").map(|s| s * 1000).filter(|r| *r > now);
        let retry_after = header("retry-after").map(|s| now + s * 1000);
        let mut result = fallback(FetchStatus::Limited);
        result.retry_at = Some(reset.or(retry_after).unwrap_or(now + 15 * 60 * 1000));
        return result;
    }
    if status == 404 {
        return FetchResult { releases: Vec::new(), status: FetchStatus::Missing, retry_at: None };
    }
    if !res.status().is_success() {
        return fallback(FetchStatus::Error);
    }

    let etag = res.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_string);
    let list: Value = match res.text().await.ok().and_then(|t| serde_json::from_str(&t).ok()) {
        Some(v) => v,
        None => return fallback(FetchStatus::Error),
    };
    let Some(list) = list.as_array() else { return fallback(FetchStatus::Error) };
    let mut releases: Vec<Release> = list.iter().filter_map(slim_release).collect();
    releases.sort_by(|a, b| version::compare(&b.version, &a.version));

    let _ = std::fs::create_dir_all(cache_dir);
    let cache = CacheFile { etag, saved_at: now_ms(), releases };
    if let Ok(text) = serde_json::to_string(&cache) {
        let _ = std::fs::write(&file, text);
    }
    FetchResult { releases: cache.releases, status: FetchStatus::Ok, retry_at: None }
}

/// Kurulabilir en yeni kararlı sürüm: ön sürüm değil ve kurulum dosyası (asset) var.
pub fn pick_latest<'a>(releases: &'a [Release], asset: &Regex) -> Option<(&'a Release, &'a Asset)> {
    releases
        .iter()
        .filter(|r| !r.prerelease)
        .find_map(|r| r.assets.iter().find(|a| asset.is_match(&a.name)).map(|a| (r, a)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release(tag: &str, prerelease: bool, assets: &[&str]) -> Release {
        slim_release(&json!({
            "id": 1,
            "tag_name": tag,
            "name": "",
            "prerelease": prerelease,
            "published_at": "2026-11-21T16:43:11Z",
            "html_url": "https://github.com/o/r/releases/tag/x",
            "body": "notes",
            "assets": assets.iter().map(|n| json!({
                "name": n, "size": 10, "browser_download_url": format!("https://github.com/o/r/releases/download/{tag}/{n}"),
                "digest": "sha256:066107C57DEC73B39F344388A439A5A487DC7BFC471490BE493840656747ABAD",
                "download_count": 21
            })).collect::<Vec<_>>()
        }))
        .unwrap()
    }

    #[test]
    fn slims_release_and_digest() {
        let r = release("v1.6.0", false, &["RobzVision-Setup-1.6.0-x64.exe"]);
        assert_eq!(r.version, "1.6.0");
        assert_eq!(r.name, "v1.6.0");
        assert_eq!(r.assets[0].downloads, 21);
        // Eski önbellekte indirme sayısı yoktur: 0 sayılır.
        let old: Asset = serde_json::from_str(r#"{"name":"a.exe","size":1,"url":"u","sha256":null}"#).unwrap();
        assert_eq!(old.downloads, 0);
        assert_eq!(
            r.assets[0].sha256.as_deref(),
            Some("066107c57dec73b39f344388a439a5a487dc7bfc471490be493840656747abad")
        );
        assert!(slim_release(&json!({ "tag_name": "v1.0.0", "draft": true })).is_none());
        assert!(slim_release(&json!({ "tag_name": "nightly" })).is_none());
    }

    #[test]
    fn picks_newest_stable_with_installer() {
        let re = Regex::new(r"^App-Setup-[0-9.]+\.exe$").unwrap();
        let releases = vec![
            release("v2.0.0-rc1", true, &["App-Setup-2.0.0.exe"]),
            release("v1.9.0", false, &["App-Portable-1.9.0.zip"]),
            release("v1.8.0", false, &["App-Setup-1.8.0.exe"]),
            release("v1.7.0", false, &["App-Setup-1.7.0.exe"]),
        ];
        let (r, a) = pick_latest(&releases, &re).unwrap();
        assert_eq!(r.version, "1.8.0");
        assert_eq!(a.name, "App-Setup-1.8.0.exe");
        assert!(pick_latest(&[], &re).is_none());
    }
}
