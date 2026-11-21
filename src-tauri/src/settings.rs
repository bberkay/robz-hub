//! Ayarlar: veri klasöründeki settings.json. Bozuk dosyada ya da yanlış türde varsayılanlar kullanılır.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "auto" | "tr" | "en"
    pub language: String,
    /// Pencere kapatılınca tepside kal.
    pub close_to_tray: bool,
    /// Windows açılışında (tepside) başlat.
    pub launch_at_login: bool,
    /// Arka planda yeni sürüm denetle.
    pub auto_check: bool,
    /// Yeni sürüm bulununca Windows bildirimi göster.
    pub notify_updates: bool,
    /// İndirilen kurulum dosyalarını kurulumdan sonra silme.
    pub keep_installers: bool,
    /// Uygulama kimliği -> bildirimi gösterilmiş son sürüm (aynı sürüm iki kez bildirilmez).
    pub notified: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            close_to_tray: false,
            launch_at_login: false,
            auto_check: true,
            notify_updates: true,
            keep_installers: false,
            notified: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let mut settings: Settings =
            std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if !matches!(settings.language.as_str(), "auto" | "tr" | "en") {
            settings.language = "auto".into();
        }
        settings
    }

    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    /// Arayüzden gelen değişiklik. Bilinmeyen anahtar ya da yanlış tür: false.
    pub fn set(&mut self, key: &str, value: &Value) -> bool {
        match (key, value) {
            ("language", Value::String(s)) if matches!(s.as_str(), "auto" | "tr" | "en") => self.language = s.clone(),
            ("closeToTray", Value::Bool(b)) => self.close_to_tray = *b,
            ("launchAtLogin", Value::Bool(b)) => self.launch_at_login = *b,
            ("autoCheck", Value::Bool(b)) => self.auto_check = *b,
            ("notifyUpdates", Value::Bool(b)) => self.notify_updates = *b,
            ("keepInstallers", Value::Bool(b)) => self.keep_installers = *b,
            _ => return false,
        }
        true
    }

    /// Arayüze gönderilen kopya (iç kayıtlar hariç).
    pub fn public(&self) -> Value {
        json!({
            "language": self.language,
            "closeToTray": self.close_to_tray,
            "launchAtLogin": self.launch_at_login,
            "autoCheck": self.auto_check,
            "notifyUpdates": self.notify_updates,
            "keepInstallers": self.keep_installers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_changes() {
        let mut s = Settings::default();
        assert!(s.set("language", &json!("en")));
        assert!(!s.set("language", &json!("de")));
        assert!(!s.set("closeToTray", &json!("yes")));
        assert!(!s.set("notified", &json!({})));
        assert!(s.set("closeToTray", &json!(true)));
        assert_eq!(s.language, "en");
        assert!(s.close_to_tray);
        assert!(s.public().get("notified").is_none());
    }

    #[test]
    fn survives_broken_files() {
        let dir = std::env::temp_dir().join(format!("robzhub-settings-{}", std::process::id()));
        let file = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(Settings::load(&file), Settings::default());
        std::fs::write(&file, r#"{ "language": "de", "autoCheck": false }"#).unwrap();
        let s = Settings::load(&file);
        assert_eq!(s.language, "auto");
        assert!(!s.auto_check);
        s.save(&file);
        assert_eq!(Settings::load(&file), s);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
