//! Arayüze giden hata: kısa bir kod ("offline", "checksum-mismatch" …) ve isteğe bağlı ayrıntı.
//! Kodun kullanıcıya gösterilecek metni arayüzün dil dosyalarındadır (ui/js/locales).

use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HubError {
    pub code: String,
    pub detail: Option<String>,
}

impl HubError {
    pub fn new(code: &str) -> Self {
        Self { code: code.to_string(), detail: None }
    }

    pub fn with(code: &str, detail: impl fmt::Display) -> Self {
        Self { code: code.to_string(), detail: Some(detail.to_string()) }
    }

    pub fn is(&self, code: &str) -> bool {
        self.code == code
    }
}

impl fmt::Display for HubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "{}: {}", self.code, detail),
            None => write!(f, "{}", self.code),
        }
    }
}

impl std::error::Error for HubError {}

pub type HubResult<T> = Result<T, HubError>;
