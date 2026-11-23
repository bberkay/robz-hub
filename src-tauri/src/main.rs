//! Robz Hub — Robz açık kaynak uygulamalarını tek yerden kur, güncelle ve başlat.
//!
//! Arayüz (ui/) WebView2'de çalışır ve bu süreçle tek bir komutla konuşur: hub_call(method, args).
//!
//! Geliştirme için ortam değişkenleri:
//!   ROBZHUB_DRY_RUN=1   kurulum/kaldırma programları çalıştırılmaz (indirme ve doğrulama gerçek); arayüz akışını
//!                       bilgisayara dokunmadan denemek için
//!   ROBZHUB_DATA_DIR    ayarlar, önbellek ve indirmeler için ayrı bir klasör
//!
//! Taşınabilir sürüm (RobzHub-Portable.exe): kurulmaz; bütün verileri exe'nin yanındaki RobzHub-Data klasöründe tutar
//! (platform::portable_dir).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
mod discover;
mod download;
mod error;
mod github;
mod hub;
mod installs;
mod linux;
mod log;
mod platform;
mod settings;
mod shell;
mod version;

use error::{HubError, HubResult};
use hub::{Hub, JobKind, HUB_ID};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt as _;

fn arg_str(args: &[Value], index: usize) -> HubResult<String> {
    args.get(index).and_then(Value::as_str).map(str::to_string).ok_or_else(|| HubError::new("bad-request"))
}

fn arg_close(args: &[Value], index: usize) -> bool {
    args.get(index).and_then(|o| o.get("closeIfRunning")).and_then(Value::as_bool).unwrap_or(false)
}

fn open_external(app: &AppHandle, url: &str) -> HubResult<()> {
    if !url.starts_with("https://") || url.chars().any(|c| c.is_whitespace() || c == '"') {
        return Err(HubError::new("bad-request"));
    }
    open_target(app, url, false)
}

/// Klasörü ya da bağlantıyı sistemin varsayılan programıyla açar. Linux'ta xdg-open temizlenmiş ortamla çağrılır
/// (linux::open_with_system); Windows'ta Tauri'nin açıcısı kullanılır.
fn open_target(app: &AppHandle, target: &str, is_path: bool) -> HubResult<()> {
    if cfg!(target_os = "linux") {
        return linux::open_with_system(target);
    }
    let opener = app.opener();
    let result = if is_path { opener.open_path(target, None::<&str>) } else { opener.open_url(target, None::<&str>) };
    result.map_err(|e| HubError::with("open-failed", e))
}

/// Robz Hub'ın kendi güncellemesi: kurulum sessizce yeni sürümü kurar, ardından Robz Hub yeniden açılır.
/// Kurulum bu süreç kapandıktan sonra başlasın diye kısa bir bekleme eklenir.
#[cfg(windows)]
fn run_self_update(setup: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe()?;
    if platform::portable_dir().is_some() {
        return replace_portable(&exe, setup);
    }
    let script = format!(
        "/d /s /c \"ping -n 3 127.0.0.1 >nul & \"{}\" /S & start \"\" \"{}\"\"",
        setup.display(),
        exe.display()
    );
    std::process::Command::new("cmd.exe").raw_arg(script).creation_flags(0x0800_0000).spawn().map(drop)
}

/// Taşınabilir sürümün güncellemesi: çalışan exe yeniden adlandırılır (Windows buna izin verir, silmeye vermez), yeni exe
/// aynı adla yerine konur ve bu süreç kapandıktan sonra açılır. Eski exe bir sonraki açılışta silinir.
#[cfg(windows)]
fn replace_portable(exe: &Path, new_exe: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    swap_exe(exe, new_exe)?;
    let script = format!("/d /s /c \"ping -n 3 127.0.0.1 >nul & start \"\" \"{}\"\"", exe.display());
    std::process::Command::new("cmd.exe").raw_arg(script).creation_flags(0x0800_0000).spawn().map(drop)
}

#[cfg_attr(not(windows), allow(dead_code))]
/// exe'yi new_exe ile değiştirir; eskisi "<ad>.exe.old" olarak kalır. Kopyalama başarısız olursa eskisi geri konur.
fn swap_exe(exe: &Path, new_exe: &Path) -> std::io::Result<()> {
    let old = old_exe(exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old)?;
    if let Err(error) = std::fs::copy(new_exe, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(error);
    }
    Ok(())
}

fn old_exe(exe: &Path) -> PathBuf {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    exe.with_file_name(name)
}

/// Linux: AppImage ise dosya yenisiyle değiştirilir; .deb/.rpm ise paket yöneticisi yeni paketi kurar (pkexec parola
/// penceresi). Ardından Robz Hub, bu süreç kapandıktan sonra yeniden açılır.
#[cfg(not(windows))]
async fn run_self_update_linux(file: &Path) -> HubResult<()> {
    let target = if let Some(appimage) = platform::appimage_path() {
        use std::os::unix::fs::PermissionsExt;
        let temp = appimage.with_extension("new");
        let io = |e: std::io::Error| HubError::with("installer-failed", e);
        std::fs::copy(file, &temp).map_err(io)?;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
        std::fs::rename(&temp, &appimage).map_err(io)?;
        appimage
    } else {
        linux::install(file).await?;
        linux::find_program("robz-hub").ok_or_else(|| HubError::with("installer-failed", "robz-hub bulunamadı"))?
    };
    let mut command = std::process::Command::new("sh");
    platform::clean_env(&mut command);
    command
        .args(["-c", "sleep 2; exec \"$0\"", &target.display().to_string()])
        .spawn()
        .map(drop)
        .map_err(|e| HubError::with("installer-failed", e))
}

async fn dispatch(app: &AppHandle, hub: &Arc<Hub>, method: &str, args: &[Value]) -> HubResult<Value> {
    match method {
        "getState" => return Ok(hub.snapshot()),
        "check" => hub.refresh(true).await,
        "install" => hub.enqueue(&arg_str(args, 0)?, JobKind::Install, arg_close(args, 1))?,
        "uninstall" => hub.enqueue(&arg_str(args, 0)?, JobKind::Uninstall, arg_close(args, 1))?,
        "installBundle" => hub.install_bundle(&arg_str(args, 0)?)?,
        "cancel" => hub.cancel(&arg_str(args, 0)?),
        "dismissJob" => hub.dismiss_job(&arg_str(args, 0)?),
        "launch" => hub.launch(&arg_str(args, 0)?)?,
        "launchBundle" => hub.launch_bundle(&arg_str(args, 0)?)?,
        "stop" => hub.stop(&arg_str(args, 0)?).await?,
        "openFolder" => {
            if let Some(dir) = hub.install_dir(&arg_str(args, 0)?) {
                open_target(app, &dir, true)?;
            }
        }
        "openExternal" => open_external(app, &arg_str(args, 0)?)?,
        "setSetting" => {
            let key = arg_str(args, 0)?;
            let value = args.get(1).cloned().unwrap_or(Value::Null);
            if hub.set_setting(&key, &value) && key == "launchAtLogin" {
                shell::apply_autostart(app, value.as_bool().unwrap_or(false));
            }
        }
        "uiLog" => log::write(format!("arayüz: {}", arg_str(args, 0)?)),
        "openLog" => {
            if let Some(dir) = log::path().and_then(Path::parent) {
                open_target(app, &dir.display().to_string(), true)?;
            }
        }
        "clearCache" => hub.clear_cache().await,
        "updateSelf" => {
            if let Some(setup) = hub.download_self_update().await? {
                if hub.dry_run {
                    hub.clear_job(HUB_ID);
                } else {
                    #[cfg(windows)]
                    run_self_update(&setup).map_err(|e| HubError::with("installer-failed", e))?;
                    #[cfg(not(windows))]
                    run_self_update_linux(&setup).await?;
                    app.exit(0);
                }
            }
        }
        _ => return Err(HubError::with("bad-request", method)),
    }
    Ok(Value::Null)
}

#[tauri::command]
async fn hub_call(app: AppHandle, hub: State<'_, Arc<Hub>>, method: String, args: Vec<Value>) -> Result<Value, HubError> {
    let hub = hub.inner().clone();
    dispatch(&app, &hub, &method, &args).await
}

fn main() {
    let dry_run = std::env::var("ROBZHUB_DRY_RUN").is_ok_and(|v| v == "1");
    let start_hidden = std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        // İkinci kez açılırsa yeni pencere yerine var olan öne gelir (ilk eklenen eklenti olmalı).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| shell::show_window(app)))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .invoke_handler(tauri::generate_handler![hub_call])
        .setup(move |app| {
            let portable = platform::portable_dir();
            let data_dir = match (std::env::var_os("ROBZHUB_DATA_DIR"), &portable) {
                (Some(dir), _) => PathBuf::from(dir),
                (None, Some(dir)) => dir.clone(),
                (None, None) => app.path().app_local_data_dir()?,
            };
            // Taşınabilir sürüm kendini güncellediyse eski exe artık silinebilir.
            if portable.is_some() {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::fs::remove_file(old_exe(&exe));
                }
            }
            log::init(data_dir.join("robz-hub.log"));
            log::write(format!(
                "Robz Hub {} açıldı: {}{}{}{}",
                app.package_info().version,
                platform::OS,
                platform::linux_format().map(|f| format!(", paket biçimi {f:?}")).unwrap_or_default(),
                if platform::appimage_path().is_some() { ", AppImage" } else { "" },
                if portable.is_some() { ", taşınabilir" } else { "" },
            ));
            let (hub, jobs) = Hub::new(app.handle().clone(), data_dir, dry_run)?;
            app.manage(hub.clone());
            hub.refresh_installs();
            tauri::async_runtime::spawn(hub.clone().worker(jobs));

            // Linux'ta tepsi için appindicator kitaplığı gerekir; yoksa Robz Hub tepsisiz çalışır.
            if let Err(error) = shell::create_tray(app.handle(), &hub) {
                eprintln!("tepsi simgesi oluşturulamadı: {error}");
            }
            // Windows açılışında (--hidden) yalnız tepside başlar.
            if !start_hidden {
                shell::create_window(app.handle())?;
            }
            shell::start_timers(hub.clone());
            tauri::async_runtime::spawn(async move { hub.refresh(false).await });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Robz Hub başlatılamadı");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_the_portable_exe() {
        let dir = std::env::temp_dir().join(format!("robzhub-swap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (exe, new_exe) = (dir.join("RobzHub-Portable.exe"), dir.join("indirilen.exe"));
        std::fs::write(&exe, b"0.1.7").unwrap();
        std::fs::write(&new_exe, b"0.1.8").unwrap();
        swap_exe(&exe, &new_exe).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"0.1.8");
        assert_eq!(std::fs::read(dir.join("RobzHub-Portable.exe.old")).unwrap(), b"0.1.7");
        // Yeni dosya yoksa eski exe yerinde kalır.
        assert!(swap_exe(&exe, &dir.join("yok.exe")).is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"0.1.8");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
