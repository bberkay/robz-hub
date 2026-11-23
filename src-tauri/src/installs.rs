//! Bilgisayardaki kurulumları bulur, çalışan uygulamaları izler, kurulum/kaldırma programlarını çalıştırır.
//!
//! Algılama iki adımlıdır:
//!   1. Windows "Yüklü uygulamalar" kaydı (Inno Setup ve NSIS kurulumları buraya yazar): klasör, sürüm, kaldırıcı.
//!   2. Kayıt yoksa uygulamanın varsayılan kurulum klasörü: exe oradaysa kurulu sayılır, sürüm exe'nin dosya
//!      bilgisinden okunur. (Kaydı silinmiş ya da elle kopyalanmış kurulumlar da böylece görünür.)
//! Robz Hub'ın dışında kurulmuş ya da uygulamanın kendi güncelleyicisiyle güncellenmiş kopyalar da aynı yoldan bulunur.

use crate::catalog::{AppDef, InstallerType};
use crate::platform::Method;
use crate::error::{HubError, HubResult};
use crate::version;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg_attr(not(windows), allow(dead_code))]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg_attr(not(windows), allow(dead_code))]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg_attr(not(windows), allow(dead_code))]
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

// Kurulum parametreleri türe göre sabittir; katalogdan parametre alınmaz. Türü bilinmeyen kurulum dosyası
// parametresiz (kullanıcının göreceği pencereyle) çalışır.
fn install_args(kind: InstallerType) -> &'static [&'static str] {
    match kind {
        InstallerType::Inno => &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/NOCANCEL", "/NORESTARTAPPLICATIONS"],
        InstallerType::Nsis => &["/S"],
        InstallerType::Auto => &[],
    }
}

fn uninstall_args(kind: InstallerType) -> &'static [&'static str] {
    match kind {
        InstallerType::Inno => &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"],
        InstallerType::Nsis => &["/S"],
        InstallerType::Auto => &[],
    }
}

fn uninstaller_name(kind: InstallerType) -> Option<&'static str> {
    match kind {
        InstallerType::Inno => Some("unins000.exe"),
        InstallerType::Nsis => Some("Uninstall.exe"),
        InstallerType::Auto => None,
    }
}

/// Kaldırıcının türü adından: Inno Setup'ınki her zaman unins000.exe (unins001.exe…), gerisi NSIS sayılır.
pub fn uninstaller_kind(path: &Path) -> InstallerType {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let digits = name.strip_prefix("unins").and_then(|rest| rest.strip_suffix(".exe"));
    if digits.is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())) {
        InstallerType::Inno
    } else {
        InstallerType::Nsis
    }
}

/// Kurulum dosyasının türü, içindeki imzadan: Inno Setup "Inno Setup Setup Data", NSIS "NullsoftInst".
/// İkisi de dosyanın başındaki birkaç megabaytta durur; bulunamazsa None.
pub fn sniff_installer(path: &Path) -> Option<InstallerType> {
    use std::io::Read;
    const INNO: &[u8] = b"Inno Setup Setup Data";
    const NSIS: &[u8] = b"NullsoftInst";
    const LIMIT: usize = 32 * 1024 * 1024;
    let mut file = std::fs::File::open(path).ok()?;
    let mut window: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 1024 * 1024];
    let mut read_total = 0usize;
    while read_total < LIMIT {
        let n = file.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        read_total += n;
        // Bir önceki parçanın sonu da aranır: imza iki parçaya bölünmüş olabilir.
        let keep = window.len().min(INNO.len());
        window.drain(..window.len() - keep);
        window.extend_from_slice(&chunk[..n]);
        if window.windows(INNO.len()).any(|w| w == INNO) {
            return Some(InstallerType::Inno);
        }
        if window.windows(NSIS.len()).any(|w| w == NSIS) {
            return Some(InstallerType::Nsis);
        }
    }
    None
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
    Some(dir.join(uninstaller_name(kind)?)).filter(|p| is_file(p))
}

// ---- Windows kayıt defteri, dosya sürümü, süreç listesi -------------------------------------------------------------

struct RegistryEntry {
    scope: &'static str,
    location: Option<String>,
    version: Option<String>,
    uninstall: Option<String>,
    /// Çoğu kurulum programı uygulamanın exe'sini yazar: "C:\…\App.exe",0
    display_icon: Option<String>,
    /// Tauri'nin NSIS kurulumu exe adını ayrıca yazar.
    main_binary: Option<String>,
}

// Kullanıcıya kurulum (HKCU) önce; ardından tüm kullanıcılar için 64 ve 32 bit görünümler.
#[cfg(windows)]
const UNINSTALL_ROOTS: [(winreg::HKEY, u32, &str); 3] = [
    (winreg::enums::HKEY_CURRENT_USER, 0, "user"),
    (winreg::enums::HKEY_LOCAL_MACHINE, winreg::enums::KEY_WOW64_64KEY, "machine"),
    (winreg::enums::HKEY_LOCAL_MACHINE, winreg::enums::KEY_WOW64_32KEY, "machine"),
];
#[cfg(windows)]
const UNINSTALL_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

#[cfg(windows)]
fn read_entry(key: &winreg::RegKey, scope: &'static str) -> RegistryEntry {
    let get = |name: &str| key.get_value::<String, _>(name).ok().filter(|v| !v.trim().is_empty());
    RegistryEntry {
        scope,
        location: get("InstallLocation").or_else(|| get("Inno Setup: App Path")),
        version: get("DisplayVersion"),
        uninstall: get("UninstallString"),
        display_icon: get("DisplayIcon"),
        main_binary: get("MainBinaryName"),
    }
}

#[cfg(windows)]
fn registry_entries(key: &str) -> Vec<RegistryEntry> {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    if !crate::catalog::SAFE_KEY.is_match(key) {
        return Vec::new();
    }
    let path = format!(r"{UNINSTALL_PATH}\{key}");
    UNINSTALL_ROOTS
        .into_iter()
        .filter_map(|(hive, view, scope)| {
            let key = RegKey::predef(hive).open_subkey_with_flags(&path, KEY_READ | view).ok()?;
            Some(read_entry(&key, scope))
        })
        .collect()
}

#[cfg(not(windows))]
fn registry_entries(_key: &str) -> Vec<RegistryEntry> {
    Vec::new()
}

/// Görünen adı aranan adlardan biriyle eşleşen kayıtlar (kendiliğinden bulunan uygulamalar için: kayıt anahtarı önceden
/// bilinmez). Ad yalnız harf ve rakamlarıyla karşılaştırılır; sonuna sürüm eklenmiş adlar da kabul edilir
/// ("Robz Vision version 1.6.0").
#[cfg(windows)]
fn registry_search(wanted: &[String]) -> Vec<RegistryEntry> {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    let mut found = Vec::new();
    for (hive, view, scope) in UNINSTALL_ROOTS {
        let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL_PATH, KEY_READ | view) else { continue };
        for name in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey_with_flags(&name, KEY_READ | view) else { continue };
            let Ok(display) = key.get_value::<String, _>("DisplayName") else { continue };
            if name_matches(&display, wanted) {
                found.push(read_entry(&key, scope));
            }
        }
    }
    found
}

#[cfg(not(windows))]
fn registry_search(_wanted: &[String]) -> Vec<RegistryEntry> {
    Vec::new()
}

#[cfg_attr(not(windows), allow(dead_code))]
/// "Robz DPI", "RobzDPI", "Robz Vision version 1.6.0" gibi adlar "robzdpi" / "robzvision" ile eşleşir.
pub fn name_matches(display: &str, wanted: &[String]) -> bool {
    let display = crate::discover::normalize(display);
    wanted.iter().filter(|w| !w.is_empty()).any(|w| {
        display.strip_prefix(w.as_str()).is_some_and(|rest| {
            let rest = rest.strip_prefix("version").or_else(|| rest.strip_prefix('v')).unwrap_or(rest);
            rest.chars().all(|c| c.is_ascii_digit())
        })
    })
}

/// Kendiliğinden bulunan bir uygulamanın kurulu exe'si: Tauri'nin yazdığı ad, DisplayIcon ya da klasördeki tek
/// (kaldırıcı olmayan) exe.
fn resolve_exe(dir: &Path, entry: &RegistryEntry, wanted: &[String]) -> Option<PathBuf> {
    let candidate = |path: PathBuf| (is_file(&path) && is_inside(&path, dir) && !is_uninstaller(&path)).then_some(path);
    if let Some(name) = entry.main_binary.as_deref().filter(|n| crate::catalog::EXE.is_match(n)) {
        if let Some(path) = candidate(dir.join(name)) {
            return Some(path);
        }
    }
    if let Some(icon) = entry.display_icon.as_deref().and_then(icon_path) {
        if let Some(path) = candidate(icon) {
            return Some(path);
        }
    }
    let exes: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) && !is_uninstaller(p))
        .collect();
    let stem = |p: &PathBuf| crate::discover::normalize(&p.file_stem().unwrap_or_default().to_string_lossy());
    exes.iter().find(|p| wanted.contains(&stem(p))).cloned().or_else(|| (exes.len() == 1).then(|| exes[0].clone()))
}

/// "C:\…\App.exe",0  ->  C:\…\App.exe
fn icon_path(value: &str) -> Option<PathBuf> {
    let text = value.trim();
    let text = match text.rsplit_once(',') {
        Some((path, index)) if index.trim().trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => path,
        _ => text,
    };
    let path = text.trim().trim_matches('"');
    path.to_ascii_lowercase().ends_with(".exe").then(|| PathBuf::from(path))
}

fn is_uninstaller(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    name.starts_with("unins") || name.starts_with("uninstall")
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

/// Karşılaştırma için yol. Windows'ta küçük harf ve ters eğik çizgi; Linux'ta yol olduğu gibi (büyük/küçük harf
/// duyarlı).
pub fn norm_path(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\").to_lowercase()
    } else {
        path.to_string()
    }
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

/// Linux: /proc/<pid>/exe bağlantısı, çalışan programın tam yolunu verir. AppImage'da program, AppImage'ın açtığı
/// geçici klasörden çalışır; onu AppImage'a bağlayan, ortamındaki APPIMAGE=<dosya> değişkenidir.
#[cfg(target_os = "linux")]
pub fn running_paths(names: &[String]) -> HashMap<String, Vec<u32>> {
    let wanted: HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let appimages = wanted.iter().any(|n| n.ends_with(".appimage"));
    let mut found: HashMap<String, Vec<u32>> = HashMap::new();
    let mut add = |path: &str, pid: u32| {
        let pids = found.entry(norm_path(path)).or_default();
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    };
    let name_of = |path: &str| Path::new(path).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let Ok(entries) = std::fs::read_dir("/proc") else { return found };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        if let Ok(exe) = std::fs::read_link(entry.path().join("exe")) {
            // Güncellenirken eski dosya silinmişse yolun sonuna " (deleted)" eklenir.
            let exe = exe.to_string_lossy().trim_end_matches(" (deleted)").to_string();
            if wanted.contains(&name_of(&exe)) {
                add(&exe, pid);
            }
        }
        if appimages {
            let Ok(environ) = std::fs::read(entry.path().join("environ")) else { continue };
            for var in environ.split(|b| *b == 0) {
                if let Some(value) = var.strip_prefix(b"APPIMAGE=") {
                    let file = String::from_utf8_lossy(value).to_string();
                    if wanted.contains(&name_of(&file)) {
                        add(&file, pid);
                    }
                }
            }
        }
    }
    found
}

#[cfg(not(any(windows, target_os = "linux")))]
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

/// Kendiliğinden bulunan uygulamalarda kayıtta aranan adlar: uygulamanın adı ve depo adı ("robzdpi").
pub fn wanted_names(def: &AppDef) -> Vec<String> {
    let slug = def.repo.rsplit('/').next().unwrap_or_default();
    let mut names = vec![crate::discover::normalize(&def.name), crate::discover::normalize(slug)];
    names.dedup();
    names
}

fn detect_by_name(def: &AppDef) -> InstallInfo {
    let wanted = wanted_names(def);
    for entry in registry_search(&wanted) {
        let Some(location) = entry.location.as_deref() else { continue };
        let dir = PathBuf::from(location.trim().trim_matches('"'));
        let Some(exe) = resolve_exe(&dir, &entry, &wanted) else { continue };
        let hint = entry.uninstall.as_deref().and_then(uninstaller_from_string);
        let uninstaller = hint.filter(|h| is_file(h) && is_inside(h, &dir));
        return InstallInfo {
            installed: true,
            version: entry.version.as_deref().and_then(version::clean).or_else(|| file_version(&exe)),
            uninstaller: uninstaller.map(|p| p.display().to_string()),
            dir: Some(dir.display().to_string()),
            exe_path: Some(exe.display().to_string()),
            scope: Some(entry.scope),
            source: Some("registry"),
        };
    }
    InstallInfo::default()
}

/// Uygulama kurulu mu, hangi sürüm, nerede. Linux'ta paket veritabanından (linux.rs).
pub fn detect(def: &AppDef) -> InstallInfo {
    if cfg!(target_os = "linux") {
        return crate::linux::detect(def);
    }
    detect_windows(def)
}

fn detect_windows(def: &AppDef) -> InstallInfo {
    let Some(win) = &def.windows else { return InstallInfo::default() };
    if win.installer == InstallerType::Auto {
        return detect_by_name(def);
    }
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

/// Süreçleri kapatır: önce kapanma isteği (Windows: pencereye, Linux: SIGTERM), force ise zorla.
async fn taskkill(pids: &[u32], force: bool) {
    let mut command = if cfg!(windows) {
        let mut command = tokio::process::Command::new("taskkill.exe");
        for pid in pids {
            command.args(["/PID", &pid.to_string()]);
        }
        if force {
            command.arg("/F");
        }
        command
    } else {
        let mut command = tokio::process::Command::new("kill");
        command.arg(if force { "-KILL" } else { "-TERM" });
        command.args(pids.iter().map(u32::to_string));
        command
    };
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
/// Linux'ta: sistem paketi pkexec ile paket yöneticisine kurdurulur, AppImage kullanıcının klasörüne yerleştirilir.
/// `icon`: AppImage'ın uygulama menüsü girişi için simge (içerik ve uzantı).
pub async fn run_installer(
    def: &AppDef,
    file: &Path,
    method: Method,
    version: &str,
    icon: Option<(Vec<u8>, &str)>,
) -> HubResult<()> {
    match method {
        Method::Deb | Method::Rpm => return crate::linux::install(file).await,
        Method::AppImage => return crate::linux::install_appimage(def, file, version, icon),
        Method::Windows => {}
    }
    let kind = match def.windows.as_ref().map_or(InstallerType::Auto, |w| w.installer) {
        InstallerType::Auto => sniff_installer(file).unwrap_or(InstallerType::Auto),
        kind => kind,
    };
    let code = run_and_wait(file, install_args(kind), Duration::from_secs(15 * 60))
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
    if cfg!(target_os = "linux") {
        return if info.source == Some("appimage") {
            crate::linux::uninstall_appimage(def)
        } else {
            crate::linux::uninstall(def).await
        };
    }
    let (Some(uninstaller), Some(dir), Some(exe)) = (&info.uninstaller, &info.dir, &info.exe_path) else {
        return Err(HubError::new("uninstaller-missing"));
    };
    let (uninstaller, dir, exe) = (Path::new(uninstaller), Path::new(dir), Path::new(exe));
    if !is_file(uninstaller) || !is_inside(uninstaller, dir) {
        return Err(HubError::new("uninstaller-missing"));
    }
    let kind = match def.windows.as_ref().map_or(InstallerType::Auto, |w| w.installer) {
        InstallerType::Auto => uninstaller_kind(uninstaller),
        kind => kind,
    };
    run_and_wait(uninstaller, uninstall_args(kind), Duration::from_secs(5 * 60))
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
    if cfg!(windows) {
        command.current_dir(dir);
    } else if let Some(home) = std::env::var_os("HOME") {
        command.current_dir(home);
    }
    // AppImage FUSE ile açılır; FUSE yoksa (bazı en küçük kurulumlarda) kendini geçici klasöre açıp çalışır.
    if exe.ends_with(".AppImage") && !Path::new("/dev/fuse").exists() {
        command.env("APPIMAGE_EXTRACT_AND_RUN", "1");
    }
    #[cfg(windows)]
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    let mut child = command.spawn().map_err(|e| spawn_error("launch-failed", e))?;
    // Linux'ta kapanan çocuk süreç "zombi" kalmasın diye beklenir (Windows'ta gerekmez, zararı da yok).
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
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

    // Windows yol kuralları (sürücü harfi, ters eğik çizgi, büyük/küçük harf duyarsızlığı)
    #[cfg(windows)]
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

    // Windows yol kuralları (sürücü harfi, ters eğik çizgi, büyük/küçük harf duyarsızlığı)
    #[cfg(windows)]
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

    #[test]
    fn matches_installed_names_loosely() {
        let wanted = vec!["robzdpi".to_string(), "robzvision".to_string()];
        assert!(name_matches("Robz DPI", &wanted));
        assert!(name_matches("RobzDPI", &wanted));
        assert!(name_matches("Robz Vision version 1.6.0", &wanted));
        assert!(name_matches("Robz Vision 1.6.0", &wanted));
        assert!(!name_matches("Robz DPI Helper", &wanted));
        assert!(!name_matches("Robz", &wanted));
        assert!(!name_matches("Robz Vision", &[String::new()]));
    }

    // Windows yol kuralları (sürücü harfi, ters eğik çizgi, büyük/küçük harf duyarsızlığı)
    #[cfg(windows)]
    #[test]
    fn reads_exe_from_display_icon() {
        assert_eq!(icon_path(r#""C:\Apps\Robz DPI\RobzDPI.exe",0"#), Some(PathBuf::from(r"C:\Apps\Robz DPI\RobzDPI.exe")));
        assert_eq!(icon_path(r"C:\Apps\X\X.exe"), Some(PathBuf::from(r"C:\Apps\X\X.exe")));
        assert_eq!(icon_path(r"C:\Apps\X\icon.ico,0"), None);
        assert_eq!(uninstaller_kind(Path::new(r"C:\Apps\X\unins000.exe")), InstallerType::Inno);
        assert_eq!(uninstaller_kind(Path::new(r"C:\Apps\X\uninstall.exe")), InstallerType::Nsis);
    }

    #[test]
    fn sniffs_installer_type() {
        let dir = std::env::temp_dir().join(format!("robzhub-sniff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, marker: &[u8]| {
            // İmza 1 MB'lık okuma parçasının sınırına denk gelsin: bölünmüş imza da bulunmalı.
            let mut bytes = vec![0u8; 1024 * 1024 - 5];
            bytes.extend_from_slice(marker);
            bytes.extend_from_slice(&[0u8; 64]);
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        assert_eq!(sniff_installer(&write("inno.exe", b"Inno Setup Setup Data (6.4.0)")), Some(InstallerType::Inno));
        assert_eq!(sniff_installer(&write("nsis.exe", b"\xEF\xBE\xAD\xDENullsoftInst")), Some(InstallerType::Nsis));
        assert_eq!(sniff_installer(&write("other.exe", b"MZ nothing here")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
