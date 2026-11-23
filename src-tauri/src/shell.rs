//! Pencere, tepsi simgesi, görev çubuğu ilerlemesi, Windows bildirimleri ve "Windows açılışında başlat".
//! İş mantığı hub.rs'tedir; burası onu masaüstüne bağlar.

use crate::hub::Hub;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::window::{Color, ProgressBarState, ProgressBarStatus};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_notification::NotificationExt as _;

const TRAY_ID: &str = "main";
const WINDOW: &str = "main";

struct Strings {
    open: &'static str,
    check: &'static str,
    quit: &'static str,
    launch: &'static str,
    update_title: &'static str,
}

fn strings(lang: &str) -> Strings {
    if lang == "en" {
        Strings { open: "Open Robz Hub", check: "Check for updates", quit: "Quit", launch: "Launch", update_title: "Update available" }
    } else {
        Strings { open: "Robz Hub'ı aç", check: "Güncellemeleri denetle", quit: "Çıkış", launch: "Başlat:", update_title: "Yeni sürüm var" }
    }
}

// ---- dil ------------------------------------------------------------------------------------------------------------

/// "Windows dili": görüntü dili, tercih edilen diller ya da bölge Türkiye ise Türkçe; değilse İngilizce.
/// (Türkiye'de İngilizce Windows kullanmak yaygındır; kullanıcı kenar çubuğundan tek tıkla değiştirebilir.)
#[cfg(windows)]
pub fn system_language() -> &'static str {
    use windows_sys::Win32::Globalization::{
        GetUserDefaultGeoName, GetUserDefaultLocaleName, GetUserPreferredUILanguages, MUI_LANGUAGE_NAME,
    };
    let mut texts = Vec::new();
    // SAFETY: her çağrıya yeterli boyutta tampon ve boyutu verilir.
    unsafe {
        let mut buf = [0u16; 85];
        let n = GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if n > 1 {
            texts.push(String::from_utf16_lossy(&buf[..(n - 1) as usize]));
        }
        let mut geo = [0u16; 16];
        let n = GetUserDefaultGeoName(geo.as_mut_ptr(), geo.len() as i32);
        if n > 1 && String::from_utf16_lossy(&geo[..(n - 1) as usize]).eq_ignore_ascii_case("TR") {
            return "tr";
        }
        let mut count = 0u32;
        let mut list = [0u16; 512];
        let mut len = list.len() as u32;
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, list.as_mut_ptr(), &mut len) != 0 {
            texts.extend(String::from_utf16_lossy(&list[..len as usize]).split('\0').map(str::to_string));
        }
    }
    if texts.iter().any(|t| t.to_ascii_lowercase().starts_with("tr")) {
        "tr"
    } else {
        "en"
    }
}

/// Linux: LANGUAGE, LC_ALL, LC_MESSAGES ya da LANG Türkçe ise ("tr_TR.UTF-8" gibi) Türkçe.
#[cfg(not(windows))]
pub fn system_language() -> &'static str {
    let turkish = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .filter(|v| !v.is_empty() && v != "C" && v != "POSIX")
        .flat_map(|v| v.split(':').map(str::to_string).collect::<Vec<_>>())
        .next()
        .is_some_and(|first| first.to_ascii_lowercase().starts_with("tr"));
    if turkish {
        "tr"
    } else {
        "en"
    }
}

// ---- pencere --------------------------------------------------------------------------------------------------------

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    } else if let Err(error) = create_window(app) {
        eprintln!("pencere açılamadı: {error}");
    }
}

pub fn create_window(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App("index.html".into()));
    // Taşınabilir sürümde WebView2'nin verisi de exe'nin yanında durur; bilgisayarda iz kalmaz.
    if let Some(dir) = crate::platform::portable_dir() {
        builder = builder.data_directory(dir);
    }
    let window = builder
        .title("Robz Hub")
        .inner_size(1240.0, 800.0)
        .min_inner_size(980.0, 640.0)
        .center()
        .theme(Some(tauri::Theme::Dark))
        .background_color(Color(13, 17, 23, 255))
        // Arayüz yalnızca kendi sayfasında kalır; bağlantılar varsayılan tarayıcıda açılır (ui/js/main.js).
        .on_navigation(|url| url.scheme() == "tauri" || url.host_str() == Some("tauri.localhost"))
        .build()?;

    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            let hub = handle.state::<Arc<Hub>>();
            // Tepsi simgesi yoksa (Linux'ta appindicator kurulu değilse) gizlenen pencereye dönülemez: kapanır.
            if hub.settings().close_to_tray && handle.tray_by_id(TRAY_ID).is_some() {
                api.prevent_close();
                if let Some(window) = handle.get_webview_window(WINDOW) {
                    let _ = window.hide();
                }
            }
        }
        WindowEvent::Focused(true) => handle.state::<Arc<Hub>>().inner().on_focus(),
        _ => {}
    });
    Ok(())
}

fn window_focused(app: &AppHandle) -> bool {
    app.get_webview_window(WINDOW).is_some_and(|w| w.is_focused().unwrap_or(false))
}

// ---- tepsi ----------------------------------------------------------------------------------------------------------

static TRAY_SIGNATURE: Mutex<String> = Mutex::new(String::new());

fn build_tray_menu(app: &AppHandle, hub: &Hub) -> tauri::Result<Menu<tauri::Wry>> {
    let t = strings(hub.language());
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "open", t.open, true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let installed = hub.installed_apps();
    for (id, name) in &installed {
        let label = format!("{} {name}", t.launch);
        menu.append(&MenuItem::with_id(app, format!("launch:{id}"), label, true, None::<&str>)?)?;
    }
    if !installed.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "check", t.check, true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", t.quit, true, None::<&str>)?)?;
    Ok(menu)
}

/// Linux'ta tepsi simgesini appindicator kitaplığı çizer; kitaplık yoksa Tauri çöker (panic = "abort"), bu yüzden
/// önce bakılır. AppImage kitaplığı kendi içinde taşır (APPDIR).
#[cfg(target_os = "linux")]
fn tray_available() -> bool {
    use std::path::PathBuf;
    let names = ["libayatana-appindicator3.so.1", "libappindicator3.so.1"];
    let mut dirs: Vec<PathBuf> =
        ["/usr/lib", "/usr/lib64", "/usr/lib/x86_64-linux-gnu", "/lib/x86_64-linux-gnu", "/usr/local/lib"]
            .map(PathBuf::from)
            .to_vec();
    if let Some(appdir) = std::env::var_os("APPDIR") {
        dirs.push(PathBuf::from(appdir).join("usr/lib"));
    }
    if let Some(paths) = std::env::var_os("LD_LIBRARY_PATH") {
        dirs.extend(std::env::split_paths(&paths));
    }
    dirs.iter().any(|dir| names.iter().any(|name| dir.join(name).exists()))
}

#[cfg(not(target_os = "linux"))]
fn tray_available() -> bool {
    true
}

pub fn create_tray(app: &AppHandle, hub: &Hub) -> tauri::Result<()> {
    if !tray_available() {
        eprintln!("tepsi kitaplığı (libayatana-appindicator3) yok: Robz Hub tepsisiz çalışıyor");
        return Ok(());
    }
    let menu = build_tray_menu(app, hub)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Robz Hub")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            let hub = app.state::<Arc<Hub>>().inner().clone();
            match event.id().as_ref() {
                "open" => show_window(app),
                "check" => {
                    tauri::async_runtime::spawn(async move { hub.refresh(true).await });
                }
                "quit" => app.exit(0),
                id => {
                    if let Some(app_id) = id.strip_prefix("launch:") {
                        let _ = hub.launch(app_id);
                    }
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Durum değişince: tepsi menüsü (kurulu uygulamalar ya da dil değiştiyse) ve görev çubuğu ilerlemesi.
pub fn state_changed(hub: &Hub) {
    let installed: Vec<String> = hub.installed_apps().into_iter().map(|(id, _)| id).collect();
    let signature = format!("{}|{}", hub.language(), installed.join(","));
    let mut last = TRAY_SIGNATURE.lock().unwrap_or_else(|p| p.into_inner());
    if *last != signature {
        if let Some(tray) = hub.app.tray_by_id(TRAY_ID) {
            if let Ok(menu) = build_tray_menu(&hub.app, hub) {
                let _ = tray.set_menu(Some(menu));
                *last = signature;
            }
        }
    }
    drop(last);
    update_taskbar(hub);
}

pub fn update_taskbar(hub: &Hub) {
    let Some(window) = hub.app.get_webview_window(WINDOW) else { return };
    let state = match hub.taskbar_progress() {
        Some(Some(percent)) => ProgressBarState { status: Some(ProgressBarStatus::Normal), progress: Some(percent) },
        Some(None) => ProgressBarState { status: Some(ProgressBarStatus::Indeterminate), progress: None },
        None => ProgressBarState { status: Some(ProgressBarStatus::None), progress: None },
    };
    let _ = window.set_progress_bar(state);
}

// ---- bildirim ve açılışta başlatma ----------------------------------------------------------------------------------

/// Yeni sürüm bildirimi. Aynı sürüm için bir kez; pencere öndeyken arayüz zaten gösterdiği için bildirim çıkmaz.
pub fn updates_found(hub: &Hub, apps: &[Value]) {
    if !hub.settings().notify_updates {
        return;
    }
    let fresh = hub.take_unnotified(apps);
    if fresh.is_empty() || window_focused(&hub.app) {
        return;
    }
    let lang = hub.language();
    let body = if let [one] = fresh.as_slice() {
        let (name, version) = (one["name"].as_str().unwrap_or_default(), one["version"].as_str().unwrap_or_default());
        if lang == "en" {
            format!("{name} {version} is out. Open Robz Hub to update.")
        } else {
            format!("{name} {version} yayınlandı. Güncellemek için Robz Hub'ı aç.")
        }
    } else if lang == "en" {
        format!("{} apps have new versions.", fresh.len())
    } else {
        format!("{} uygulama için yeni sürüm var.", fresh.len())
    };
    let _ = hub.app.notification().builder().title(strings(lang).update_title).body(body).show();
}

/// Yalnız kurulu sürümde: geliştirme kipinde geçici exe'yi Windows açılışına eklemeyelim.
pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    let autostart = app.autolaunch();
    let _ = if enabled { autostart.enable() } else { autostart.disable() };
}

/// Arka plan zamanlayıcıları: kurulu uygulamalar 5 sn'de bir (pencere kapalı ya da tepsideyken de), GitHub'daki
/// sürümler 6 saatte bir denetlenir. 5 sn'lik okuma ucuzdur: yalnız uygulamaların adıyla eşleşen süreçlere ve
/// birkaç kayıt defteri anahtarına bakılır.
pub fn start_timers(hub: Arc<Hub>) {
    let poll = hub.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            tick.tick().await;
            poll.poll();
        }
    });
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
        tick.tick().await; // ilk denetim açılışta ayrıca yapılır
        loop {
            tick.tick().await;
            if hub.settings().auto_check {
                hub.refresh(false).await;
            }
        }
    });
}
