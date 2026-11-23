//! Linux'ta uygulama paketleri (.deb / .rpm).
//!
//! Kurulu sürüm paket veritabanından okunur: dpkg'nin durum dosyası doğrudan, rpm ise `rpm -q` ile (veritabanı
//! değişmedikçe yeniden sorulmaz; 5 sn'lik denetim ucuz kalır). Kurma ve kaldırma sistemin paket yöneticisine
//! (apt, dnf, zypper ya da rpm) yaptırılır; yönetici yetkisini pkexec ister: masaüstünün parola penceresi açılır,
//! Robz Hub parolayı görmez.
//!
//! Kod her sistemde derlenir (testler Windows'ta da çalışır); yalnız Linux'ta kullanılır.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use crate::catalog::AppDef;
use crate::error::{HubError, HubResult};
use crate::installs::InstallInfo;
use crate::platform::{linux_format, LinuxFormat};
use crate::version;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const DPKG_STATUS: &str = "/var/lib/dpkg/status";
const RPM_DBS: [&str; 2] = ["/var/lib/rpm", "/usr/lib/sysimage/rpm"];

/// PATH içinde (ya da standart yerlerde) bir program.
pub fn find_program(name: &str) -> Option<PathBuf> {
    let dirs = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    dirs.into_iter()
        .chain(["/usr/bin", "/usr/local/bin", "/usr/sbin", "/bin", "/sbin"].map(PathBuf::from))
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// "1:0.2.0-1" -> "0.2.0": dönem (epoch) ve paket revizyonu atılır.
pub fn package_version(raw: &str) -> Option<String> {
    let text = raw.trim();
    let text = text.split_once(':').map_or(text, |(_, rest)| rest);
    let text = text.split_once('-').map_or(text, |(upstream, _)| upstream);
    version::clean(text)
}

// ---- kurulu sürüm -----------------------------------------------------------------------------------------------------

/// dpkg durum dosyasından kurulu paketler: ad -> sürüm. Dosya değişmedikçe yeniden okunmaz.
fn dpkg_installed() -> HashMap<String, String> {
    static CACHE: Mutex<Option<(SystemTime, HashMap<String, String>)>> = Mutex::new(None);
    let modified = std::fs::metadata(DPKG_STATUS).and_then(|m| m.modified()).ok();
    let mut cache = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if let (Some(modified), Some((at, map))) = (modified, cache.as_ref()) {
        if *at == modified {
            return map.clone();
        }
    }
    let map = std::fs::read_to_string(DPKG_STATUS).map(|text| parse_dpkg_status(&text)).unwrap_or_default();
    if let Some(modified) = modified {
        *cache = Some((modified, map.clone()));
    }
    map
}

pub fn parse_dpkg_status(text: &str) -> HashMap<String, String> {
    let mut installed = HashMap::new();
    for paragraph in text.split("\n\n") {
        let (mut name, mut status, mut ver) = (None, None, None);
        for line in paragraph.lines() {
            if let Some(v) = line.strip_prefix("Package: ") {
                name = Some(v.trim());
            } else if let Some(v) = line.strip_prefix("Status: ") {
                status = Some(v.trim());
            } else if let Some(v) = line.strip_prefix("Version: ") {
                ver = Some(v.trim());
            }
        }
        if let (Some(name), Some("install ok installed"), Some(ver)) = (name, status, ver) {
            installed.insert(name.to_string(), ver.to_string());
        }
    }
    installed
}

fn rpm_db_modified() -> Option<SystemTime> {
    RPM_DBS.iter().filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok()).max()
}

/// `rpm -q` sonucu; rpm veritabanı değişmedikçe aynı paket yeniden sorulmaz.
fn rpm_version(package: &str) -> Option<String> {
    static CACHE: Mutex<Option<(Option<SystemTime>, HashMap<String, Option<String>>)>> = Mutex::new(None);
    let modified = rpm_db_modified();
    {
        let mut cache = CACHE.lock().unwrap_or_else(|p| p.into_inner());
        match cache.as_ref() {
            Some((at, map)) if *at == modified && modified.is_some() => {
                if let Some(found) = map.get(package) {
                    return found.clone();
                }
            }
            _ => *cache = Some((modified, HashMap::new())),
        }
    }
    let output = std::process::Command::new("rpm").args(["-q", "--qf", "%{VERSION}", package]).output().ok();
    let found = output.filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let found = found.filter(|v| !v.is_empty());
    if let Some((_, map)) = CACHE.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
        map.insert(package.to_string(), found.clone());
    }
    found
}

fn installed_version(package: &str) -> Option<String> {
    match linux_format()? {
        LinuxFormat::Deb => dpkg_installed().remove(package),
        LinuxFormat::Rpm => rpm_version(package),
    }
}

pub fn detect(def: &AppDef) -> InstallInfo {
    let Some(linux) = &def.linux else { return InstallInfo::default() };
    let Some(raw) = installed_version(&linux.package) else { return InstallInfo::default() };
    let exe = find_program(&linux.exe);
    InstallInfo {
        installed: true,
        version: package_version(&raw),
        dir: exe.as_deref().and_then(Path::parent).map(|d| d.display().to_string()),
        exe_path: exe.map(|p| p.display().to_string()),
        uninstaller: None,
        scope: Some("machine"),
        source: Some("package"),
    }
}

// ---- kurma ve kaldırma ------------------------------------------------------------------------------------------------

/// Paket yöneticisi komutu (yönetici yetkisiyle çalışacak program ve argümanları).
fn install_command(file: &Path) -> HubResult<(PathBuf, Vec<String>)> {
    let file = file.display().to_string();
    let found = |name: &str| find_program(name);
    let command = match linux_format().ok_or_else(|| HubError::new("unsupported"))? {
        LinuxFormat::Deb => match found("apt-get") {
            // apt bağımlılıkları da kurar; dosya yolu tam verilmeli.
            Some(apt) => (apt, vec!["install".into(), "-y".into(), "--allow-downgrades".into(), file]),
            None => (found("dpkg").ok_or_else(|| HubError::new("unsupported"))?, vec!["-i".into(), file]),
        },
        LinuxFormat::Rpm => {
            if let Some(dnf) = found("dnf") {
                (dnf, vec!["install".into(), "-y".into(), file])
            } else if let Some(zypper) = found("zypper") {
                (zypper, vec!["--non-interactive".into(), "install".into(), "--allow-unsigned-rpm".into(), file])
            } else {
                (found("rpm").ok_or_else(|| HubError::new("unsupported"))?, vec!["-U".into(), "--replacepkgs".into(), file])
            }
        }
    };
    Ok(command)
}

fn remove_command(package: &str) -> HubResult<(PathBuf, Vec<String>)> {
    let package = package.to_string();
    let found = |name: &str| find_program(name);
    let command = match linux_format().ok_or_else(|| HubError::new("unsupported"))? {
        LinuxFormat::Deb => match found("apt-get") {
            Some(apt) => (apt, vec!["remove".into(), "-y".into(), package]),
            None => (found("dpkg").ok_or_else(|| HubError::new("unsupported"))?, vec!["-r".into(), package]),
        },
        LinuxFormat::Rpm => {
            if let Some(dnf) = found("dnf") {
                (dnf, vec!["remove".into(), "-y".into(), package])
            } else if let Some(zypper) = found("zypper") {
                (zypper, vec!["--non-interactive".into(), "remove".into(), package])
            } else {
                (found("rpm").ok_or_else(|| HubError::new("unsupported"))?, vec!["-e".into(), package])
            }
        }
    };
    Ok(command)
}

/// Komutu pkexec ile yönetici olarak çalıştırır. pkexec ortamı temizlediği için apt'nin soru sormaması
/// (DEBIAN_FRONTEND) env ile verilir.
async fn run_as_admin(code: &str, program: PathBuf, args: Vec<String>) -> HubResult<()> {
    let pkexec = find_program("pkexec").ok_or_else(|| HubError::with(code, "pkexec bulunamadı"))?;
    let env = find_program("env").unwrap_or_else(|| PathBuf::from("/usr/bin/env"));
    let mut command = tokio::process::Command::new(pkexec);
    command.arg(env).arg("DEBIAN_FRONTEND=noninteractive").arg(program).args(&args).kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(20 * 60), command.output())
        .await
        .map_err(|_| HubError::with(code, "zaman aşımı"))?
        .map_err(|e| HubError::with(code, e))?;
    match output.status.code() {
        Some(0) => Ok(()),
        // 126: parola penceresi kapatıldı, 127: yetki verilmedi
        Some(126) | Some(127) => Err(HubError::new("auth-cancelled")),
        other => {
            let detail = String::from_utf8_lossy(&output.stderr);
            let last = detail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default().trim().to_string();
            Err(HubError::with(code, format!("çıkış kodu {}: {last}", other.unwrap_or(-1))))
        }
    }
}

pub async fn install(file: &Path) -> HubResult<()> {
    let (program, args) = install_command(file)?;
    run_as_admin("installer-failed", program, args).await
}

pub async fn uninstall(def: &AppDef) -> HubResult<()> {
    let linux = def.linux.as_ref().ok_or_else(|| HubError::new("unsupported"))?;
    let (program, args) = remove_command(&linux.package)?;
    run_as_admin("uninstall-failed", program, args).await?;
    for _ in 0..20 {
        if installed_version(&linux.package).is_none() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(HubError::with("uninstall-failed", "paket hâlâ kurulu"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_installed_packages_from_dpkg_status() {
        let status = "Package: robz-dpi\nStatus: install ok installed\nVersion: 0.2.0\n\n\
                      Package: removed\nStatus: deinstall ok config-files\nVersion: 1.0\n\n\
                      Package: libc6\nStatus: install ok installed\nVersion: 2.36-9+deb12u4\n";
        let installed = parse_dpkg_status(status);
        assert_eq!(installed.get("robz-dpi").map(String::as_str), Some("0.2.0"));
        assert!(!installed.contains_key("removed"));
        assert_eq!(package_version("1:0.2.0-1").as_deref(), Some("0.2.0"));
        assert_eq!(package_version("2.36-9+deb12u4").as_deref(), Some("2.36"));
    }
}
