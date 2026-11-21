//! Robz Hub — Robz açık kaynak uygulamalarını tek yerden kur, güncelle ve başlat.
//!
//! Arayüz (ui/) WebView2'de çalışır ve bu süreçle tek bir komutla konuşur: hub_call(method, args).
//!
//! Geliştirme için ortam değişkenleri:
//!   ROBZHUB_DRY_RUN=1   kurulum/kaldırma programları çalıştırılmaz (indirme ve doğrulama gerçek); arayüz akışını
//!                       bilgisayara dokunmadan denemek için
//!   ROBZHUB_DATA_DIR    ayarlar, önbellek ve indirmeler için ayrı bir klasör

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
mod download;
mod error;
mod github;
mod hub;
mod installs;
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
    app.opener().open_url(url, None::<&str>).map_err(|e| HubError::with("unknown", e))
}

/// Robz Hub'ın kendi güncellemesi: kurulum sessizce yeni sürümü kurar, ardından Robz Hub yeniden açılır.
/// Kurulum bu süreç kapandıktan sonra başlasın diye kısa bir bekleme eklenir.
#[cfg(windows)]
fn run_self_update(setup: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe()?;
    let script = format!(
        "/d /s /c \"ping -n 3 127.0.0.1 >nul & \"{}\" /S & start \"\" \"{}\"\"",
        setup.display(),
        exe.display()
    );
    std::process::Command::new("cmd.exe").raw_arg(script).creation_flags(0x0800_0000).spawn().map(drop)
}

#[cfg(not(windows))]
fn run_self_update(_setup: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other("yalnız Windows"))
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
                app.opener().open_path(dir, None::<&str>).map_err(|e| HubError::with("unknown", e))?;
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
        "clearCache" => hub.clear_cache().await,
        "updateSelf" => {
            if let Some(setup) = hub.download_self_update().await? {
                if hub.dry_run {
                    hub.clear_job(HUB_ID);
                } else {
                    run_self_update(&setup).map_err(|e| HubError::with("installer-failed", e))?;
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
            let data_dir = match std::env::var_os("ROBZHUB_DATA_DIR") {
                Some(dir) => PathBuf::from(dir),
                None => app.path().app_local_data_dir()?,
            };
            let (hub, jobs) = Hub::new(app.handle().clone(), data_dir, dry_run)?;
            app.manage(hub.clone());
            hub.refresh_installs();
            tauri::async_runtime::spawn(hub.clone().worker(jobs));

            shell::create_tray(app.handle(), &hub)?;
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
