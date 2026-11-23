//! Küçük bir günlük dosyası: veri klasöründe "robz-hub.log".
//!
//! Açılış bilgisi (sürüm, sistem, kurulum biçimi), işlerin sonucu (kur, güncelle, kaldır; hata ayrıntısıyla), GitHub
//! denetimindeki sorunlar ve arayüzde yakalanmayan hatalar yazılır. Bir sorun bildirilirken eklenir; Ayarlar'daki
//! "Günlük dosyasını göster" klasörünü açar. Dosya 256 KB'ı geçince eski yarısı atılır. Hiçbir yere gönderilmez;
//! kullanıcının ev klasörü satırlarda "~" olarak geçer.

use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 256 * 1024;

static FILE: OnceLock<PathBuf> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn init(file: PathBuf) {
    let _ = FILE.set(file);
}

pub fn path() -> Option<&'static Path> {
    FILE.get().map(PathBuf::as_path)
}

/// "2026-11-23 15:04:05" (UTC). Takvim hesabı: Howard Hinnant'ın gün sayısından tarihe çevirme yöntemi.
pub fn timestamp(unix_seconds: u64) -> String {
    let (days, rest) = (unix_seconds / 86_400, unix_seconds % 86_400);
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}", rest / 3_600, rest % 3_600 / 60, rest % 60)
}

/// Satırdaki ev klasörünü "~" yapar ve satır sonlarını boşluğa çevirir.
pub fn scrub(line: &str, home: Option<&str>) -> String {
    let line = line.replace(['\n', '\r'], " ");
    match home.filter(|h| h.len() > 1) {
        Some(home) => line.replace(home, "~"),
        None => line,
    }
}

pub fn write(line: impl Display) {
    let Some(file) = FILE.get() else { return };
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let text: String = scrub(&line.to_string(), home.as_deref()).chars().take(2_000).collect();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Dosya büyüdüyse yalnız son yarısı tutulur.
    if std::fs::metadata(file).is_ok_and(|m| m.len() > MAX_BYTES) {
        if let Ok(old) = std::fs::read(file) {
            let keep = &old[old.len() - (MAX_BYTES / 2) as usize..];
            let start = keep.iter().position(|b| *b == b'\n').map_or(0, |i| i + 1);
            let _ = std::fs::write(file, &keep[start..]);
        }
    }
    if let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(out, "{} {text}", timestamp(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_time_and_hides_the_home_folder() {
        assert_eq!(timestamp(0), "1970-01-01 00:00:00");
        assert_eq!(timestamp(1_795_446_245), "2026-11-23 15:04:05");
        assert_eq!(timestamp(1_709_251_199), "2024-02-29 23:59:59");
        assert_eq!(
            scrub("kuruldu: /home/ali/.local/share/robz-hub/apps/x\nikinci satır", Some("/home/ali")),
            "kuruldu: ~/.local/share/robz-hub/apps/x ikinci satır"
        );
        assert_eq!(scrub(r"C:\Users\Ali\AppData\Local\Robz DPI", Some(r"C:\Users\Ali")), r"~\AppData\Local\Robz DPI");
        assert_eq!(scrub("değişmez", None), "değişmez");
    }
}
