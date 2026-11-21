//! Kurulum dosyasını indirir ve doğrular.
//!
//! Yalnızca ilgili deponun GitHub sürüm adresinden (https://github.com/<depo>/releases/download/…) indirilir. Dosya
//! önce "<ad>.part" olarak yazılır; boyutu API'deki boyutla, SHA-256 özeti API'nin verdiği özetle aynıysa asıl adına
//! taşınır. Tutmazsa silinir ve hiç çalıştırılmaz. Yarım indirme sürdürülmez: her deneme baştan alır.

use crate::error::{HubError, HubResult};
use crate::github::Asset;
use futures_util::StreamExt;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

static FILE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9 _.()-]{0,120}$").unwrap());

/// Bir takılmada beklenecek en uzun süre: bu kadar süre hiç veri gelmezse indirme bırakılır.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, Default)]
pub struct Progress {
    pub received: u64,
    pub total: u64,
    pub speed: f64,
}

pub fn check_release_url(url: &str, repo: &str) -> HubResult<()> {
    let prefix = format!("https://github.com/{repo}/releases/download/");
    if url.starts_with(&prefix) && !url.contains("..") && !url.contains('?') && !url.contains('#') {
        Ok(())
    } else {
        Err(HubError::with("download-failed", "beklenmeyen adres"))
    }
}

/// Dosya adı sürüm kaydından gelir; klasör dışına çıkamasın.
pub fn safe_file_name(name: &str) -> HubResult<&str> {
    if FILE_NAME.is_match(name) && !name.contains("..") {
        Ok(name)
    } else {
        Err(HubError::with("download-failed", "geçersiz dosya adı"))
    }
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn io_error(error: std::io::Error) -> HubError {
    // ERROR_HANDLE_DISK_FULL (39), ERROR_DISK_FULL (112)
    if matches!(error.raw_os_error(), Some(39 | 112)) {
        HubError::new("disk-full")
    } else {
        HubError::with("download-failed", error)
    }
}

/// Dönüş: (dosya yolu, özet karşılaştırıldı mı).
pub async fn download_asset(
    client: &reqwest::Client,
    asset: &Asset,
    repo: &str,
    dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(Progress),
) -> HubResult<(PathBuf, bool)> {
    check_release_url(&asset.url, repo)?;
    let name = safe_file_name(&asset.name)?;
    let file = dir.join(name);
    let part = dir.join(format!("{name}.part"));
    tokio::fs::create_dir_all(dir).await.map_err(io_error)?;

    // Önceki denemeden kalan tam dosya, özeti tutuyorsa yeniden indirilmez.
    if let Some(expected) = &asset.sha256 {
        if tokio::fs::metadata(&file).await.is_ok_and(|m| m.len() == asset.size) {
            let path = file.clone();
            if let Ok(Ok(hash)) = tokio::task::spawn_blocking(move || hash_file(&path)).await {
                if &hash == expected {
                    return Ok((file, true));
                }
            }
        }
    }
    let _ = tokio::fs::remove_file(&file).await;
    let _ = tokio::fs::remove_file(&part).await;

    let res = client.get(&asset.url).send().await.map_err(|e| HubError::with("offline", e))?;
    if !res.status().is_success() {
        return Err(HubError::with("download-failed", format!("HTTP {}", res.status().as_u16())));
    }
    let total = if asset.size > 0 { asset.size } else { res.content_length().unwrap_or(0) };

    let streamed: HubResult<(u64, String)> = async {
        let mut out = tokio::fs::File::create(&part).await.map_err(io_error)?;
        let mut hasher = Sha256::new();
        let mut stream = res.bytes_stream();
        let (mut received, mut last_bytes, mut speed) = (0u64, 0u64, 0f64);
        let mut last_tick = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(HubError::new("cancelled"));
            }
            let next = tokio::time::timeout(STALL_TIMEOUT, stream.next())
                .await
                .map_err(|_| HubError::with("download-failed", "bağlantı yanıt vermiyor"))?;
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|e| HubError::with("download-failed", e))?;
            hasher.update(&chunk);
            received += chunk.len() as u64;
            out.write_all(&chunk).await.map_err(io_error)?;
            let elapsed = last_tick.elapsed();
            if elapsed >= Duration::from_millis(200) {
                let instant = (received - last_bytes) as f64 / elapsed.as_secs_f64();
                speed = if speed > 0.0 { speed * 0.7 + instant * 0.3 } else { instant };
                last_tick = Instant::now();
                last_bytes = received;
                on_progress(Progress { received, total, speed });
            }
        }
        out.flush().await.map_err(io_error)?;
        on_progress(Progress { received, total, speed: 0.0 });
        Ok((received, hex::encode(hasher.finalize())))
    }
    .await;

    let fail = |error: HubError| async {
        let _ = tokio::fs::remove_file(&part).await;
        Err(error)
    };
    let (received, hash) = match streamed {
        Ok(v) => v,
        Err(error) => return fail(error).await,
    };
    if asset.size > 0 && received != asset.size {
        return fail(HubError::with("size-mismatch", format!("{received} / {}", asset.size))).await;
    }
    if let Some(expected) = &asset.sha256 {
        if &hash != expected {
            return fail(HubError::new("checksum-mismatch")).await;
        }
    }
    tokio::fs::rename(&part, &file).await.map_err(io_error)?;
    Ok((file, asset.sha256.is_some()))
}

pub fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(t) if t.is_dir() => dir_size(&entry.path()),
            _ => entry.metadata().map(|m| m.len()).unwrap_or(0),
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_release_urls_of_the_repo() {
        let repo = "kutsalkiremitci/robz-play";
        assert!(check_release_url("https://github.com/kutsalkiremitci/robz-play/releases/download/v0.4.0/RobzPlay-Setup.exe", repo).is_ok());
        assert!(check_release_url("https://github.com/someone/else/releases/download/v1/x.exe", repo).is_err());
        assert!(check_release_url("http://github.com/kutsalkiremitci/robz-play/releases/download/v1/x.exe", repo).is_err());
        assert!(check_release_url("https://github.com/kutsalkiremitci/robz-play/releases/download/../../x.exe", repo).is_err());
        assert!(check_release_url("https://evil.example/?https://github.com/kutsalkiremitci/robz-play/releases/download/", repo).is_err());
    }

    #[test]
    fn file_names_stay_in_folder() {
        assert!(safe_file_name("RobzVision-Setup-1.6.0-x64.exe").is_ok());
        assert!(safe_file_name("..\\..\\evil.exe").is_err());
        assert!(safe_file_name("a/b.exe").is_err());
        assert!(safe_file_name("").is_err());
    }

    #[test]
    fn hashes_files() {
        let dir = std::env::temp_dir().join(format!("robzhub-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.txt");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(hash_file(&file).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(dir_size(&dir), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
