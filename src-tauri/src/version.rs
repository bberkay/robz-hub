//! Sürüm etiketlerini ("v1.6.0", "0.4.0.0", "1.10.1-rc1") ayrıştırır ve karşılaştırır.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    nums: Vec<u64>,
    pre: String,
}

pub fn parse(value: &str) -> Option<Version> {
    let s = value.trim();
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let end = s.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(s.len());
    let core = s[..end].trim_end_matches('.');
    if !core.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut nums = Vec::new();
    for part in core.split('.') {
        nums.push(part.parse::<u64>().ok()?);
    }
    // Windows dosya sürümü dört parçalıdır (0.4.0.0); sondaki fazladan sıfırlar anlam taşımaz.
    while nums.len() > 3 && nums.last() == Some(&0) {
        nums.pop();
    }
    let pre = s[end..]
        .strip_prefix('-')
        .map(|p| p.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').collect())
        .unwrap_or_default();
    Some(Version { nums, pre })
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.nums.len().max(other.nums.len());
        for i in 0..len {
            let a = self.nums.get(i).copied().unwrap_or(0);
            let b = other.nums.get(i).copied().unwrap_or(0);
            if a != b {
                return a.cmp(&b);
            }
        }
        // Aynı sayılarda ön sürüm (1.2.0-rc1) asıl sürümden (1.2.0) küçüktür.
        match (self.pre.is_empty(), other.pre.is_empty()) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            _ => self.pre.cmp(&other.pre),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Ayrıştırılamayan sürüm her şeyden küçük sayılır.
pub fn compare(a: &str, b: &str) -> Ordering {
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

/// Gösterim için: "v1.6.0" -> "1.6.0", "0.4.0.0" -> "0.4.0".
pub fn clean(value: &str) -> Option<String> {
    let v = parse(value)?;
    let nums: Vec<String> = v.nums.iter().map(u64::to_string).collect();
    let mut out = nums.join(".");
    if !v.pre.is_empty() {
        out.push('-');
        out.push_str(&v.pre);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tags_and_file_versions() {
        assert_eq!(clean("v1.6.0").as_deref(), Some("1.6.0"));
        assert_eq!(clean("0.4.0.0").as_deref(), Some("0.4.0"));
        assert_eq!(clean("1.2.3.4").as_deref(), Some("1.2.3.4"));
        assert_eq!(clean("v1.10.1-rc1").as_deref(), Some("1.10.1-rc1"));
        assert_eq!(clean("Robz"), None);
        assert_eq!(clean(""), None);
    }

    #[test]
    fn compares_numerically() {
        assert_eq!(compare("1.10.0", "1.9.2"), Ordering::Greater);
        assert_eq!(compare("v1.3.2", "1.6.0"), Ordering::Less);
        assert_eq!(compare("0.4.0.0", "v0.4.0"), Ordering::Equal);
        assert_eq!(compare("1.2.0-rc1", "1.2.0"), Ordering::Less);
        assert_eq!(compare("1.2.0", "garbage"), Ordering::Greater);
    }
}
