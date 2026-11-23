//! Robz Hub hangi sistemde çalışıyor ve o sistemde hangi kurulum dosyası kullanılır.
//!
//! Windows'ta her uygulamanın kurulum dosyası (.exe) vardır. Linux'ta yalnız katalogda "linux" bölümü olan uygulamalar
//! kurulabilir; dosya, sistemin paket biçimine göre seçilir (Debian/Ubuntu: .deb, Fedora/openSUSE: .rpm). Diğerleri
//! arayüzde "Linux'ta desteklenmiyor" görünür.

use crate::catalog::{AppDef, HubConf};
use regex::Regex;
use serde::Serialize;

/// "windows" | "linux" | "other"
pub const OS: &str = if cfg!(windows) {
    "windows"
} else if cfg!(target_os = "linux") {
    "linux"
} else {
    "other"
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub enum LinuxFormat {
    Deb,
    Rpm,
}

/// Sistemin paket biçimi: dpkg veritabanı varsa .deb, rpm veritabanı varsa .rpm. Linux dışında None.
pub fn linux_format() -> Option<LinuxFormat> {
    #[cfg(target_os = "linux")]
    {
        use std::path::Path;
        use std::sync::OnceLock;
        static FORMAT: OnceLock<Option<LinuxFormat>> = OnceLock::new();
        *FORMAT.get_or_init(|| {
            if Path::new("/var/lib/dpkg/status").is_file() {
                Some(LinuxFormat::Deb)
            } else if Path::new("/var/lib/rpm").is_dir() || Path::new("/usr/lib/sysimage/rpm").is_dir() {
                Some(LinuxFormat::Rpm)
            } else {
                None
            }
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Robz Hub bir AppImage olarak mı çalışıyor (AppImage çalışırken APPIMAGE değişkenine kendi yolunu yazar).
pub fn appimage_path() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()).map(std::path::PathBuf::from)
    } else {
        None
    }
}

/// Bu sistemde uygulamanın kurulum dosyasını seçen ifade; kurulamıyorsa None.
pub fn app_asset(def: &AppDef) -> Option<&Regex> {
    // Testler her sistemde Windows davranışıyla çalışır; sisteme göre seçim app_asset_for ile ayrıca sınanır.
    let os = if cfg!(test) { "windows" } else { OS };
    app_asset_for(def, os, linux_format())
}

pub fn app_asset_for<'a>(def: &'a AppDef, os: &str, format: Option<LinuxFormat>) -> Option<&'a Regex> {
    match os {
        "windows" => Some(&def.windows.asset_re),
        "linux" => {
            let linux = def.linux.as_ref()?;
            match format? {
                LinuxFormat::Deb => linux.deb_re.as_ref(),
                LinuxFormat::Rpm => linux.rpm_re.as_ref(),
            }
        }
        _ => None,
    }
}

/// Robz Hub'ın kendi güncellemesinin dosyası: Windows'ta kurulum programı, Linux'ta AppImage ya da sistem paketi.
pub fn hub_asset(hub: &HubConf) -> Option<&Regex> {
    match OS {
        "windows" => Some(&hub.asset_re),
        "linux" if appimage_path().is_some() => hub.linux.appimage.as_ref(),
        "linux" => match linux_format()? {
            LinuxFormat::Deb => hub.linux.deb.as_ref(),
            LinuxFormat::Rpm => hub.linux.rpm.as_ref(),
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn picks_the_package_for_the_system() {
        let catalog = catalog::parse_str(catalog::BUNDLED).unwrap();
        let play = catalog.app("robz-play").unwrap();
        let dpi = catalog.app("robz-dpi").unwrap();
        assert!(app_asset_for(play, "windows", None).is_some());
        assert!(app_asset_for(play, "linux", Some(LinuxFormat::Deb)).is_none());
        assert!(app_asset_for(dpi, "linux", Some(LinuxFormat::Deb)).unwrap().is_match("RobzDPI-0.2.0-amd64.deb"));
        assert!(app_asset_for(dpi, "linux", Some(LinuxFormat::Rpm)).unwrap().is_match("RobzDPI-0.2.0-x86_64.rpm"));
        assert!(app_asset_for(dpi, "linux", None).is_none());
        assert!(app_asset_for(dpi, "other", None).is_none());
    }
}
