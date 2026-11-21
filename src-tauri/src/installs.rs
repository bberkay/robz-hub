//! Bilgisayardaki kurulumları bulur, çalışan uygulamaları izler, kurulum/kaldırma programlarını çalıştırır.
//!
//! Algılama iki adımlıdır:
//!   1. Windows "Yüklü uygulamalar" kaydı (Inno Setup ve NSIS kurulumları buraya yazar): klasör, sürüm, kaldırıcı.
//!   2. Kayıt yoksa uygulamanın varsayılan kurulum klasörü: exe oradaysa kurulu sayılır, sürüm exe'nin dosya
//!      bilgisinden okunur. (Kaydı silinmiş ya da elle kopyalanmış kurulumlar da böylece görünür.)
//! Robz Hub'ın dışında kurulmuş ya da uygulamanın kendi güncelleyicisiyle güncellenmiş kopyalar da aynı yoldan bulunur.

use crate::catalog::{AppDef, InstallerType};
use crate::error::{HubError, HubResult};
use crate::version;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub dir: Option<String>,
    pub exe_path: Option<String>,
    pub uninstaller: Option<String>,
    pub scope: Option<&'static str>,
    pub source: Option<&'static str>,
}

// Kurulum parametreleri türe göre sabittir; katalogdan parametre alınmaz.
fn install_args(kind: InstallerType) -> &'static [&'static str] {
    match kind {
        InstallerType::Inno => &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/NOCANCEL", "/NORESTARTAPPLICATIONS"],
        InstallerType::Nsis => &["/S"],
    }
}

fn uninstall_args(kind: InstallerType) -> &'static [&'static str] {
    match kind {
        InstallerType::Inno => &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"],
        InstallerType::Nsis => &["/S"],
    }
}

fn uninstaller_name(kind: InstallerType) -> &'static str {
    match kind {
        InstallerType::Inno => "unins000.exe",
        InstallerType::Nsis => "Uninstall.exe",
    }
}

/// "%LOCALAPPDATA%\Programs\Robz Play" -> C:\Users\…\AppData\Local\Programs\Robz Play
pub fn expand_dir(template: &str) -> Option<PathBuf> {
    let rest = template.strip_prefix('%')?;
    let end = rest.find('%')?;
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = match rest[..end].to_ascii_uppercase().as_str() {
        "LOCALAPPDATA" => var("LOCALAPPDATA"),
        "APPDATA" => var("APPDATA"),
        "PROGRAMFILES" => var("ProgramW6432").or_else(|| var("ProgramFiles")),
        "PROGRAMFILES(X86)" => var("ProgramFiles(x86)"),
        _ => None,
    }?;
    let mut path = PathBuf::from(base);
    for part in rest[end + 1..].split('\\').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return None;
        }
        path.push(part);
    }
    Some(path)
}

/// "C:\…\unins000.exe" /SILENT  ->  C:\…\unins000.exe
pub fn uninstaller_from_string(command: &str) -> Option<PathBuf> {
    let text = command.trim();
    if let Some(rest) = text.strip_prefix('"') {
        let path = &rest[..rest.find('"')?];
        return path.to_ascii_lowercase().ends_with(".exe").then(|| PathBuf::from(path));
    }
    let end = text.to_ascii_lowercase().find(".exe")? + 4;
    Some(PathBuf::from(&text[..end]))
}

/// file, dir klasörünün içinde mi (büyük/küçük harf duyarsız, ".." yok).
pub fn is_inside(file: &Path, dir: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let (file, dir) = (norm(file), norm(dir));
    !file.split('\\').any(|part| part == "..") && file.len() > dir.len() + 1 && file.starts_with(&format!("{dir}\\"))
}

fn is_file(path: &Path) -> bool {
    path.is_file()
}

fn find_uninstaller(dir: &Path, kind: InstallerType, hint: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(hint) = hint.filter(|h| is_file(h) && is_inside(h, dir)) {
        return Some(hint);
    }
    Some(dir.join(uninstaller_name(kind))).filter(|p| is_file(p))
}

// ---- Windows kayıt defteri, dosya sürümü, süreç listesi -------------------------------------------------------------

struct RegistryEntry {
    scope: &'static str,
    location: Option<String>,
    version: Option<String>,
    uninstall: Option<String>,
}

#[cfg(windows)]
fn registry_entries(key: &str) -> Vec<RegistryEntry> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    if !crate::catalog::SAFE_KEY.is_match(key) {
        return Vec::new();
    }
    let path = format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{key}");
    // Kullanıcıya kurulum (HKCU) önce; ardından tüm kullanıcılar için 64 ve 32 bit görünümler.
    let locations = [
        (HKEY_CURRENT_USER, 0, "user"),
        (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY, "machine"),
        (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY, "machine"),
    ];
    locations
        .into_iter()
        .filter_map(|(hive, view, scope)| {
            let key = RegKey::predef(hive).open_subkey_with_flags(&path, KEY_READ | view).ok()?;
            let get = |name: &str| key.get_value::<String, _>(name).ok().filter(|v| !v.trim().is_empty());
            Some(RegistryEntry {
                scope,
                location: get("InstallLocation").or_else(|| get("Inno Setup: App Path")),
                version: get("DisplayVersion"),
                uninstall: get("UninstallString"),
            })
        })
        .collect()
}

#[cfg(not(windows))]
fn registry_entries(_key: &str) -> Vec<RegistryEntry> {
    Vec::new()
}

/// exe'nin ürün sürümü (sürüm kaynağından). Yoksa ya da 0.0.0.0 ise None.
#[cfg(windows)]
pub fn file_version(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: tamponlar Windows'un istediği boyutta ayrılır; işaretçiler yalnız tampon yaşarken okunur.
    unsafe {
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, buf.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let root = [u16::from(b'\\'), 0];
        let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        if VerQueryValueW(buf.as_ptr().cast(), root.as_ptr(), &mut info, &mut len) == 0
            || info.is_null()
            || (len as usize) < std::mem::size_of::<VS_FIXEDFILEINFO>()
        {
            return None;
        }
        let fixed = &*(info as *const VS_FIXEDFILEINFO);
        if fixed.dwSignature != 0xFEEF_04BD {
            return None;
        }
        let (ms, ls) = (fixed.dwProductVersionMS, fixed.dwProductVersionLS);
        let text = format!("{}.{}.{}.{}", ms >> 16, ms & 0xffff, ls >> 16, ls & 0xffff);
        (text != "0.0.0.0").then(|| version::clean(&text)).flatten()
    }
}

#[cfg(not(windows))]
pub fn file_version(_path: &Path) -> Option<String> {
    None
}

/// Karşılaştırma için yol: küçük harf, ters eğik çizgi.
pub fn norm_path(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

/// Adı `names` içinde olan çalışan süreçler: tam exe yolu (norm_path) -> PID'ler.
///
/// Ad yetmez: aynı adla başka bir kopya çalışıyor olabilir (ör. geliştirme klasöründen açılmış bir derleme). Robz Hub
/// yalnız kurulu kopyayı "çalışıyor" sayar ve yalnız onu kapatır. Yolu okunamayan süreçler (başka kullanıcının ya da
/// sistemin) atlanır.
#[cfg(windows)]
pub fn running_paths(names: &[String]) -> HashMap<String, Vec<u32>> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let wanted: HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let mut found: HashMap<String, Vec<u32>> = HashMap::new();
    // SAFETY: tanıtıcılar yalnız bu blokta kullanılır ve kapatılır; tamponların boyutu çağrıya verilir.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return found;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
            if wanted.contains(&name) {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                if !process.is_null() {
                    let mut buf = [0u16; 1024];
                    let mut size = buf.len() as u32;
                    if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut size) != 0 {
                        let path = String::from_utf16_lossy(&buf[..size as usize]);
                        found.entry(norm_path(&path)).or_default().push(entry.th32ProcessID);
                    }
                    CloseHandle(process);
                }
            }
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    found
}

#[cfg(not(windows))]
pub fn running_paths(_names: &[String]) -> HashMap<String, Vec<u32>> {
    HashMap::new()
}

/// Bu exe yolundan çalışan süreçlerin PID'leri.
pub fn pids_of(exe_path: &str) -> Vec<u32> {
    let name = Path::new(exe_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    running_paths(&[name]).remove(&norm_path(exe_path)).unwrap_or_default()
}

/// Kurulu kopya çalışıyor mu?
pub fn is_running(info: &InstallInfo) -> bool {
    info.exe_path.as_deref().is_some_and(|exe| !pids_of(exe).is_empty())
}

// ---- algılama --------------------------------------------------------------------------------------------------------

pub fn detect(def: &AppDef) -> InstallInfo {
    let win = &def.windows;
    for entry in registry_entries(&win.uninstall_key) {
        let Some(location) = entry.location else { continue };
        let dir = PathBuf::from(location.trim().trim_matches('"'));
        let exe = dir.join(&win.exe);
        if !is_file(&exe) {
            continue; // kayıt kalmış ama dosyalar silinmiş
        }
        let hint = entry.uninstall.as_deref().and_then(uninstaller_from_string);
        return InstallInfo {
            installed: true,
            version: entry.version.as_deref().and_then(version::clean).or_else(|| file_version(&exe)),
            uninstaller: find_uninstaller(&dir, win.installer, hint).map(|p| p.display().to_string()),
            dir: Some(dir.display().to_string()),
            exe_path: Some(exe.display().to_string()),
            scope: Some(entry.scope),
            source: Some("registry"),
        };
    }

    if let Some(dir) = win.default_dir.as_deref().and_then(expand_dir) {
        let exe = dir.join(&win.exe);
        if is_file(&exe) {
            return InstallInfo {
                installed: true,
                version: file_version(&exe),
                uninstaller: find_uninstaller(&dir, win.installer, None).map(|p| p.display().to_string()),
                dir: Some(dir.display().to_string()),
                exe_path: Some(exe.display().to_string()),
                scope: Some("user"),
                source: Some("folder"),
            };
        }
    }
    InstallInfo::default()
}

// ---- çalıştırma -----------------------------------------------------------------------------------------------------

async fn taskkill(pids: &[u32], force: bool) {
    let mut command = tokio::process::Command::new("taskkill.exe");
    for pid in pids {
        command.args(["/PID", &pid.to_string()]);
    }
    if force {
        command.arg("/F");
    }
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let _ = tokio::time::timeout(Duration::from_secs(10), command.status()).await;
}

/// Kurulu kopyayı kapatır: önce penceresine kapanma isteği gönderilir; kapanmazsa (ör. tepsiye küçülüyorsa)
/// sonlandırılır. Aynı adla başka bir yerden çalışan kopyalara dokunulmaz.
pub async fn close_app(info: &InstallInfo) -> bool {
    let Some(exe) = info.exe_path.as_deref() else { return true };
    for force in [false, true] {
        let pids = pids_of(exe);
        if pids.is_empty() {
            return true;
        }
        taskkill(&pids, force).await;
        for _ in 0..12 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if pids_of(exe).is_empty() {
                return true;
            }
        }
    }
    false
}

async fn run_and_wait(file: &Path, args: &[&str], timeout: Duration) -> std::io::Result<Option<i32>> {
    let mut child = tokio::process::Command::new(file).args(args).kill_on_drop(true).spawn()?;
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => Ok(status?.code()),
        Err(_) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "zaman aşımı")),
    }
}

fn spawn_error(code: &str, error: std::io::Error) -> HubError {
    if error.raw_os_error() == Some(740) {
        HubError::with(code, "yönetici izni gerekiyor")
    } else {
        HubError::with(code, error)
    }
}

/// İndirilen kurulum dosyasını sessiz kipte çalıştırır; bitene kadar bekler.
pub async fn run_installer(def: &AppDef, file: &Path) -> HubResult<()> {
    let code = run_and_wait(file, install_args(def.windows.installer), Duration::from_secs(15 * 60))
        .await
        .map_err(|e| spawn_error("installer-failed", e))?;
    match code {
        Some(0) => Ok(()),
        other => Err(HubError::with("installer-failed", format!("çıkış kodu {}", other.unwrap_or(-1)))),
    }
}

/// Kaldırıcıyı sessiz kipte çalıştırır ve dosyalar gerçekten silinene kadar bekler. NSIS kaldırıcısı kendini geçici
/// klasöre kopyalayıp hemen döndüğü için çıkış koduna değil, exe'nin kaybolmasına bakılır.
pub async fn run_uninstaller(def: &AppDef, info: &InstallInfo) -> HubResult<()> {
    let (Some(uninstaller), Some(dir), Some(exe)) = (&info.uninstaller, &info.dir, &info.exe_path) else {
        return Err(HubError::new("uninstaller-missing"));
    };
    let (uninstaller, dir, exe) = (Path::new(uninstaller), Path::new(dir), Path::new(exe));
    if !is_file(uninstaller) || !is_inside(uninstaller, dir) {
        return Err(HubError::new("uninstaller-missing"));
    }
    run_and_wait(uninstaller, uninstall_args(def.windows.installer), Duration::from_secs(5 * 60))
        .await
        .map_err(|e| spawn_error("uninstall-failed", e))?;
    for _ in 0..120 {
        if !is_file(exe) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(HubError::with("uninstall-failed", "dosyalar silinmedi"))
}

pub fn launch(info: &InstallInfo) -> HubResult<()> {
    let (Some(exe), Some(dir)) = (&info.exe_path, &info.dir) else { return Err(HubError::new("not-installed")) };
    if !info.installed || !is_file(Path::new(exe)) {
        return Err(HubError::new("not-installed"));
    }
    let mut command = std::process::Command::new(exe);
    command.current_dir(dir);
    #[cfg(windows)]
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    command.spawn().map(drop).map_err(|e| spawn_error("launch-failed", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uninstall_strings() {
        assert_eq!(
            uninstaller_from_string(r#""C:\Apps\Robz Vision\unins000.exe" /SILENT"#),
            Some(PathBuf::from(r"C:\Apps\Robz Vision\unins000.exe"))
        );
        assert_eq!(uninstaller_from_string(r"C:\Apps\x\Uninstall.exe /S"), Some(PathBuf::from(r"C:\Apps\x\Uninstall.exe")));
        assert_eq!(uninstaller_from_string(r#""C:\Apps\x\run.bat""#), None);
        assert_eq!(uninstaller_from_string(""), None);
    }

    #[test]
    fn matches_running_copies_by_full_path() {
        let installed = r"C:\Users\a\AppData\Local\Programs\Robz Play\RobzPlay.exe";
        assert_eq!(norm_path(installed), norm_path(r"c:/users/A/appdata/local/programs/robz play/robzplay.exe"));
        // Aynı adla geliştirme klasöründen çalışan kopya kurulu kopya sayılmaz.
        assert_ne!(norm_path(installed), norm_path(r"C:\Users\a\Desktop\RobzPlay\dist\RobzPlay\RobzPlay.exe"));
        // Bu test sürecinin kendisi adıyla ve tam yoluyla bulunur.
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_string();
        let found = running_paths(&[name]);
        assert!(found.get(&norm_path(&me.to_string_lossy())).is_some_and(|pids| pids.contains(&std::process::id())));
    }

    #[test]
    fn checks_folder_containment() {
        let dir = Path::new(r"C:\Users\a\AppData\Local\Programs\Robz Play");
        assert!(is_inside(Path::new(r"C:\Users\a\AppData\Local\Programs\Robz Play\Uninstall.exe"), dir));
        assert!(is_inside(Path::new(r"c:\users\A\appdata\local\programs\robz play\uninstall.exe"), dir));
        assert!(!is_inside(Path::new(r"C:\Users\a\AppData\Local\Programs\Robz Player\x.exe"), dir));
        assert!(!is_inside(Path::new(r"C:\Users\a\AppData\Local\Programs\Robz Play\..\evil.exe"), dir));
        assert!(!is_inside(dir, dir));
    }

    #[test]
    fn expands_known_folders_only() {
        std::env::set_var("LOCALAPPDATA", r"C:\Users\a\AppData\Local");
        assert_eq!(
            expand_dir(r"%LOCALAPPDATA%\Programs\Robz Play"),
            Some(PathBuf::from(r"C:\Users\a\AppData\Local\Programs\Robz Play"))
        );
        assert_eq!(expand_dir(r"%WINDIR%\System32"), None);
        assert_eq!(expand_dir(r"%LOCALAPPDATA%\..\Roaming"), None);
        assert_eq!(expand_dir(r"C:\Windows"), None);
    }
}
