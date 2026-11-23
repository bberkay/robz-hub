//! Robz Hub hangi sistemde çalışıyor ve bir uygulama bu sistemde nasıl kurulur.
//!
//! Windows'ta uygulamanın kurulum programı (.exe) kullanılır. Linux'ta önce dağıtımın kendi paket biçimi denenir
//! (Debian/Ubuntu ailesinde .deb, Fedora/openSUSE ailesinde .rpm); o yoksa ya da dağıtım başka bir aileden geliyorsa
//! (Arch, Manjaro ve diğerleri) AppImage kullanılır. Hiçbiri yoksa uygulama "desteklenmiyor" görünür ve nedeni
//! söylenir: bu işletim sistemi için hiç sürümü yok mu, yoksa Linux sürümü var da bu dağıtımda kurulamıyor mu.

use crate::catalog::{AppDef, HubConf};
use crate::github::{self, Release};
use crate::installs::InstallInfo;
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

/// Uygulamanın bu sistemde kurulma yolu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// Windows kurulum programı (Inno Setup ya da NSIS)
    Windows,
    /// Linux sistem paketi (pkexec ile paket yöneticisi)
    Deb,
    Rpm,
    /// Linux'ta kullanıcının kendi klasörüne, parola istemeden
    AppImage,
}

/// Bu sistemde kurulamama nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Unsupported {
    /// Uygulamanın bu işletim sistemi için hiç sürümü yok.
    Os,
    /// Linux sürümü var ama bu dağıtımın kurabileceği biçimde değil (ör. Arch'ta yalnız .deb/.rpm).
    Distro,
}

#[derive(Debug, Clone, Copy)]
pub struct Plan<'a> {
    pub asset: &'a Regex,
    pub method: Method,
}

// ---- Linux dağıtımı ---------------------------------------------------------------------------------------------------

/// /etc/os-release'deki ID ve ID_LIKE değerlerine göre dağıtım ailesi.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Deb,
    Rpm,
    /// Dağıtım belli ama .deb/.rpm ailesinden değil (Arch, Manjaro, NixOS, Void, Gentoo, Alpine…)
    Other,
    /// os-release okunamadı
    Unknown,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const DEB_IDS: &[&str] = &[
    "debian", "ubuntu", "linuxmint", "mint", "pop", "elementary", "zorin", "kali", "raspbian", "neon", "deepin", "pureos",
    "devuan", "mx", "parrot", "tuxedo",
];
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const RPM_IDS: &[&str] = &[
    "fedora", "rhel", "centos", "rocky", "almalinux", "ol", "opensuse", "suse", "sles", "mageia", "openmandriva", "nobara",
    "amzn", "ultramarine",
];

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn family_from_os_release(text: &str) -> Family {
    let mut ids: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim() == "ID" || key.trim() == "ID_LIKE" {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            ids.extend(value.split_whitespace().map(str::to_ascii_lowercase));
        }
    }
    if ids.is_empty() {
        Family::Unknown
    } else if ids.iter().any(|id| DEB_IDS.contains(&id.as_str())) {
        Family::Deb
    } else if ids.iter().any(|id| RPM_IDS.contains(&id.as_str()) || id.starts_with("opensuse")) {
        Family::Rpm
    } else {
        Family::Other
    }
}

/// Sistemin paket biçimi. Dağıtım ailesi os-release'den okunur, paket veritabanının varlığıyla doğrulanır; Arch gibi
/// başka ailelerde (rpm aracı kurulu olsa bile) None. Linux dışında None.
pub fn linux_format() -> Option<LinuxFormat> {
    #[cfg(target_os = "linux")]
    {
        use std::path::Path;
        use std::sync::OnceLock;
        static FORMAT: OnceLock<Option<LinuxFormat>> = OnceLock::new();
        *FORMAT.get_or_init(|| {
            let release = ["/etc/os-release", "/usr/lib/os-release"]
                .iter()
                .find_map(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            let dpkg = Path::new("/var/lib/dpkg/status").is_file();
            let rpm = Path::new("/var/lib/rpm").is_dir() || Path::new("/usr/lib/sysimage/rpm").is_dir();
            match family_from_os_release(&release) {
                Family::Deb => dpkg.then_some(LinuxFormat::Deb),
                Family::Rpm => rpm.then_some(LinuxFormat::Rpm),
                Family::Other => None,
                Family::Unknown if dpkg => Some(LinuxFormat::Deb),
                Family::Unknown if rpm => Some(LinuxFormat::Rpm),
                Family::Unknown => None,
            }
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Taşınabilir kipte verilerin klasörü (exe'nin yanında "RobzHub-Data"). Yalnız Windows'ta: exe'nin adında "portable"
/// geçiyorsa (RobzHub-Portable.exe) ya da yanında "portable" adlı bir dosya varsa. Kurulu sürümde None.
pub fn portable_dir() -> Option<std::path::PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    portable_dir_for(&exe, |p| p.exists())
}

pub fn portable_dir_for(exe: &std::path::Path, exists: impl Fn(&std::path::Path) -> bool) -> Option<std::path::PathBuf> {
    let dir = exe.parent()?;
    let named = exe.file_stem().is_some_and(|s| s.to_string_lossy().to_ascii_lowercase().contains("portable"));
    (named || exists(&dir.join("portable"))).then(|| dir.join("RobzHub-Data"))
}

/// Robz Hub bir AppImage olarak mı çalışıyor (AppImage çalışırken APPIMAGE değişkenine kendi yolunu yazar).
pub fn appimage_path() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()).map(std::path::PathBuf::from)
    } else {
        None
    }
}

// ---- kurulum planı ----------------------------------------------------------------------------------------------------

/// Uygulama bu sistemde nasıl kurulur (ya da neden kurulamaz). Karar sürümdeki dosyalara göre verilir: bir yol
/// ancak son kararlı sürümlerden birinde o dosya varsa seçilir. Kuruluysa aynı yoldan güncellenir: AppImage olarak
/// kurulmuş bir uygulama paket olarak, paket olarak kurulmuş bir uygulama AppImage olarak yeniden kurulmaz.
/// Henüz hiç sürüm yoksa (ya da sürüm listesi alınamadıysa) yalnız yapılandırmaya bakılır; arayüz "henüz sürüm yok" der.
pub fn plan<'a>(def: &'a AppDef, install: &InstallInfo, releases: &[Release]) -> Result<Plan<'a>, Unsupported> {
    // Testler her sistemde Windows davranışıyla çalışır; sisteme göre seçim plan_for ile ayrıca sınanır.
    let os = if cfg!(test) { "windows" } else { OS };
    plan_for(def, install.source, os, linux_format(), releases)
}

pub fn plan_for<'a>(
    def: &'a AppDef,
    installed_from: Option<&str>,
    os: &str,
    format: Option<LinuxFormat>,
    releases: &[Release],
) -> Result<Plan<'a>, Unsupported> {
    let released = releases.iter().any(|r| !r.prerelease);
    let in_releases = |re: &Regex| github::pick_latest(releases, re).is_some();
    let usable = |re: &Regex| !released || in_releases(re);
    match os {
        "windows" => def
            .windows
            .as_ref()
            .filter(|w| usable(&w.asset_re))
            .map(|w| Plan { asset: &w.asset_re, method: Method::Windows })
            .ok_or(Unsupported::Os),
        "linux" => {
            let Some(linux) = &def.linux else { return Err(Unsupported::Os) };
            let pick = |re: Option<&'a Regex>, method| re.filter(|re| usable(re)).map(|asset| Plan { asset, method });
            let package = match format {
                Some(LinuxFormat::Deb) => pick(linux.deb_re.as_ref(), Method::Deb),
                Some(LinuxFormat::Rpm) => pick(linux.rpm_re.as_ref(), Method::Rpm),
                None => None,
            };
            let appimage = pick(linux.appimage_re.as_ref(), Method::AppImage);
            let chosen = if installed_from == Some("appimage") { appimage.or(package) } else { package.or(appimage) };
            chosen.ok_or_else(|| {
                // Linux için bir dosya var ama bu dağıtımın kurabileceği biçimde değil mi, yoksa hiç yok mu?
                let any = [&linux.deb_re, &linux.rpm_re, &linux.appimage_re]
                    .into_iter()
                    .any(|re| re.as_ref().is_some_and(|re| in_releases(re)));
                if any {
                    Unsupported::Distro
                } else {
                    Unsupported::Os
                }
            })
        }
        _ => Err(Unsupported::Os),
    }
}

/// Robz Hub'ın kendi güncellemesinin dosyası: Windows'ta kurulum programı (taşınabilirde RobzHub-Portable.exe),
/// Linux'ta AppImage ya da sistem paketi.
pub fn hub_asset(hub: &HubConf) -> Option<&Regex> {
    match OS {
        "windows" if portable_dir().is_some() => hub.portable.as_ref(),
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

    // Taşınabilir kip yalnız Windows'ta (yollar Windows kurallarıyla)
    #[cfg(windows)]
    #[test]
    fn detects_the_portable_build() {
        use std::path::{Path, PathBuf};
        let no_marker = |_: &Path| false;
        assert_eq!(
            portable_dir_for(Path::new(r"D:\Araçlar\RobzHub-Portable.exe"), no_marker),
            Some(PathBuf::from(r"D:\Araçlar\RobzHub-Data"))
        );
        assert_eq!(portable_dir_for(Path::new(r"C:\Users\a\AppData\Local\Robz Hub\RobzHub.exe"), no_marker), None);
        // Adı değiştirilse de yanındaki "portable" dosyası taşınabilir kipte tutar.
        let marker = |p: &Path| p.ends_with("portable");
        assert!(portable_dir_for(Path::new(r"E:\RobzHub.exe"), marker).is_some());
    }

    #[test]
    fn reads_the_distribution_family() {
        let arch = "NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\n";
        let manjaro = "NAME=\"Manjaro Linux\"\nID=manjaro\nID_LIKE=arch\n";
        let ubuntu = "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n";
        let mint = "ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n";
        let fedora = "NAME=\"Fedora Linux\"\nID=fedora\n";
        let tumbleweed = "ID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n";
        assert_eq!(family_from_os_release(arch), Family::Other);
        assert_eq!(family_from_os_release(manjaro), Family::Other);
        assert_eq!(family_from_os_release(ubuntu), Family::Deb);
        assert_eq!(family_from_os_release(mint), Family::Deb);
        assert_eq!(family_from_os_release(fedora), Family::Rpm);
        assert_eq!(family_from_os_release(tumbleweed), Family::Rpm);
        assert_eq!(family_from_os_release(""), Family::Unknown);
    }

    fn release(files: &[&str]) -> Release {
        Release {
            id: 1,
            tag: "v1.0.0".into(),
            version: "1.0.0".into(),
            name: "1.0.0".into(),
            published_at: String::new(),
            prerelease: false,
            url: String::new(),
            body: String::new(),
            assets: files
                .iter()
                .map(|n| github::Asset { name: n.to_string(), size: 1, url: String::new(), sha256: None, downloads: 0 })
                .collect(),
        }
    }

    #[test]
    fn picks_how_to_install_from_the_release_files() {
        let catalog = catalog::parse_str(catalog::BUNDLED).unwrap();
        let play = catalog.app("robz-play").unwrap();
        let dpi = catalog.app("robz-dpi").unwrap();
        let method = |def, from, os, format, files: &[&str]| {
            plan_for(def, from, os, format, &[release(files)]).map(|p| p.method)
        };
        let (deb, rpm) = (Some(LinuxFormat::Deb), Some(LinuxFormat::Rpm));
        let windows_only = ["RobzPlay-Setup.exe"];
        let dpi_files = ["RobzDPI-Setup.exe", "RobzDPI-Linux-amd64.deb", "RobzDPI-Linux-x86_64.rpm"];

        assert_eq!(method(play, None, "windows", None, &windows_only), Ok(Method::Windows));
        assert_eq!(method(play, None, "linux", deb, &windows_only), Err(Unsupported::Os));
        assert_eq!(method(dpi, None, "linux", deb, &dpi_files), Ok(Method::Deb));
        assert_eq!(method(dpi, None, "linux", rpm, &dpi_files), Ok(Method::Rpm));
        // Arch: Linux sürümü var ama yalnız .deb/.rpm
        assert_eq!(method(dpi, None, "linux", None, &dpi_files), Err(Unsupported::Distro));
        assert_eq!(method(dpi, None, "other", None, &dpi_files), Err(Unsupported::Os));

        // Sürüme AppImage eklenince katalog değişmeden: Arch'ta AppImage, Ubuntu'da yine .deb; AppImage olarak
        // kuruluysa AppImage'la güncellenir.
        let with_appimage = ["RobzDPI-Setup.exe", "RobzDPI-Linux-amd64.deb", "RobzDPI-0.3.0-x86_64.AppImage"];
        assert_eq!(method(dpi, None, "linux", None, &with_appimage), Ok(Method::AppImage));
        assert_eq!(method(dpi, None, "linux", deb, &with_appimage), Ok(Method::Deb));
        assert_eq!(method(dpi, Some("appimage"), "linux", deb, &with_appimage), Ok(Method::AppImage));
        // Fedora'da .rpm yok ama AppImage var
        assert_eq!(method(dpi, None, "linux", rpm, &with_appimage), Ok(Method::AppImage));

        // Yalnız Linux'ta çalışan uygulama: Windows'ta desteklenmez.
        assert_eq!(method(play, None, "windows", None, &["RobzPlay-1.0.0-amd64.AppImage"]), Err(Unsupported::Os));
        assert_eq!(method(play, None, "linux", None, &["RobzPlay-1.0.0-amd64.AppImage"]), Ok(Method::AppImage));

        // Henüz sürüm yoksa yapılandırmaya göre (arayüz "henüz sürüm yok" der).
        assert_eq!(plan_for(play, None, "windows", None, &[]).map(|p| p.method), Ok(Method::Windows));
    }
}
